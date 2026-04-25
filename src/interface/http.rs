//! HTTP adapter: thin axum handlers translating between DTOs and the
//! application services. No business logic lives here.

use std::sync::{Arc, Mutex};

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, patch, post, put},
};
use tower_http::cors::{Any, CorsLayer};

use crate::domain::actor::Actor;
use crate::domain::crew_lead::CrewLeadId;
use crate::domain::errors::DomainError;
use crate::domain::passenger::PassengerId;
use crate::domain::resource::ResourceId;
use crate::domain::tier::Tier;
use crate::interface::composition_root::{World, build_demo_world};
use crate::interface::dto::{
    AccessibleQuery, ActorOnlyReq, AddCrewLeadReq, AdminEventDto, ChangeTierReq,
    CreatePassengerReq, CreateResourceReq, CrewLeadDto, ErrorBody, PassengerDto, RemoveCrewLeadReq,
    ReplaceCrewLeadReq, ResourceDto, TierCountsDto, TierDto, TopNQuery, TopResourceDto,
    UsageEventDto, UseResourceReq,
};

/// Shared state held by every handler.
pub type AppState = Arc<Mutex<World>>;

/// Build the axum router with CORS and the full PRMS endpoint surface.
pub fn router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/health", get(health))
        // crew leads
        .route("/crew-leads", get(list_crew_leads).post(add_crew_lead))
        .route(
            "/crew-leads/:old_id",
            put(replace_crew_lead).delete(remove_crew_lead),
        )
        // passengers
        .route("/passengers", get(list_passengers).post(create_passenger))
        .route(
            "/passengers/:id",
            get(get_passenger).delete(soft_delete_passenger),
        )
        .route("/passengers/:id/tier", patch(change_passenger_tier))
        // resources
        .route("/resources", get(list_resources).post(create_resource))
        .route("/resources/accessible", get(list_accessible_resources))
        .route(
            "/resources/:id",
            get(get_resource).delete(soft_delete_resource),
        )
        .route("/resources/:id/min-tier", patch(change_resource_min_tier))
        // access
        .route("/access", post(use_resource))
        // audit + usage
        .route("/audit", get(list_admin_events))
        .route("/usage", get(list_usage_events))
        // reports
        .route("/reports/by-tier", get(report_by_tier))
        .route("/reports/top-resources", get(report_top_resources))
        .route(
            "/reports/history/:passenger_id",
            get(report_personal_history),
        )
        // admin
        .route("/reset", post(reset_world))
        .with_state(state)
        .layer(cors)
}

// ---------- error mapping ----------------------------------------------

/// `DomainError` → HTTP. Validation failures at the boundary use 400;
/// authorisation 403; not-found 404; conflicts 409. New variants get a
/// default 500 to surface unhandled cases.
fn map_err(e: &DomainError) -> (StatusCode, &'static str) {
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
    let msg = e.to_string();
    let (status, code) = map_err(e);
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
    // SAFETY (re: AGENTS.md §8): a poisoned lock means a previous
    // handler panicked mid-mutation, leaving the World in an
    // unknown state. Continuing would corrupt the audit trail, so
    // we deliberately propagate the panic to crash the worker.
    state.lock().expect("world mutex poisoned")
}

// ---------- handlers ---------------------------------------------------

async fn health() -> &'static str {
    "ok"
}

async fn list_crew_leads(State(state): State<AppState>) -> Json<Vec<CrewLeadDto>> {
    let w = lock_world(&state);
    Json(w.crew_leads.list().iter().map(CrewLeadDto::from).collect())
}

async fn replace_crew_lead(
    State(state): State<AppState>,
    Path(old_id): Path<String>,
    Json(req): Json<ReplaceCrewLeadReq>,
) -> Response {
    let mut w = lock_world(&state);
    match w.crew_leads.replace_audited(
        &CrewLeadId(req.actor_id),
        &CrewLeadId(old_id),
        req.new_lead.into(),
    ) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => err_response_owned(&e),
    }
}

async fn list_passengers(State(state): State<AppState>) -> Json<Vec<PassengerDto>> {
    let w = lock_world(&state);
    Json(w.passengers.list().iter().map(PassengerDto::from).collect())
}

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

async fn list_resources(State(state): State<AppState>) -> Json<Vec<ResourceDto>> {
    let w = lock_world(&state);
    Json(w.resources.list().iter().map(ResourceDto::from).collect())
}

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

async fn use_resource(State(state): State<AppState>, Json(req): Json<UseResourceReq>) -> Response {
    let mut w = lock_world(&state);
    let actor = Actor::Passenger(PassengerId(req.passenger_id));
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
    // Stable ordering: Silver, Gold, Platinum
    rows.sort_by_key(|r| match r.tier {
        TierDto::Silver => 0,
        TierDto::Gold => 1,
        TierDto::Platinum => 2,
    });
    Json(rows)
}

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

async fn add_crew_lead(State(state): State<AppState>, Json(req): Json<AddCrewLeadReq>) -> Response {
    let mut w = lock_world(&state);
    // CL-R2 — always 409 (`CrewLeadLimitReached`) by design.
    match w.crew_leads.add(req.lead.into()) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => err_response_owned(&e),
    }
}

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

async fn get_passenger(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    let w = lock_world(&state);
    match w.passengers.get(&PassengerId(id)) {
        Ok(p) => (StatusCode::OK, Json(PassengerDto::from(p))).into_response(),
        Err(e) => err_response_owned(&e),
    }
}

async fn get_resource(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    let w = lock_world(&state);
    match w.resources.get(&ResourceId(id)) {
        Ok(r) => (StatusCode::OK, Json(ResourceDto::from(r))).into_response(),
        Err(e) => err_response_owned(&e),
    }
}

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

async fn reset_world(State(state): State<AppState>, Json(req): Json<ActorOnlyReq>) -> Response {
    // Demo-only affordance, but still gated: caller must identify as
    // an existing crew lead so an anonymous client can't wipe state.
    {
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
    *lock_world(&state) = fresh;
    StatusCode::NO_CONTENT.into_response()
}
