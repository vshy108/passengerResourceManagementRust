// HTTP integration tests for the /reset admin endpoint.
// See `tests/http_health.rs` for an explanation of the test harness.
#![cfg(feature = "http")]

mod http_common;

use axum::http::{Method, StatusCode};
use serde_json::json;

use http_common::{ARIA, app, req, send};

#[tokio::test]
async fn reset_replaces_state_with_fresh_demo_world() {
    let app = app();
    // Mutate something.
    let _ = send(
        &app,
        req(
            Method::DELETE,
            "/passengers/ps-001",
            Some(json!({"actor_id": ARIA})),
        ),
    )
    .await;
    let (_, before) = send(&app, req(Method::GET, "/passengers", None)).await;
    assert_eq!(before.as_array().unwrap().len(), 2);

    let (status, _) = send(
        &app,
        req(Method::POST, "/reset", Some(json!({"actor_id": ARIA}))),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (_, after) = send(&app, req(Method::GET, "/passengers", None)).await;
    assert_eq!(after.as_array().unwrap().len(), 3);
}

#[tokio::test]
async fn reset_rejects_unknown_actor_with_403() {
    let app = app();
    let (status, body) = send(
        &app,
        req(Method::POST, "/reset", Some(json!({"actor_id": "nobody"}))),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["code"], "UnauthorizedActor");
}
