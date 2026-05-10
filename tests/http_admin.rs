// HTTP integration tests for the /reset admin endpoint.
// See `tests/http_health.rs` for an explanation of the test harness.
#![cfg(feature = "http")]

mod http_common;

use axum::http::{Method, StatusCode};

use http_common::{CL_TOKEN, app, auth_req, req, send};

#[tokio::test]
async fn reset_replaces_state_with_fresh_demo_world() {
    let app = app();
    // Mutate something.
    let _ = send(
        &app,
        auth_req(Method::DELETE, "/passengers/ps-001", CL_TOKEN, None),
    )
    .await;
    let (_, before) = send(&app, req(Method::GET, "/passengers", None)).await;
    assert_eq!(before.as_array().unwrap().len(), 2);

    let (status, _) = send(&app, auth_req(Method::POST, "/reset", CL_TOKEN, None)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (_, after) = send(&app, req(Method::GET, "/passengers", None)).await;
    assert_eq!(after.as_array().unwrap().len(), 3);
}

#[tokio::test]
async fn reset_rejects_unknown_actor_with_403() {
    let app = app();
    // FIX: unknown-actor token → resolves to an actor-id not in crew-lead list → 403.
    // A completely invalid token (not in api_keys at all) → 401, so we need a
    // token that maps to a non-crew-lead actor. We test 401 separately.
    let (status, body) = send(
        &app,
        // PS_TOKEN maps to "ps-001" which is NOT a crew lead → 403 UnauthorizedActor.
        auth_req(Method::POST, "/reset", "test-ps-001", None),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["code"], "UnauthorizedActor");
}
