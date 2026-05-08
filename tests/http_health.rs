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

use http_common::{ARIA, app, req, send};

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
        req(
            Method::POST,
            "/passengers",
            Some(json!({
                "actor_id": ARIA,
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
        "actor_id": ARIA,
        "id": "ps-x",
        "name": huge,
        "tier": "Silver"
    });
    let r = Request::builder()
        .method(Method::POST)
        .uri("/passengers")
        .header("content-type", "application/json")
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
    let ct = res
        .headers()
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(ct.contains("text/plain"));
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let text = std::str::from_utf8(&bytes).unwrap();
    // Check a representative metric is present.
    assert!(text.contains("prms_crew_leads_total 3"));
    assert!(text.contains("prms_passengers_active_total 3"));
    assert!(text.contains("prms_resources_active_total 3"));
    assert!(text.contains("prms_usage_events_total 0"));
}
