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
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

// `subtle::ConstantTimeEq` provides timing-safe byte comparison so an
// attacker cannot infer token prefixes from response latency (OWASP A07).
use subtle::ConstantTimeEq;

// `axum` is the HTTP framework. The `use { a, b, c }` syntax imports
// multiple items from one path in a single statement.
use axum::{
    Json,
    Router,
    body::Bytes,
    // Axum *extractors* — types that pull data out of a request:
    //   Path<T>   -> URL path parameters
    //   Query<T>  -> query-string parameters (?foo=bar)
    //   State<T>  -> shared application state (AppState here)
    //   Json<T>   -> deserialised JSON body (also used for responses)
    extract::{DefaultBodyLimit, FromRequestParts, Path, Query, State},
    http::{HeaderName, HeaderValue, StatusCode, request::Parts},
    response::{IntoResponse, Response},
    // HTTP method routers — `get(handler)` registers a GET handler.
    routing::{get, patch, post, put},
};
// `tower-http` is a collection of reusable HTTP middlewares.
use tower_governor::GovernorLayer;
use tower_governor::governor::GovernorConfigBuilder;
use tower_http::cors::{Any, CorsLayer};
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;

use crate::application::access_service::AccessService;
use crate::application::crew_lead_service::CrewLeadService;
use crate::application::passenger_service::PassengerService;
use crate::application::resource_service::ResourceService;
use crate::domain::actor::Actor;
use crate::domain::crew_lead::CrewLeadId;
use crate::domain::errors::DomainError;
use crate::domain::passenger::PassengerId;
use crate::domain::resource::ResourceId;
use crate::domain::tier::Tier;
use crate::infrastructure::SqliteEntityStore;
use crate::infrastructure::fake_clock::FakeClock;
use crate::interface::composition_root::{AuditSink, UsageSink, World, build_demo_world};
use crate::interface::dto::{
    AccessibleQuery, AddCrewLeadReq, AdminEventDto, ChangeTierReq, CreatePassengerReq,
    CreateResourceReq, CrewLeadDto, ErrorBody, HealthReadyDto, OutcomeDto, PaginationQuery,
    PassengerDto, ReplaceCrewLeadReq, ResourceDto, TierCountsDto, TierDto, TopNQuery,
    TopResourceDto, UsageEventDto, UseResourceReq,
};

/// Cached idempotency response: raw body bytes, status code, and Unix-second expiry.
struct IdempotencyEntry {
    status: StatusCode,
    body: Bytes,
    /// Unix timestamp (seconds) after which this entry may be evicted.
    expires_at: u64,
}

/// In-memory idempotency cache.  Keys are the client-supplied `Idempotency-Key` values.
/// Entries expire after `IDEMPOTENCY_TTL_SECS` seconds.
const IDEMPOTENCY_TTL_SECS: u64 = 600; // 10 minutes

/// Per-aggregate lock shards.
///
/// Each aggregate gets its own `RwLock` so concurrent writes to DIFFERENT
/// aggregates proceed without serialization. Only writes to the SAME aggregate
/// type contend with each other.
///
/// **Lock acquisition order** (prevents deadlocks when multiple locks are needed
/// simultaneously, e.g. `use_resource` and `reset_world`):
///
///   `crew_leads` → `passengers` → `resources` → `access` → `audit_sink`
///
/// Never acquire a lock out of this order.
struct WorldShards {
    crew_leads: RwLock<CrewLeadService>,
    passengers: RwLock<PassengerService<FakeClock>>,
    resources: RwLock<ResourceService<FakeClock>>,
    access: RwLock<AccessService<FakeClock, UsageSink>>,
    audit_sink: RwLock<AuditSink>,
    /// Present when `PRMS_DB_PATH` is set. Not behind a lock because it is
    /// immutable after construction (only `sync_all` is called on it, never
    /// replaced). `SqliteEntityStore` is internally Mutex-protected.
    entity_store: Option<SqliteEntityStore>,
}

/// Shared state held by every handler. `Clone` is cheap: all fields are
/// `Arc`-wrapped.
///
/// Uses per-aggregate `RwLock`s (via `WorldShards`) so concurrent reads on ANY
/// aggregate proceed without blocking, and concurrent writes to DIFFERENT
/// aggregates proceed without serialization. Only writes to the SAME aggregate
/// type must wait for the current writer.
#[derive(Clone)]
pub struct AppState {
    world: Arc<WorldShards>,
    /// Maps bearer token → actor ID string. Immutable after construction.
    api_keys: Arc<HashMap<String, String>>,
    /// Idempotency cache: `Idempotency-Key` header → cached response.
    idempotency: Arc<Mutex<HashMap<String, IdempotencyEntry>>>,
}

impl AppState {
    /// Decompose `world` into per-aggregate `RwLock`s.
    ///
    /// The public signature is unchanged from the single-lock version so
    /// `http_common::app()` and `serve.rs` require no modification.
    pub fn new(world: World, api_keys: HashMap<String, String>) -> Self {
        let World {
            crew_leads,
            passengers,
            resources,
            access,
            audit_sink,
            entity_store,
        } = world;
        Self {
            world: Arc::new(WorldShards {
                crew_leads: RwLock::new(crew_leads),
                passengers: RwLock::new(passengers),
                resources: RwLock::new(resources),
                access: RwLock::new(access),
                audit_sink: RwLock::new(audit_sink),
                entity_store,
            }),
            api_keys: Arc::new(api_keys),
            idempotency: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

/// Returns the current Unix timestamp in seconds (used for idempotency TTL).
fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Look up an idempotency key. Returns the cached response if present and unexpired.
/// Evicts stale entries opportunistically on every lookup.
fn idempotency_get(state: &AppState, key: &str) -> Option<Response> {
    let mut cache = state
        .idempotency
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let now = now_secs();
    // Opportunistic eviction of expired entries to bound memory growth.
    cache.retain(|_, v| v.expires_at > now);
    cache.get(key).map(|e| {
        // FIX: (StatusCode, Bytes).into_response() does not set Content-Type.
        // Explicitly set application/json so retried requests are indistinguishable
        // from the original 201 response.
        (
            e.status,
            [(axum::http::header::CONTENT_TYPE, "application/json")],
            e.body.clone(),
        )
            .into_response()
    })
}

/// Store a response body under an idempotency key for `IDEMPOTENCY_TTL_SECS`.
/// Only called after a successful (2xx) domain operation.
fn idempotency_put(state: &AppState, key: String, status: StatusCode, body: Bytes) {
    let mut cache = state
        .idempotency
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    cache.insert(
        key,
        IdempotencyEntry {
            status,
            body,
            expires_at: now_secs() + IDEMPOTENCY_TTL_SECS,
        },
    );
}

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
/// Rate limiting is **disabled** — use this in tests to avoid IP-based throttling.
pub fn router(state: AppState) -> Router {
    router_with(state, CorsOrigins::Any, false, false, 10, 50)
}

/// Build the axum router with explicit CORS configuration.
///
/// `enable_reset` — when `false` the `/reset` route is not registered,
/// making it impossible to wipe state via the HTTP API. Set to `true`
/// only for local dev / integration tests.
///
/// `enable_rate_limit` — when `true` attaches a per-IP governor layer.
/// Set to `false` in tests to avoid in-process requests all sharing the
/// same loopback IP exhausting the bucket.
///
/// `rate_limit_rps` — tokens replenished per second per IP (default 10).
///
/// `rate_limit_burst` — initial token credit before throttling (default 50).
///
/// # Panics
///
/// Panics if the `GovernorConfigBuilder` produces an invalid configuration,
/// i.e. if `rate_limit_rps` is 0.
pub fn router_with(
    state: AppState,
    cors_origins: CorsOrigins,
    enable_reset: bool,
    enable_rate_limit: bool,
    rate_limit_rps: u64,
    rate_limit_burst: u32,
) -> Router {
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
        .route("/health/ready", get(health_ready))
        .route("/metrics", get(metrics))
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
        // Rate limiting: configurable per-IP token-bucket governor.
        // Defends against accidental/malicious high-frequency clients
        // (OWASP A04 — Insecure Design, resource exhaustion).
        // `per_second(rps)` = 1 token every (1000/rps) ms.
        // `burst_size(burst)` = initial credit for short legitimate bursts.
        // Disabled in tests: in-process requests all share the same loopback IP,
        // which would exhaust the token bucket and cause spurious test failures.
        .layer(tower::util::option_layer(if enable_rate_limit {
            Some(GovernorLayer::new(std::sync::Arc::new(
                GovernorConfigBuilder::default()
                    .per_second(rate_limit_rps)
                    .burst_size(rate_limit_burst)
                    .finish()
                    .expect("valid governor config (rps must be non-zero)"),
            )))
        } else {
            None
        }))
        // `.layer(...)` wraps the entire router in a middleware. Layers
        // run in REVERSE registration order on the request side and in
        // declaration order on the response side (tower convention).
        .layer(cors)
        // Request-id: assign a UUID if the client did not send one,
        // then propagate it back on the response so logs can correlate.
        .layer(PropagateRequestIdLayer::new(x_request_id.clone()))
        .layer(SetRequestIdLayer::new(x_request_id, MakeRequestUuid))
        // Structured tracing span per request: logs method, URI, status,
        // and latency at INFO level. Correlates with request-id via
        // the propagated x-request-id header set above.
        .layer(TraceLayer::new_for_http())
        // FIX: security response headers (OWASP A05 — Security Misconfiguration).
        // SetResponseHeaderLayer::if_not_present preserves any value the handler
        // already set (e.g. Content-Type set by axum) and only injects the default
        // for headers that are absent — safe to stack multiple times.
        .layer(SetResponseHeaderLayer::if_not_present(
            axum::http::header::HeaderName::from_static("x-content-type-options"),
            axum::http::HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            axum::http::header::HeaderName::from_static("x-frame-options"),
            axum::http::HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            axum::http::header::HeaderName::from_static("referrer-policy"),
            axum::http::HeaderValue::from_static("no-referrer"),
        ))
        // FIX: Content-Security-Policy instructs browsers to block all
        // content sources (OWASP A05). A JSON API has no scripts, styles,
        // or media of its own; "default-src 'none'" is the strictest safe
        // value and prevents browsers from executing injected content if
        // a response is ever rendered as HTML by a misbehaving client.
        .layer(SetResponseHeaderLayer::if_not_present(
            axum::http::header::HeaderName::from_static("content-security-policy"),
            axum::http::HeaderValue::from_static("default-src 'none'"),
        ))
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

/// Return a 400 Bad Request response with a plain message.
/// Used by handlers to reject invalid input before reaching the domain.
fn bad_request(msg: &str) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorBody {
            error: msg.to_owned(),
            code: "InvalidInput".to_owned(),
        }),
    )
        .into_response()
}

/// Flush all entity state to `SQLite`. No-op when no entity store is configured.
///
/// Collects data under brief per-aggregate read locks, releases all locks,
/// then calls `sync_all` outside any lock — so I/O never blocks other handlers.
///
/// # Panics
/// Panics if any `RwLock` is poisoned or any `SQLite` write fails (a divergence
/// between in-memory and persistent state is unrecoverable, so crashing is correct).
fn flush_to_db(state: &AppState) {
    let Some(store) = &state.world.entity_store else {
        return;
    };
    // Collect under brief, sequentially-released read locks.
    let leads = state
        .world
        .crew_leads
        .read()
        .expect("crew_leads rwlock poisoned")
        .list()
        .to_vec();
    let (active_pax, deleted_pax) = {
        let pax = state
            .world
            .passengers
            .read()
            .expect("passengers rwlock poisoned");
        (pax.list().to_vec(), pax.deleted().to_vec())
    };
    let (active_res, deleted_res) = {
        let res = state
            .world
            .resources
            .read()
            .expect("resources rwlock poisoned");
        (res.list().to_vec(), res.deleted().to_vec())
    };
    // FIX: sync_all wraps all three entity tables in a single BEGIN IMMEDIATE /
    // COMMIT transaction so a crash mid-flush cannot produce split-brain state.
    store.sync_all(&leads, &active_pax, &deleted_pax, &active_res, &deleted_res);
}

/// Axum extractor that resolves an `Authorization: Bearer <token>` header
/// to the actor-ID string stored in `AppState::api_keys`.
/// Returns 401 Unauthorized if the header is absent or the token is unknown.
pub struct AuthActor(pub String);

impl FromRequestParts<AppState> for AuthActor {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // Extract "Authorization: Bearer <token>" header and strip the prefix.
        let token = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer "))
            .map(str::trim);

        // FIX: use constant-time comparison for every key in the map so that
        // timing differences cannot reveal which token prefixes are valid
        // (OWASP A07 — Identification and Authentication Failures).
        // The linear scan always visits ALL keys regardless of match position;
        // short-circuit on the first match would leak timing information.
        let actor_id: Option<String> = token.and_then(|t| {
            let t_bytes = t.as_bytes();
            let mut found: Option<&str> = None;
            for (key, actor) in state.api_keys.iter() {
                // ct_eq returns Choice(1) on match, Choice(0) on mismatch —
                // the comparison always runs to completion regardless of result.
                let eq: bool = key.as_bytes().ct_eq(t_bytes).into();
                if eq {
                    // Record the match but continue scanning so all keys are
                    // always visited (constant-time across all keys).
                    found = Some(actor.as_str());
                }
            }
            found.map(str::to_owned)
        });

        match actor_id {
            Some(id) => Ok(AuthActor(id)),
            // FIX: missing/unknown token returns 401 (not 403) — the caller
            // is unauthenticated, not merely unauthorised for this resource.
            None => Err((
                StatusCode::UNAUTHORIZED,
                Json(ErrorBody {
                    error: "missing or invalid bearer token".into(),
                    code: "Unauthorized".into(),
                }),
            )
                .into_response()),
        }
    }
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

#[utoipa::path(get, path = "/health/ready", tag = "system",
    responses(
        (status = 200, description = "System ready — entity counts included", body = HealthReadyDto),
        (status = 503, description = "World lock poisoned", body = ErrorBody),
    ))]
async fn health_ready(State(state): State<AppState>) -> Response {
    use crate::application::ports::UsageEventSource;
    // DB liveness check — entity_store is immutable after init, no lock needed.
    if let Some(false) = state
        .world
        .entity_store
        .as_ref()
        .map(SqliteEntityStore::ping_db)
    {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorBody {
                error: "database unreachable".into(),
                code: "DatabaseUnreachable".into(),
            }),
        )
            .into_response();
    }
    // Collect counts under per-aggregate read locks. Each lock is acquired and
    // released individually — a poisoned lock returns 503 for early detection.
    macro_rules! read_or_503 {
        ($lock:expr, $label:literal) => {
            match $lock.read() {
                Ok(g) => g,
                Err(_) => {
                    return (
                        StatusCode::SERVICE_UNAVAILABLE,
                        Json(ErrorBody {
                            error: concat!($label, " rwlock poisoned").into(),
                            code: "InternalError".into(),
                        }),
                    )
                        .into_response()
                }
            }
        };
    }
    let crew_leads_count = read_or_503!(state.world.crew_leads, "crew_leads")
        .list()
        .len();
    let passengers_count = read_or_503!(state.world.passengers, "passengers")
        .list()
        .len();
    let resources_count = read_or_503!(state.world.resources, "resources")
        .list()
        .len();
    let usage_count = read_or_503!(state.world.access, "access")
        .sink()
        .list()
        .len();
    let admin_count = read_or_503!(state.world.audit_sink, "audit_sink")
        .snapshot()
        .len();
    Json(HealthReadyDto {
        status: "ready".into(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
        crew_leads: crew_leads_count,
        passengers_active: passengers_count,
        resources_active: resources_count,
        usage_events: usage_count,
        admin_events: admin_count,
    })
    .into_response()
}

/// Prometheus text format metrics. Not included in the `OpenAPI` spec
/// (Prometheus scraping is a separate concern from the REST API).
async fn metrics(State(state): State<AppState>) -> impl IntoResponse {
    use crate::application::ports::UsageEventSource;
    use crate::domain::usage_event::Outcome;
    // Collect each metric under its own brief read lock.
    let crew_leads = state
        .world
        .crew_leads
        .read()
        .expect("crew_leads rwlock poisoned")
        .list()
        .len();
    let passengers = state
        .world
        .passengers
        .read()
        .expect("passengers rwlock poisoned")
        .list()
        .len();
    let resources = state
        .world
        .resources
        .read()
        .expect("resources rwlock poisoned")
        .list()
        .len();
    let admin = state
        .world
        .audit_sink
        .read()
        .expect("audit_sink rwlock poisoned")
        .snapshot()
        .len();
    let (usage_total, allowed, denied) = {
        let access = state.world.access.read().expect("access rwlock poisoned");
        let usage = access.sink().list();
        (
            usage.len(),
            usage
                .iter()
                .filter(|e| e.outcome == Outcome::Allowed)
                .count(),
            usage
                .iter()
                .filter(|e| e.outcome == Outcome::Denied)
                .count(),
        )
    };
    let body = format!(
        "# HELP prms_crew_leads_total Active crew leads.\n\
         # TYPE prms_crew_leads_total gauge\n\
         prms_crew_leads_total {crew_leads}\n\
         # HELP prms_passengers_active_total Active passengers.\n\
         # TYPE prms_passengers_active_total gauge\n\
         prms_passengers_active_total {passengers}\n\
         # HELP prms_resources_active_total Active resources.\n\
         # TYPE prms_resources_active_total gauge\n\
         prms_resources_active_total {resources}\n\
         # HELP prms_usage_events_total Total usage events recorded.\n\
         # TYPE prms_usage_events_total counter\n\
         prms_usage_events_total {usage_total}\n\
         # HELP prms_usage_events_allowed_total Usage events with Allowed outcome.\n\
         # TYPE prms_usage_events_allowed_total counter\n\
         prms_usage_events_allowed_total {allowed}\n\
         # HELP prms_usage_events_denied_total Usage events with Denied outcome.\n\
         # TYPE prms_usage_events_denied_total counter\n\
         prms_usage_events_denied_total {denied}\n\
         # HELP prms_admin_events_total Total admin events recorded.\n\
         # TYPE prms_admin_events_total counter\n\
         prms_admin_events_total {admin}\n",
    );
    (
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        body,
    )
}

#[utoipa::path(get, path = "/crew-leads", tag = "crew-leads",
    params(PaginationQuery),
    responses((status = 200, description = "All crew leads", body = Vec<CrewLeadDto>)))]
async fn list_crew_leads(
    State(state): State<AppState>,
    Query(page): Query<PaginationQuery>,
) -> Json<Vec<CrewLeadDto>> {
    let crew_leads = state
        .world
        .crew_leads
        .read()
        .expect("crew_leads rwlock poisoned");
    Json(
        crew_leads
            .list()
            .iter()
            .skip(page.offset())
            .take(page.limit())
            .map(CrewLeadDto::from)
            .collect(),
    )
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
    Path(old_id): Path<String>,
    AuthActor(actor_id): AuthActor,
    Json(req): Json<ReplaceCrewLeadReq>,
) -> Response {
    if let Err(msg) = req.new_lead.validate() {
        return bad_request(msg);
    }
    let new_id_for_log = req.new_lead.id.clone();
    let result = {
        let mut crew_leads = state
            .world
            .crew_leads
            .write()
            .expect("crew_leads rwlock poisoned");
        crew_leads.replace_audited(
            &CrewLeadId(actor_id.clone()),
            &CrewLeadId(old_id.clone()),
            req.new_lead.into(),
        )
    }; // write lock released before flush
    match result {
        Ok(()) => {
            flush_to_db(&state);
            tracing::info!(old_id = %old_id, new_id = %new_id_for_log, actor = %actor_id, "crew lead replaced");
            StatusCode::NO_CONTENT.into_response()
        }
        Err(e) => err_response_owned(&e),
    }
}

#[utoipa::path(get, path = "/passengers", tag = "passengers",
    params(PaginationQuery),
    responses((status = 200, body = Vec<PassengerDto>)))]
async fn list_passengers(
    State(state): State<AppState>,
    Query(page): Query<PaginationQuery>,
) -> Json<Vec<PassengerDto>> {
    let passengers = state
        .world
        .passengers
        .read()
        .expect("passengers rwlock poisoned");
    Json(
        passengers
            .list()
            .iter()
            .skip(page.offset())
            .take(page.limit())
            .map(PassengerDto::from)
            .collect(),
    )
}

#[utoipa::path(post, path = "/passengers", tag = "passengers",
    request_body = CreatePassengerReq,
    responses(
        (status = 201, body = PassengerDto),
        (status = 409, body = ErrorBody)))]
async fn create_passenger(
    State(state): State<AppState>,
    AuthActor(actor_id): AuthActor,
    headers: axum::http::HeaderMap,
    Json(req): Json<CreatePassengerReq>,
) -> Response {
    if let Err(msg) = req.validate() {
        return bad_request(msg);
    }
    let idem_key = headers
        .get("idempotency-key")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    if let Some(cached) = idem_key.as_deref().and_then(|k| idempotency_get(&state, k)) {
        return cached;
    }
    let actor = Actor::CrewLead(CrewLeadId(actor_id.clone()));
    let result = {
        let mut passengers = state
            .world
            .passengers
            .write()
            .expect("passengers rwlock poisoned");
        passengers.create(&actor, PassengerId(req.id), req.name, Tier::from(req.tier))
    }; // write lock released before flush
    match result {
        Ok(p) => {
            flush_to_db(&state);
            tracing::info!(passenger_id = %p.id.0, tier = ?p.tier, actor = %actor_id, "passenger created");
            let dto = PassengerDto::from(&p);
            let body = serde_json::to_vec(&dto).expect("PassengerDto serialization is infallible");
            if let Some(key) = idem_key {
                idempotency_put(&state, key, StatusCode::CREATED, Bytes::from(body));
            }
            (StatusCode::CREATED, Json(dto)).into_response()
        }
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
    AuthActor(actor_id): AuthActor,
    Json(req): Json<ChangeTierReq>,
) -> Response {
    let actor = Actor::CrewLead(CrewLeadId(actor_id.clone()));
    let result = {
        let mut passengers = state
            .world
            .passengers
            .write()
            .expect("passengers rwlock poisoned");
        passengers.change_tier(&actor, &PassengerId(id.clone()), Tier::from(req.tier))
    };
    match result {
        Ok(()) => {
            flush_to_db(&state);
            tracing::info!(passenger_id = %id, tier = ?req.tier, actor = %actor_id, "passenger tier changed");
            StatusCode::NO_CONTENT.into_response()
        }
        Err(e) => err_response_owned(&e),
    }
}

#[utoipa::path(delete, path = "/passengers/{id}", tag = "passengers",
    params(("id" = String, Path)),
    responses((status = 204), (status = 401, body = ErrorBody), (status = 404, body = ErrorBody)))]
async fn soft_delete_passenger(
    State(state): State<AppState>,
    Path(id): Path<String>,
    AuthActor(actor_id): AuthActor,
) -> Response {
    let actor = Actor::CrewLead(CrewLeadId(actor_id.clone()));
    let result = {
        let mut passengers = state
            .world
            .passengers
            .write()
            .expect("passengers rwlock poisoned");
        passengers.soft_delete(&actor, &PassengerId(id.clone()))
    };
    match result {
        Ok(()) => {
            flush_to_db(&state);
            tracing::info!(passenger_id = %id, actor = %actor_id, "passenger soft-deleted");
            StatusCode::NO_CONTENT.into_response()
        }
        Err(e) => err_response_owned(&e),
    }
}

#[utoipa::path(get, path = "/resources", tag = "resources",
    params(PaginationQuery),
    responses((status = 200, body = Vec<ResourceDto>)))]
async fn list_resources(
    State(state): State<AppState>,
    Query(page): Query<PaginationQuery>,
) -> Json<Vec<ResourceDto>> {
    let resources = state
        .world
        .resources
        .read()
        .expect("resources rwlock poisoned");
    Json(
        resources
            .list()
            .iter()
            .skip(page.offset())
            .take(page.limit())
            .map(ResourceDto::from)
            .collect(),
    )
}

#[utoipa::path(post, path = "/resources", tag = "resources",
    request_body = CreateResourceReq,
    responses(
        (status = 201, body = ResourceDto),
        (status = 409, body = ErrorBody)))]
async fn create_resource(
    State(state): State<AppState>,
    AuthActor(actor_id): AuthActor,
    headers: axum::http::HeaderMap,
    Json(req): Json<CreateResourceReq>,
) -> Response {
    if let Err(msg) = req.validate() {
        return bad_request(msg);
    }
    let idem_key = headers
        .get("idempotency-key")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    if let Some(cached) = idem_key.as_deref().and_then(|k| idempotency_get(&state, k)) {
        return cached;
    }
    let actor = Actor::CrewLead(CrewLeadId(actor_id.clone()));
    let result = {
        let mut resources = state
            .world
            .resources
            .write()
            .expect("resources rwlock poisoned");
        resources.create(
            &actor,
            ResourceId(req.id),
            req.name,
            req.category,
            Tier::from(req.min_tier),
        )
    }; // write lock released before flush
    match result {
        Ok(r) => {
            flush_to_db(&state);
            tracing::info!(resource_id = %r.id.0, min_tier = ?r.min_tier, actor = %actor_id, "resource created");
            let dto = ResourceDto::from(&r);
            let body = serde_json::to_vec(&dto).expect("ResourceDto serialization is infallible");
            if let Some(key) = idem_key {
                idempotency_put(&state, key, StatusCode::CREATED, Bytes::from(body));
            }
            (StatusCode::CREATED, Json(dto)).into_response()
        }
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
    AuthActor(actor_id): AuthActor,
    Json(req): Json<ChangeTierReq>,
) -> Response {
    let actor = Actor::CrewLead(CrewLeadId(actor_id.clone()));
    let result = {
        let mut resources = state
            .world
            .resources
            .write()
            .expect("resources rwlock poisoned");
        resources.change_min_tier(&actor, &ResourceId(id.clone()), Tier::from(req.tier))
    };
    match result {
        Ok(()) => {
            flush_to_db(&state);
            tracing::info!(resource_id = %id, min_tier = ?req.tier, actor = %actor_id, "resource min-tier changed");
            StatusCode::NO_CONTENT.into_response()
        }
        Err(e) => err_response_owned(&e),
    }
}

#[utoipa::path(delete, path = "/resources/{id}", tag = "resources",
    params(("id" = String, Path)),
    responses((status = 204), (status = 401, body = ErrorBody), (status = 404, body = ErrorBody)))]
async fn soft_delete_resource(
    State(state): State<AppState>,
    Path(id): Path<String>,
    AuthActor(actor_id): AuthActor,
) -> Response {
    let actor = Actor::CrewLead(CrewLeadId(actor_id.clone()));
    let result = {
        let mut resources = state
            .world
            .resources
            .write()
            .expect("resources rwlock poisoned");
        resources.soft_delete(&actor, &ResourceId(id.clone()))
    };
    match result {
        Ok(()) => {
            flush_to_db(&state);
            tracing::info!(resource_id = %id, actor = %actor_id, "resource soft-deleted");
            StatusCode::NO_CONTENT.into_response()
        }
        Err(e) => err_response_owned(&e),
    }
}

#[utoipa::path(post, path = "/access", tag = "access",
    request_body = UseResourceReq,
    responses(
        (status = 200, body = UsageEventDto),
        (status = 403, body = ErrorBody)))]
async fn use_resource(
    State(state): State<AppState>,
    AuthActor(actor_id): AuthActor,
    Json(req): Json<UseResourceReq>,
) -> Response {
    if let Err(msg) = req.validate() {
        return bad_request(msg);
    }
    let actor = Actor::Passenger(PassengerId(actor_id.clone()));
    // Acquire in canonical order (passengers → resources → access) to prevent
    // deadlocks if another handler holds a subset of these locks concurrently.
    // Per-aggregate locks replace the old borrow-splitting pattern:
    //   passengers and resources are read-locked (shareable),
    //   access is write-locked (exclusive, needed to append the usage event).
    let passengers = state
        .world
        .passengers
        .read()
        .expect("passengers rwlock poisoned");
    let resources = state
        .world
        .resources
        .read()
        .expect("resources rwlock poisoned");
    let mut access = state.world.access.write().expect("access rwlock poisoned");
    match access.use_resource(
        &actor,
        &*passengers,
        &*resources,
        &ResourceId(req.resource_id),
    ) {
        Ok(ev) => {
            tracing::info!(
                passenger_id = %actor_id,
                resource_id = %ev.resource_id.0,
                outcome = ?ev.outcome,
                "resource access recorded"
            );
            (StatusCode::OK, Json(UsageEventDto::from(&ev))).into_response()
        }
        Err(e) => err_response_owned(&e),
    }
}

#[utoipa::path(get, path = "/audit", tag = "audit",
    params(PaginationQuery),
    responses((status = 200, body = Vec<AdminEventDto>)))]
async fn list_admin_events(
    State(state): State<AppState>,
    Query(page): Query<PaginationQuery>,
) -> Json<Vec<AdminEventDto>> {
    let audit_sink = state
        .world
        .audit_sink
        .read()
        .expect("audit_sink rwlock poisoned");
    Json(
        audit_sink
            .snapshot()
            .iter()
            .skip(page.offset())
            .take(page.limit())
            .map(AdminEventDto::from)
            .collect(),
    )
}

#[utoipa::path(get, path = "/usage", tag = "audit",
    params(PaginationQuery),
    responses((status = 200, body = Vec<UsageEventDto>)))]
async fn list_usage_events(
    State(state): State<AppState>,
    Query(page): Query<PaginationQuery>,
) -> Json<Vec<UsageEventDto>> {
    use crate::application::ports::UsageEventSource;
    let access = state.world.access.read().expect("access rwlock poisoned");
    Json(
        access
            .sink()
            .list()
            .iter()
            .skip(page.offset())
            .take(page.limit())
            .map(UsageEventDto::from)
            .collect(),
    )
}

#[utoipa::path(get, path = "/reports/by-tier", tag = "reports",
    responses((status = 200, body = Vec<TierCountsDto>)))]
async fn report_by_tier(State(state): State<AppState>) -> Json<Vec<TierCountsDto>> {
    use crate::application::reporting_service::ReportingService;
    let access = state.world.access.read().expect("access rwlock poisoned");
    let report = ReportingService::new(access.sink()).aggregate_by_tier();
    let mut rows: Vec<TierCountsDto> = report
        .into_iter()
        .map(|(tier, c)| TierCountsDto {
            tier: tier.into(),
            allowed: c.allowed,
            denied: c.denied,
        })
        .collect();
    rows.sort_by_key(|r| match r.tier {
        TierDto::Silver => 0,
        TierDto::Gold => 1,
        TierDto::Diamond => 2,
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
    let access = state.world.access.read().expect("access rwlock poisoned");
    let n = q.n.unwrap_or(5);
    Json(
        ReportingService::new(access.sink())
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
    let access = state.world.access.read().expect("access rwlock poisoned");
    Json(
        ReportingService::new(access.sink())
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
async fn add_crew_lead(
    State(state): State<AppState>,
    AuthActor(actor_id): AuthActor,
    Json(req): Json<AddCrewLeadReq>,
) -> Response {
    let new_id = req.lead.id.clone();
    let result = {
        let mut crew_leads = state
            .world
            .crew_leads
            .write()
            .expect("crew_leads rwlock poisoned");
        crew_leads.add(req.lead.into())
    };
    match result {
        Ok(()) => {
            flush_to_db(&state);
            tracing::info!(new_id = %new_id, actor = %actor_id, "crew lead added");
            StatusCode::NO_CONTENT.into_response()
        }
        Err(e) => err_response_owned(&e),
    }
}

#[utoipa::path(delete, path = "/crew-leads/{id}", tag = "crew-leads",
    params(("id" = String, Path)),
    responses(
        (status = 204),
        (status = 401, body = ErrorBody),
        (status = 409, body = ErrorBody)))]
async fn remove_crew_lead(
    State(state): State<AppState>,
    AuthActor(actor_id): AuthActor,
    Path(id): Path<String>,
) -> Response {
    let result = {
        let mut crew_leads = state
            .world
            .crew_leads
            .write()
            .expect("crew_leads rwlock poisoned");
        crew_leads.remove(&CrewLeadId(id.clone()))
    };
    match result {
        Ok(()) => {
            flush_to_db(&state);
            tracing::info!(removed_id = %id, actor = %actor_id, "crew lead removed");
            StatusCode::NO_CONTENT.into_response()
        }
        Err(e) => err_response_owned(&e),
    }
}

#[utoipa::path(get, path = "/passengers/{id}", tag = "passengers",
    params(("id" = String, Path)),
    responses((status = 200, body = PassengerDto), (status = 404, body = ErrorBody)))]
async fn get_passenger(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    let passengers = state
        .world
        .passengers
        .read()
        .expect("passengers rwlock poisoned");
    match passengers.get(&PassengerId(id)) {
        Ok(p) => (StatusCode::OK, Json(PassengerDto::from(p))).into_response(),
        Err(e) => err_response_owned(&e),
    }
}

#[utoipa::path(get, path = "/resources/{id}", tag = "resources",
    params(("id" = String, Path)),
    responses((status = 200, body = ResourceDto), (status = 404, body = ErrorBody)))]
async fn get_resource(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    let resources = state
        .world
        .resources
        .read()
        .expect("resources rwlock poisoned");
    match resources.get(&ResourceId(id)) {
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
    let resources = state
        .world
        .resources
        .read()
        .expect("resources rwlock poisoned");
    Json(
        resources
            .list_accessible_for(Tier::from(q.tier))
            .iter()
            .map(ResourceDto::from)
            .collect(),
    )
}

#[utoipa::path(post, path = "/reset", tag = "system",
    responses((status = 204), (status = 401, body = ErrorBody), (status = 403, body = ErrorBody)))]
async fn reset_world(State(state): State<AppState>, AuthActor(actor_id): AuthActor) -> Response {
    // Gate: caller must be an existing crew lead.
    {
        let crew_leads = state
            .world
            .crew_leads
            .read()
            .expect("crew_leads rwlock poisoned");
        let id = CrewLeadId(actor_id.clone());
        if !crew_leads.list().iter().any(|c| c.id == id) {
            return err_response_owned(&DomainError::UnauthorizedActor);
        }
    } // read lock released before write locks below

    let fresh = match build_demo_world() {
        Ok(w) => w,
        Err(e) => return err_response_owned(&e),
    };
    let World {
        crew_leads: new_cl,
        passengers: new_pax,
        resources: new_res,
        access: new_acc,
        audit_sink: new_aud,
        // entity_store: keep the existing store — don't replace it
        ..
    } = fresh;

    // Acquire all five write locks in canonical order to atomically replace
    // all aggregates. Readers are blocked for the brief replacement window.
    // This is acceptable: reset_world is a demo-only endpoint.
    {
        let mut cl = state
            .world
            .crew_leads
            .write()
            .expect("crew_leads rwlock poisoned");
        let mut pax = state
            .world
            .passengers
            .write()
            .expect("passengers rwlock poisoned");
        let mut res = state
            .world
            .resources
            .write()
            .expect("resources rwlock poisoned");
        let mut acc = state.world.access.write().expect("access rwlock poisoned");
        let mut aud = state
            .world
            .audit_sink
            .write()
            .expect("audit_sink rwlock poisoned");
        *cl = new_cl;
        *pax = new_pax;
        *res = new_res;
        *acc = new_acc;
        *aud = new_aud;
    } // all write locks released before flush

    flush_to_db(&state);
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
        health, health_ready,
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
        CrewLeadDto, AddCrewLeadReq, ReplaceCrewLeadReq,
        PassengerDto, CreatePassengerReq, ChangeTierReq,
        ResourceDto, CreateResourceReq,
        UseResourceReq,
        UsageEventDto, AdminEventDto,
        TierCountsDto, TopResourceDto,
        HealthReadyDto,
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
