// File-level `#![cfg(feature = "http")]`: when the `http` Cargo
// feature is OFF, this entire file compiles to nothing — no test
// binary, no axum dependency. Pairs with the same gate in `src/`.
#![cfg(feature = "http")]

// `mod http_common;` looks for `tests/http_common/mod.rs` (or
// `tests/http_common.rs`) and includes it as a private module. This
// is HOW we share the `app()`/`req()`/`send()` helpers across each
// `tests/http_*.rs` binary; Cargo would otherwise compile the helper
// file as its OWN test binary, which we don't want.
mod http_common;

use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use http_body_util::BodyExt;
// `serde_json::json!` is a macro that builds a `Value` from JSON-like
// syntax: `json!({"a": 1})` -> a `Value::Object` with one entry.
// Lifesaver in tests — no need to define a struct just to make a request.
use serde_json::{Value, json};
use tower::ServiceExt;

use http_common::{CL_TOKEN, app, auth_req, req, send};

use std::collections::HashMap;

use passenger_resource_management::interface::composition_root::build_demo_world;
use passenger_resource_management::interface::http::{AppState, CorsOrigins, router_with};

// `#[tokio::test]` is the async equivalent of `#[test]` — the macro
// wraps the test in a tokio runtime so we can `.await` futures inside.
// REQUIRED whenever the test calls async axum/tower code.
#[tokio::test]
async fn health_returns_ok() {
    let app = app();
    let (status, body) = send(&app, req(Method::GET, "/health", None)).await;
    assert_eq!(status, StatusCode::OK);
    // Indexing a `serde_json::Value` is dynamic — returns a default
    // `Value::Null` for missing keys instead of panicking. Convert with
    // `.as_str()` / `.as_array()` / etc., each returning Option.
    assert_eq!(body, Value::String("ok".into()));
}

#[tokio::test]
async fn openapi_json_lists_paths_and_schemas() {
    let app = app();
    let (status, body) = send(&app, req(Method::GET, "/openapi.json", None)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["openapi"].as_str().unwrap_or(""), "3.1.0");
    assert!(body["info"]["title"].as_str().unwrap().contains("PRMS"));
    let paths = body["paths"].as_object().expect("paths object");
    assert!(paths.contains_key("/health"));
    assert!(paths.contains_key("/passengers"));
    assert!(paths.contains_key("/access"));
    let schemas = body["components"]["schemas"]
        .as_object()
        .expect("schemas object");
    assert!(schemas.contains_key("PassengerDto"));
    assert!(schemas.contains_key("TierDto"));
}

#[tokio::test]
async fn request_id_is_assigned_and_propagated() {
    let app = app();
    let r = req(Method::GET, "/health", None);
    let res = app.clone().oneshot(r).await.unwrap();
    let id = res
        .headers()
        .get("x-request-id")
        .expect("x-request-id header")
        .to_str()
        .unwrap()
        .to_string();
    assert!(!id.is_empty());
    // Drain body so the test fully completes the response.
    let _ = res.into_body().collect().await.unwrap();
}

#[tokio::test]
async fn request_id_echoes_client_supplied_value() {
    let app = app();
    let r = Request::builder()
        .method(Method::GET)
        .uri("/health")
        .header("x-request-id", "client-supplied-123")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(r).await.unwrap();
    assert_eq!(
        res.headers().get("x-request-id").unwrap(),
        "client-supplied-123"
    );
    let _ = res.into_body().collect().await.unwrap();
}

#[tokio::test]
async fn unknown_route_returns_404() {
    let app = app();
    let (status, _) = send(&app, req(Method::GET, "/does-not-exist", None)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn auth_check_returns_actor_for_valid_token() {
    let app = app();
    let (status, body) = send(&app, auth_req(Method::GET, "/auth/check", CL_TOKEN, None)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["actor_id"], "cl-aria");
}

#[tokio::test]
async fn auth_check_rejects_missing_token() {
    let app = app();
    let (status, body) = send(&app, req(Method::GET, "/auth/check", None)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["code"], "Unauthorized");
}

#[tokio::test]
async fn malformed_json_body_returns_4xx() {
    let app = app();
    let bad = Request::builder()
        .method(Method::POST)
        .uri("/access")
        .header("content-type", "application/json")
        .body(Body::from("{not-json"))
        .expect("request");
    let (status, _) = send(&app, bad).await;
    assert!(status.is_client_error());
}

#[tokio::test]
async fn unknown_tier_string_is_rejected() {
    let app = app();
    let (status, _) = send(
        &app,
        auth_req(
            Method::POST,
            "/passengers",
            CL_TOKEN,
            Some(json!({
                "id": "ps-x",
                "name": "X",
                "tier": "Bronze"
            })),
        ),
    )
    .await;
    assert!(status.is_client_error());
}

#[tokio::test]
async fn oversized_body_is_rejected_with_413() {
    let app = app();
    // `"x".repeat(n)` -> a String of `n` copies. Used here to build a
    // body deliberately larger than the 64 KiB cap set in http.rs, to
    // verify the `DefaultBodyLimit` middleware rejects it with 413.
    let huge = "x".repeat(70 * 1024);
    let body = json!({
        "id": "ps-x",
        "name": huge,
        "tier": "Silver"
    });
    let r = Request::builder()
        .method(Method::POST)
        .uri("/passengers")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {CL_TOKEN}"))
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();
    let res = app.clone().oneshot(r).await.unwrap();
    assert_eq!(res.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn health_ready_returns_entity_counts() {
    let app = app();
    let (status, body) = send(&app, req(Method::GET, "/health/ready", None)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"].as_str().unwrap(), "ready");
    // Version field must be present and non-empty (populated from CARGO_PKG_VERSION).
    assert!(!body["version"].as_str().unwrap_or("").is_empty());
    // Demo world seeds 3 crew leads, 3 passengers, 3 resources.
    assert_eq!(body["crew_leads"].as_u64().unwrap(), 3);
    assert_eq!(body["passengers_active"].as_u64().unwrap(), 3);
    assert_eq!(body["resources_active"].as_u64().unwrap(), 3);
    // No access events yet in a fresh world.
    assert_eq!(body["usage_events"].as_u64().unwrap(), 0);
}

#[tokio::test]
async fn metrics_returns_prometheus_text() {
    let app = app();
    let r = req(Method::GET, "/metrics", None);
    let res = app.clone().oneshot(r).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let ct = res.headers().get("content-type").unwrap().to_str().unwrap();
    assert!(ct.contains("text/plain"));
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let text = std::str::from_utf8(&bytes).unwrap();
    // Check a representative metric is present.
    assert!(text.contains("prms_crew_leads_total 3"));
    assert!(text.contains("prms_passengers_active_total 3"));
    assert!(text.contains("prms_resources_active_total 3"));
    assert!(text.contains("prms_usage_events_total 0"));
}

#[tokio::test]
async fn metrics_counts_allowed_and_denied_after_access_events() {
    let app = app();
    // FIX: the `allowed`/`denied` filter closures in `metrics()` (http.rs
    // lines 483-484) are only executed when there are usage events. Drive
    // one allowed + one denied event so both filter arms execute.
    let _ = send(
        &app,
        auth_req(
            Method::POST,
            "/access",
            "test-ps-001",
            Some(json!({"resource_id": "res-lounge"})),
        ),
    )
    .await;
    let _ = send(
        &app,
        auth_req(
            Method::POST,
            "/access",
            "test-ps-001",
            Some(json!({"resource_id": "res-bridge"})),
        ),
    )
    .await;
    let r = req(Method::GET, "/metrics", None);
    let res = app.clone().oneshot(r).await.unwrap();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let text = std::str::from_utf8(&bytes).unwrap();
    assert!(text.contains("prms_usage_events_total 2"));
    assert!(text.contains("prms_usage_events_allowed_total 1"));
    assert!(text.contains("prms_usage_events_denied_total 1"));
}

#[tokio::test]
async fn security_response_headers_are_set() {
    // Verifies that every security header injected by router_with()'s
    // SetResponseHeaderLayer stack is present on a plain GET response.
    let app = app();
    let r = req(Method::GET, "/health", None);
    let res = app.clone().oneshot(r).await.unwrap();
    let h = res.headers();
    assert_eq!(h.get("x-content-type-options").unwrap(), "nosniff");
    assert_eq!(h.get("x-frame-options").unwrap(), "DENY");
    assert_eq!(h.get("referrer-policy").unwrap(), "no-referrer");
    // FIX: content-security-policy was missing; added in production-gap pass.
    // 'default-src none' blocks all browser content sources for this JSON API.
    assert_eq!(
        h.get("content-security-policy").unwrap(),
        "default-src 'none'"
    );
    let _ = res.into_body().collect().await.unwrap();
}

#[tokio::test]
async fn cors_list_origins_allows_listed_origin() {
    // FIX: CorsOrigins::List branch in router_with() was never reached by
    // any test. This test exercises the branch that actually enforces origin
    // restrictions (http.rs CorsLayer::new().allow_origin(origins) arm).
    let world = build_demo_world().expect("bootstrap");
    let api_keys: HashMap<String, String> =
        [("test-cl-aria".to_owned(), "cl-aria".to_owned())].into();
    let state = AppState::new(world, api_keys);
    // Allow only example.com as an origin.
    let allowed: axum::http::HeaderValue = "http://example.com".parse().unwrap();
    let list_app = router_with(
        state,
        CorsOrigins::List(vec![allowed]),
        false,
        false,
        10,
        50,
    );

    let r = Request::builder()
        .method(Method::GET)
        .uri("/health")
        .header("origin", "http://example.com")
        .body(Body::empty())
        .unwrap();
    let res = list_app.clone().oneshot(r).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // The CORS middleware echoes the listed origin back on the response.
    let acao = res.headers().get("access-control-allow-origin");
    assert_eq!(acao.unwrap(), "http://example.com");
    let _ = res.into_body().collect().await.unwrap();
}

#[tokio::test]
async fn rate_limit_returns_429_after_burst_exhausted() {
    // FIX: the GovernorLayer rate-limit path (http.rs lines 177-180) was
    // never tested. With rps=1 and burst=1 the second back-to-back request
    // to the same loopback IP exhausts the token bucket and returns 429.
    let world = build_demo_world().expect("bootstrap");
    let state = AppState::new(world, HashMap::new());
    // enable_rate_limit=true, rps=1, burst=1 — strict bucket for testing.
    let rate_app = router_with(state, CorsOrigins::Any, false, true, 1, 1);

    // FIX: tower's oneshot provides no real TCP socket, so PeerIpKeyExtractor
    // cannot find a SocketAddr. Inject ConnectInfo<SocketAddr> as a request
    // extension — the same mechanism axum's IntoMakeServiceWithConnectInfo
    // uses for real connections — so the governor has a valid IP to key on.
    let make_req = || {
        use axum::extract::ConnectInfo;
        use std::net::SocketAddr;
        let mut r = Request::builder()
            .method(Method::GET)
            .uri("/health")
            .body(Body::empty())
            .unwrap();
        r.extensions_mut()
            .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 12345))));
        r
    };

    // First request — consumes the single burst token and succeeds.
    let res1 = rate_app.clone().oneshot(make_req()).await.unwrap();
    assert_eq!(res1.status(), StatusCode::OK);
    let _ = res1.into_body().collect().await.unwrap();

    // Second immediate request — bucket is empty, must be rate-limited.
    let res2 = rate_app.clone().oneshot(make_req()).await.unwrap();
    assert_eq!(res2.status(), StatusCode::TOO_MANY_REQUESTS);
    let _ = res2.into_body().collect().await.unwrap();
}
