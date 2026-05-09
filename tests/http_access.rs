// HTTP integration tests for the access endpoint. The full pattern
// for this file is documented in `tests/http_health.rs`; from here on
// each http_*.rs file just calls `app()` + `send(req(...))` and
// asserts on `(status, body)`.
#![cfg(feature = "http")]

mod http_common;

use axum::http::{Method, StatusCode};
use serde_json::json;

use http_common::{PS_TOKEN, app, auth_req, req, send};

#[tokio::test]
async fn access_allowed_returns_event() {
    let app = app();
    let (status, body) = send(
        &app,
        auth_req(
            Method::POST,
            "/access",
            PS_TOKEN,
            Some(json!({"resource_id": "res-lounge"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["outcome"], "Allowed");
}

#[tokio::test]
async fn access_denied_returns_403_with_event_recorded() {
    let app = app();
    let (status, body) = send(
        &app,
        auth_req(
            Method::POST,
            "/access",
            PS_TOKEN,
            Some(json!({"resource_id": "res-bridge"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["code"], "AccessDenied");
    let (_, usage) = send(&app, req(Method::GET, "/usage", None)).await;
    let arr = usage.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["outcome"], "Denied");
}
