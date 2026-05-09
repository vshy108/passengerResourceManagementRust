// HTTP integration tests for /crew-leads endpoints (list, add,
// remove, replace). See `tests/http_health.rs` for harness details.
#![cfg(feature = "http")]

mod http_common;

use axum::http::{Method, StatusCode};
use serde_json::json;

use http_common::{ARIA, CL_TOKEN, app, auth_req, req, send};

#[tokio::test]
async fn list_crew_leads_returns_three_seeded_leads() {
    let app = app();
    let (status, body) = send(&app, req(Method::GET, "/crew-leads", None)).await;
    assert_eq!(status, StatusCode::OK);
    let arr = body.as_array().expect("array");
    assert_eq!(arr.len(), 3);
    assert_eq!(arr[0]["id"], ARIA);
}

#[tokio::test]
async fn add_crew_lead_returns_409_limit_reached() {
    let app = app();
    let (status, body) = send(
        &app,
        auth_req(
            Method::POST,
            "/crew-leads",
            CL_TOKEN,
            Some(json!({"lead": {"id": "cl-x", "name": "X"}})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "CrewLeadLimitReached");
}

#[tokio::test]
async fn remove_crew_lead_returns_409_minimum_breached() {
    let app = app();
    let (status, body) = send(
        &app,
        auth_req(Method::DELETE, "/crew-leads/cl-aria", CL_TOKEN, None),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "CrewLeadMinimumBreached");
}

#[tokio::test]
async fn replace_crew_lead_returns_204_and_updates_list() {
    let app = app();
    let (status, _) = send(
        &app,
        auth_req(
            Method::PUT,
            "/crew-leads/cl-aria",
            CL_TOKEN,
            Some(json!({"new_lead": {"id": "cl-aria2", "name": "Aria 2"}})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, body) = send(&app, req(Method::GET, "/crew-leads", None)).await;
    // `iter().map(...).collect::<Vec<&str>>()` extracts each id as a
    // borrowed string slice from the JSON array. The turbofish-free
    // form works here because the binding `let ids: Vec<&str>` gives
    // collect() its target type. `as_str().unwrap()` is acceptable in
    // tests — in production we'd return a `Result` instead.
    let ids: Vec<&str> = body
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["id"].as_str().unwrap())
        .collect();
    assert!(ids.contains(&"cl-aria2"));
    assert!(!ids.contains(&"cl-aria"));
}

#[tokio::test]
async fn replace_crew_lead_unknown_id_returns_404() {
    let app = app();
    let (status, body) = send(
        &app,
        auth_req(
            Method::PUT,
            "/crew-leads/cl-zzz",
            CL_TOKEN,
            Some(json!({"new_lead": {"id": "cl-y", "name": "Y"}})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], "CrewLeadNotFound");
}
