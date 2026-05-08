//! HTTP adapter: thin axum handlers translating between DTOs and the
//! application services. No business logic lives here.

// `utoipa`'s `#[derive(OpenApi)]` expansion uses `for_each` on a slice
// iterator, which clippy::pedantic flags as `needless_for_each`. The
// expansion is out of our control, so we silence the lint at module
// scope rather than per-item (the attribute does not propagate into
// derive-generated tokens).
// `#![allow(...)]` (with the `!`) is INNER attribute — applies to the
// whole module/file. `#[allow(...)]` (no `!`) applies to one item.
#![allow(clippy::needless_for_each)]

// Standard-library threading primitives. See in_memory_admin_event_sink
// for the Arc<Mutex<...>> pattern explanation.
use std::sync::{Arc, Mutex};

// `axum` is the HTTP framework. The `use { a, b, c }` syntax imports
// multiple items from one path in a single statement.
use axum::{
    Json, Router,
    // Axum *extractors* — types that pull data out of a request:
    //   Path<T>   -> URL path parameters
    //   Query<T>  -> query-string parameters (?foo=bar)
    //   State<T>  -> shared application state (AppState here)
    //   Json<T>   -> deserialised JSON body (also used for responses)
    extract::{DefaultBodyLimit, Path, Query, State},
    http::{HeaderName, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    // HTTP method routers — `get(handler)` registers a GET handler.
    routing::{get, patch, post, put},
};
// `tower-http` is a collection of reusable HTTP middlewares.
use tower_http::cors::{Any, CorsLayer};
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use utoipa::OpenApi;

use crate::domain::actor::Actor;
use crate::domain::crew_lead::CrewLeadId;
use crate::domain::errors::DomainError;
use crate::domain::passenger::PassengerId;
use crate::domain::resource::ResourceId;
use crate::domain::tier::Tier;
use crate::interface::composition_root::{World, build_demo_world};
use crate::interface::dto::{
    AccessibleQuery, ActorOnlyReq, AddCrewLeadReq, AdminEventDto, ChangeTierReq,
    CreatePassengerReq, CreateResourceReq, CrewLeadDto, ErrorBody, OutcomeDto, PassengerDto,
    RemoveCrewLeadReq, ReplaceCrewLeadReq, ResourceDto, TierCountsDto, TierDto, TopNQuery,
    TopResourceDto, UsageEventDto, UseResourceReq,
};

/// Shared state held by every handler.
// `type` declares an alias — `AppState` is just a shorter name for the
// `Arc<Mutex<World>>` triple. Doesn't introduce a new type.
pub type AppState = Arc<Mutex<World>>;

/// CORS origin policy. `Any` accepts any origin (dev/demo default);
/// `List` accepts only the listed origins (production-style).
#[derive(Debug, Clone, Default)]
pub enum CorsOrigins {
    // `#[default]` marks which variant `Default::default()` returns.
    // Required when deriving `Default` on an enum.
    #[default]
    Any,
    List(Vec<HeaderValue>),
}

/// Build the axum router with CORS and the full PRMS endpoint surface.
///
/// Equivalent to [`router_with`] using `CorsOrigins::Any` and reset disabled.
pub fn router(state: AppState) -> Router {
    router_with(state, CorsOrigins::Any, false)
}

/// Build the axum router with explicit CORS configuration.
///
/// `enable_reset` — when `false` the `/reset` route is not registered,
/// making it impossible to wipe state via the HTTP API. Set to `true`
/// only for local dev / integration tests.
pub fn router_with(state: AppState, cors_origins: CorsOrigins, enable_reset: bool) -> Router {
    let cors = match cors_origins {
        CorsOrigins::Any => CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any),
        CorsOrigins::List(origins) => CorsLayer::new()
            .allow_origin(origins)
            .allow_methods(Any)
            .allow_headers(Any),
    };

    let x_request_id = HeaderName::from_static("x-request-id");

    // Builder-style chain. Each `.route(...)` returns a new Router with
    // one more endpoint registered. `Router::new()` starts empty.
    Router::new()
        .route("/health", get(health))
        .route("/openapi.json", get(openapi_json))
        // crew leads
        // `.post(handler)` chained after `.get(...)` registers a second
        // method on the same path. `{old_id}` is a path parameter
        // captured by `Path<String>` extractors in the handler.
        .route("/crew-leads", get(list_crew_leads).post(add_crew_lead))
        .route(
            "/crew-leads/{old_id}",
            put(replace_crew_lead).delete(remove_crew_lead),
        )
        // passengers
        .route("/passengers", get(list_passengers).post(create_passenger))
        .route(
            "/passengers/{id}",
            get(get_passenger).delete(soft_delete_passenger),
        )
        .route("/passengers/{id}/tier", patch(change_passenger_tier))
        // resources
        .route("/resources", get(list_resources).post(create_resource))
        .route("/resources/accessible", get(list_accessible_resources))
        .route(
            "/resources/{id}",
            get(get_resource).delete(soft_delete_resource),
        )
        .route("/resources/{id}/min-tier", patch(change_resource_min_tier))
        // access
        .route("/access", post(use_resource))
        // audit + usage
        .route("/audit", get(list_admin_events))
        .route("/usage", get(list_usage_events))
        // reports
        .route("/reports/by-tier", get(report_by_tier))
        .route("/reports/top-resources", get(report_top_resources))
        .route(
            "/reports/history/{passenger_id}",
            get(report_personal_history),
        )
        // admin — only registered when explicitly enabled (not in production)
        .merge(if enable_reset {
            Router::new().route("/reset", post(reset_world))
        } else {
            Router::new()
        })
        // Inject shared state into every handler that uses `State<AppState>`.
        .with_state(state)
        // 64 KiB body cap — every request DTO in this app is tiny.
        // Defends against accidental/malicious oversized payloads.
        .layer(DefaultBodyLimit::max(64 * 1024))
        // `.layer(...)` wraps the entire router in a middleware. Layers
        // run in REVERSE registration order on the request side and in
        // declaration order on the response side (tower convention).
        .layer(cors)
        // Request-id: assign a UUID if the client did not send one,
        // then propagate it back on the response so logs can correlate.
        .layer(PropagateRequestIdLayer::new(x_request_id.clone()))
        .layer(SetRequestIdLayer::new(x_request_id, MakeRequestUuid))
}

// ---------- error mapping ----------------------------------------------

/// `DomainError` → HTTP. Validation failures at the boundary use 400;
/// authorisation 403; not-found 404; conflicts 409. New variants get a
/// default 500 to surface unhandled cases.
fn map_err(e: &DomainError) -> (StatusCode, &'static str) {
    // `use DomainError as D;` is a local alias for brevity in the match.
    use DomainError as D;
    match e {
        D::UnauthorizedActor => (StatusCode::FORBIDDEN, "UnauthorizedActor"),
        D::AccessDenied => (StatusCode::FORBIDDEN, "AccessDenied"),
        D::CrewLeadNotFound => (StatusCode::NOT_FOUND, "CrewLeadNotFound"),
        D::PassengerNotFound => (StatusCode::NOT_FOUND, "PassengerNotFound"),
        D::ResourceNotFound => (StatusCode::NOT_FOUND, "ResourceNotFound"),
        D::CrewLeadAlreadyExists => (StatusCode::CONFLICT, "CrewLeadAlreadyExists"),
        D::PassengerAlreadyExists => (StatusCode::CONFLICT, "PassengerAlreadyExists"),
        D::ResourceAlreadyExists => (StatusCode::CONFLICT, "ResourceAlreadyExists"),
        D::CrewLeadLimitReached => (StatusCode::CONFLICT, "CrewLeadLimitReached"),
        D::CrewLeadMinimumBreached => (StatusCode::CONFLICT, "CrewLeadMinimumBreached"),
        D::CrewLeadBootstrapInvalid => (StatusCode::BAD_REQUEST, "CrewLeadBootstrapInvalid"),
    }
}

fn err_response_owned(e: &DomainError) -> Response {
    // `e.to_string()` calls `Display` (provided by `thiserror::Error`).
    let msg = e.to_string();
    let (status, code) = map_err(e);
    // A tuple `(StatusCode, Json<...>)` implements `IntoResponse`, so
    // chaining `.into_response()` produces a uniform `Response` type.
    (
        status,
        Json(ErrorBody {
            error: msg,
            code: code.to_owned(),
        }),
    )
        .into_response()
}

fn lock_world(state: &AppState) -> std::sync::MutexGuard<'_, World> {
    // `MutexGuard<'_, World>` is the RAII lock guard: dereferences to
    // `World`, releases the lock automatically when dropped.
    // The `'_` is an anonymous lifetime tied to `state`.
    //
    // SAFETY (re: AGENTS.md §8): a poisoned lock means a previous
    // handler panicked mid-mutation, leaving the World in an
    // unknown state. Continuing would corrupt the audit trail, so
    // we deliberately propagate the panic to crash the worker.
    state.lock().expect("world mutex poisoned")
}

// ---------- handlers ---------------------------------------------------

// `#[utoipa::path(...)]` annotates the handler so utoipa can include
// it in the auto-generated OpenAPI schema served at /openapi.json.

#[utoipa::path(get, path = "/health", tag = "system",
    responses((status = 200, description = "Server is up", body = String)))]
// `async fn` declares a function returning a `Future`. Axum handlers
// MUST be async — runtime (tokio) drives them. `&'static str` is fine
// to return: axum's `IntoResponse` for it sends the bytes verbatim
// with `text/plain` and 200 OK.
async fn health() -> &'static str {
    "ok"
}

#[utoipa::path(get, path = "/crew-leads", tag = "crew-leads",
    responses((status = 200, description = "All crew leads", body = Vec<CrewLeadDto>)))]
async fn list_crew_leads(State(state): State<AppState>) -> Json<Vec<CrewLeadDto>> {
    // `State(state): State<AppState>` destructures the extractor in
    // the parameter pattern: `state` is now `AppState` (= Arc<Mutex<World>>).
    let w = lock_world(&state);
    // `Json(value)` wraps any `Serialize` value as a JSON response.
    // `iter().map(...).collect()` is the standard Rust pipeline for
    // transforming a slice into a Vec of converted items.
    Json(w.crew_leads.list().iter().map(CrewLeadDto::from).collect())
}

#[utoipa::path(put, path = "/crew-leads/{old_id}", tag = "crew-leads",
    params(("old_id" = String, Path, description = "Crew lead ID being replaced")),
    request_body = ReplaceCrewLeadReq,
    responses(
        (status = 204, description = "Replaced"),
        (status = 404, body = ErrorBody),
        (status = 403, body = ErrorBody)))]
async fn replace_crew_lead(
    State(state): State<AppState>,
    // Each function argument is a separate axum extractor. Order DOES
    // matter: the body extractor (`Json<...>`) MUST come last because
    // it consumes the request body. Using two body extractors is a
    // compile error caught by axum's trait bounds.
    Path(old_id): Path<String>,
    Json(req): Json<ReplaceCrewLeadReq>,
) -> Response {
    // `&mut w` because `replace_audited` mutates the service state.
    let mut w = lock_world(&state);
    match w.crew_leads.replace_audited(
        &CrewLeadId(req.actor_id),
        &CrewLeadId(old_id),
        // `req.new_lead.into()` calls `From<CrewLeadDto> for CrewLead`.
        req.new_lead.into(),
    ) {
        // `Ok(())` matches the unit-Ok variant. `()` is the empty tuple
        // (Rust's "void"). NO_CONTENT (204) is the conventional response
        // for successful mutations with no body to return.
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => err_response_owned(&e),
    }
}

#[utoipa::path(get, path = "/passengers", tag = "passengers",
    responses((status = 200, body = Vec<PassengerDto>)))]
async fn list_passengers(State(state): State<AppState>) -> Json<Vec<PassengerDto>> {
    let w = lock_world(&state);
    Json(w.passengers.list().iter().map(PassengerDto::from).collect())
}

#[utoipa::path(post, path = "/passengers", tag = "passengers",
    request_body = CreatePassengerReq,
    responses(
        (status = 201, body = PassengerDto),
        (status = 409, body = ErrorBody)))]
async fn create_passenger(
    State(state): State<AppState>,
    Json(req): Json<CreatePassengerReq>,
) -> Response {
    let mut w = lock_world(&state);
    let actor = Actor::CrewLead(CrewLeadId(req.actor_id));
    match w
        .passengers
        .create(&actor, PassengerId(req.id), req.name, Tier::from(req.tier))
    {
        Ok(p) => (StatusCode::CREATED, Json(PassengerDto::from(&p))).into_response(),
        Err(e) => err_response_owned(&e),
    }
}

#[utoipa::path(patch, path = "/passengers/{id}/tier", tag = "passengers",
    params(("id" = String, Path)),
    request_body = ChangeTierReq,
    responses((status = 204), (status = 404, body = ErrorBody)))]
async fn change_passenger_tier(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<ChangeTierReq>,
) -> Response {
    let mut w = lock_world(&state);
    let actor = Actor::CrewLead(CrewLeadId(req.actor_id));
    match w
        .passengers
        .change_tier(&actor, &PassengerId(id), Tier::from(req.tier))
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => err_response_owned(&e),
    }
}

#[utoipa::path(delete, path = "/passengers/{id}", tag = "passengers",
    params(("id" = String, Path)),
    request_body = ActorOnlyReq,
    responses((status = 204), (status = 404, body = ErrorBody)))]
async fn soft_delete_passenger(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<ActorOnlyReq>,
) -> Response {
    let mut w = lock_world(&state);
    let actor = Actor::CrewLead(CrewLeadId(req.actor_id));
    match w.passengers.soft_delete(&actor, &PassengerId(id)) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => err_response_owned(&e),
    }
}

#[utoipa::path(get, path = "/resources", tag = "resources",
    responses((status = 200, body = Vec<ResourceDto>)))]
async fn list_resources(State(state): State<AppState>) -> Json<Vec<ResourceDto>> {
    let w = lock_world(&state);
    Json(w.resources.list().iter().map(ResourceDto::from).collect())
}

#[utoipa::path(post, path = "/resources", tag = "resources",
    request_body = CreateResourceReq,
    responses(
        (status = 201, body = ResourceDto),
        (status = 409, body = ErrorBody)))]
async fn create_resource(
    State(state): State<AppState>,
    Json(req): Json<CreateResourceReq>,
) -> Response {
    let mut w = lock_world(&state);
    let actor = Actor::CrewLead(CrewLeadId(req.actor_id));
    match w.resources.create(
        &actor,
        ResourceId(req.id),
        req.name,
        req.category,
        Tier::from(req.min_tier),
    ) {
        Ok(r) => (StatusCode::CREATED, Json(ResourceDto::from(&r))).into_response(),
        Err(e) => err_response_owned(&e),
    }
}

#[utoipa::path(patch, path = "/resources/{id}/min-tier", tag = "resources",
    params(("id" = String, Path)),
    request_body = ChangeTierReq,
    responses((status = 204), (status = 404, body = ErrorBody)))]
async fn change_resource_min_tier(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<ChangeTierReq>,
) -> Response {
    let mut w = lock_world(&state);
    let actor = Actor::CrewLead(CrewLeadId(req.actor_id));
    match w
        .resources
        .change_min_tier(&actor, &ResourceId(id), Tier::from(req.tier))
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => err_response_owned(&e),
    }
}

#[utoipa::path(delete, path = "/resources/{id}", tag = "resources",
    params(("id" = String, Path)),
    request_body = ActorOnlyReq,
    responses((status = 204), (status = 404, body = ErrorBody)))]
async fn soft_delete_resource(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<ActorOnlyReq>,
) -> Response {
    let mut w = lock_world(&state);
    let actor = Actor::CrewLead(CrewLeadId(req.actor_id));
    match w.resources.soft_delete(&actor, &ResourceId(id)) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => err_response_owned(&e),
    }
}

#[utoipa::path(post, path = "/access", tag = "access",
    request_body = UseResourceReq,
    responses(
        (status = 200, body = UsageEventDto),
        (status = 403, body = ErrorBody)))]
async fn use_resource(State(state): State<AppState>, Json(req): Json<UseResourceReq>) -> Response {
    let mut w = lock_world(&state);
    let actor = Actor::Passenger(PassengerId(req.passenger_id));
    // BORROW SPLITTING: `access.use_resource` needs `&mut self` on
    // `access` AND immutable borrows on `passengers` + `resources`,
    // all from the same `World`. The borrow checker won't let us call
    // `w.passengers.list()` while we hold `&mut w.access`, so we
    // destructure `*w` once into separate field bindings — the
    // compiler now sees three INDEPENDENT borrows it can track.
    // `..` ignores the remaining fields we don't need (`audit_sink`,
    // `crew_leads`).
    let World {
        passengers,
        resources,
        access,
        ..
    } = &mut *w;
    match access.use_resource(&actor, passengers, resources, &ResourceId(req.resource_id)) {
        Ok(ev) => (StatusCode::OK, Json(UsageEventDto::from(&ev))).into_response(),
        Err(e) => err_response_owned(&e),
    }
}

#[utoipa::path(get, path = "/audit", tag = "audit",
    responses((status = 200, body = Vec<AdminEventDto>)))]
async fn list_admin_events(State(state): State<AppState>) -> Json<Vec<AdminEventDto>> {
    let w = lock_world(&state);
    Json(
        w.audit_sink
            .snapshot()
            .iter()
            .map(AdminEventDto::from)
            .collect(),
    )
}

#[utoipa::path(get, path = "/usage", tag = "audit",
    responses((status = 200, body = Vec<UsageEventDto>)))]
async fn list_usage_events(State(state): State<AppState>) -> Json<Vec<UsageEventDto>> {
    use crate::application::ports::UsageEventSource;
    let w = lock_world(&state);
    Json(
        w.access
            .sink()
            .list()
            .iter()
            .map(UsageEventDto::from)
            .collect(),
    )
}

#[utoipa::path(get, path = "/reports/by-tier", tag = "reports",
    responses((status = 200, body = Vec<TierCountsDto>)))]
async fn report_by_tier(State(state): State<AppState>) -> Json<Vec<TierCountsDto>> {
    use crate::application::reporting_service::ReportingService;
    let w = lock_world(&state);
    let report = ReportingService::new(w.access.sink()).aggregate_by_tier();
    let mut rows: Vec<TierCountsDto> = report
        .into_iter()
        .map(|(tier, c)| TierCountsDto {
            tier: tier.into(),
            allowed: c.allowed,
            denied: c.denied,
        })
        .collect();
    // Stable ordering: Silver, Gold, Diamond, Platinum
    rows.sort_by_key(|r| match r.tier {
        TierDto::Silver   => 0,
        TierDto::Gold     => 1,
        TierDto::Diamond  => 2,
        TierDto::Platinum => 3,
    });
    Json(rows)
}

#[utoipa::path(get, path = "/reports/top-resources", tag = "reports",
    params(TopNQuery),
    responses((status = 200, body = Vec<TopResourceDto>)))]
async fn report_top_resources(
    State(state): State<AppState>,
    Query(q): Query<TopNQuery>,
) -> Json<Vec<TopResourceDto>> {
    use crate::application::reporting_service::ReportingService;
    let w = lock_world(&state);
    let n = q.n.unwrap_or(5);
    Json(
        ReportingService::new(w.access.sink())
            .top_resources(n)
            .into_iter()
            .map(|(rid, count)| TopResourceDto {
                resource_id: rid.0,
                allowed_count: count,
            })
            .collect(),
    )
}

#[utoipa::path(get, path = "/reports/history/{passenger_id}", tag = "reports",
    params(("passenger_id" = String, Path)),
    responses((status = 200, body = Vec<UsageEventDto>)))]
async fn report_personal_history(
    State(state): State<AppState>,
    Path(passenger_id): Path<String>,
) -> Json<Vec<UsageEventDto>> {
    use crate::application::reporting_service::ReportingService;
    let w = lock_world(&state);
    Json(
        ReportingService::new(w.access.sink())
            .personal_history(&PassengerId(passenger_id))
            .iter()
            .map(UsageEventDto::from)
            .collect(),
    )
}

// ---------- new endpoints ---------------------------------------------

#[utoipa::path(post, path = "/crew-leads", tag = "crew-leads",
    request_body = AddCrewLeadReq,
    responses(
        (status = 204),
        (status = 409, body = ErrorBody)))]
async fn add_crew_lead(State(state): State<AppState>, Json(req): Json<AddCrewLeadReq>) -> Response {
    let mut w = lock_world(&state);
    // CL-R2 — always 409 (`CrewLeadLimitReached`) by design.
    match w.crew_leads.add(req.lead.into()) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => err_response_owned(&e),
    }
}

#[utoipa::path(delete, path = "/crew-leads/{id}", tag = "crew-leads",
    params(("id" = String, Path)),
    request_body = RemoveCrewLeadReq,
    responses(
        (status = 204),
        (status = 409, body = ErrorBody)))]
async fn remove_crew_lead(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(_req): Json<RemoveCrewLeadReq>,
) -> Response {
    let mut w = lock_world(&state);
    // CL-R3 — always 409 (`CrewLeadMinimumBreached`) by design.
    match w.crew_leads.remove(&CrewLeadId(id)) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => err_response_owned(&e),
    }
}

#[utoipa::path(get, path = "/passengers/{id}", tag = "passengers",
    params(("id" = String, Path)),
    responses((status = 200, body = PassengerDto), (status = 404, body = ErrorBody)))]
async fn get_passenger(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    let w = lock_world(&state);
    match w.passengers.get(&PassengerId(id)) {
        Ok(p) => (StatusCode::OK, Json(PassengerDto::from(p))).into_response(),
        Err(e) => err_response_owned(&e),
    }
}

#[utoipa::path(get, path = "/resources/{id}", tag = "resources",
    params(("id" = String, Path)),
    responses((status = 200, body = ResourceDto), (status = 404, body = ErrorBody)))]
async fn get_resource(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    let w = lock_world(&state);
    match w.resources.get(&ResourceId(id)) {
        Ok(r) => (StatusCode::OK, Json(ResourceDto::from(r))).into_response(),
        Err(e) => err_response_owned(&e),
    }
}

#[utoipa::path(get, path = "/resources/accessible", tag = "resources",
    params(AccessibleQuery),
    responses((status = 200, body = Vec<ResourceDto>)))]
async fn list_accessible_resources(
    State(state): State<AppState>,
    Query(q): Query<AccessibleQuery>,
) -> Json<Vec<ResourceDto>> {
    let w = lock_world(&state);
    Json(
        w.resources
            .list_accessible_for(Tier::from(q.tier))
            .iter()
            .map(ResourceDto::from)
            .collect(),
    )
}

#[utoipa::path(post, path = "/reset", tag = "system",
    request_body = ActorOnlyReq,
    responses((status = 204), (status = 403, body = ErrorBody)))]
async fn reset_world(State(state): State<AppState>, Json(req): Json<ActorOnlyReq>) -> Response {
    // Demo-only affordance, but still gated: caller must identify as
    // an existing crew lead so an anonymous client can't wipe state.
    {
        // Inner block scopes the lock guard so it's RELEASED before
        // the `*lock_world(...) = fresh` reassignment below — otherwise
        // we'd hold two guards at once and deadlock.
        let w = lock_world(&state);
        let actor_id = CrewLeadId(req.actor_id.clone());
        if !w.crew_leads.list().iter().any(|c| c.id == actor_id) {
            return err_response_owned(&DomainError::UnauthorizedActor);
        }
    }
    let fresh = match build_demo_world() {
        Ok(w) => w,
        Err(e) => return err_response_owned(&e),
    };
    // `*guard = value` writes through the deref to replace the World.
    // The guard is dropped at the end of this expression, releasing
    // the lock before we return.
    *lock_world(&state) = fresh;
    StatusCode::NO_CONTENT.into_response()
}

// ---------- OpenAPI ----------------------------------------------------

// `utoipa::OpenApi` is a derive macro that builds a static OpenAPI spec
// at compile time from the `#[utoipa::path(...)]` annotations on each
// handler. The empty struct `ApiDoc` is just a carrier for the impl.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "PRMS HTTP API",
        // `env!("CARGO_PKG_VERSION")` is a compile-time macro that
        // expands to the value of the env var (set by Cargo to the
        // package version from Cargo.toml). Errors out at compile time
        // if the var is missing.
        version = env!("CARGO_PKG_VERSION"),
        description = "Spaceship X26 Passenger Resource Management System."
    ),
    paths(
        health,
        list_crew_leads, add_crew_lead, replace_crew_lead, remove_crew_lead,
        list_passengers, create_passenger, get_passenger,
        change_passenger_tier, soft_delete_passenger,
        list_resources, create_resource, get_resource,
        list_accessible_resources, change_resource_min_tier, soft_delete_resource,
        use_resource,
        list_admin_events, list_usage_events,
        report_by_tier, report_top_resources, report_personal_history,
        reset_world,
    ),
    components(schemas(
        TierDto, OutcomeDto,
        CrewLeadDto, AddCrewLeadReq, ReplaceCrewLeadReq, RemoveCrewLeadReq,
        PassengerDto, CreatePassengerReq, ChangeTierReq,
        ResourceDto, CreateResourceReq,
        ActorOnlyReq, UseResourceReq,
        UsageEventDto, AdminEventDto,
        TierCountsDto, TopResourceDto,
        ErrorBody,
    )),
    tags(
        (name = "system", description = "Health & admin"),
        (name = "crew-leads", description = "Crew lead management"),
        (name = "passengers", description = "Passenger lifecycle"),
        (name = "resources", description = "Resource lifecycle"),
        (name = "access", description = "Access checks"),
        (name = "audit", description = "Admin / usage audit logs"),
        (name = "reports", description = "Aggregated reports"),
    )
)]
struct ApiDoc;

async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    // `ApiDoc::openapi()` is generated by the derive macro above.
    Json(ApiDoc::openapi())
}
