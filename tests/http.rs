//! Integration tests for the axum HTTP adapter
//! (`src/interface/http.rs`). These run only when the `http` feature
//! is enabled — guarded with `#![cfg(feature = "http")]` so the
//! default `cargo test` / `cargo nextest run` invocation still
//! compiles.
//!
//! No real network: requests go through `tower::ServiceExt::oneshot`.

#![cfg(feature = "http")]

use std::sync::{Arc, Mutex};

use axum::{
    Router,
    body::Body,
    http::{Method, Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

use passenger_resource_management::interface::composition_root::build_demo_world;
use passenger_resource_management::interface::http::router;

const ARIA: &str = "cl-aria";

fn app() -> Router {
    let world = build_demo_world().expect("bootstrap");
    let state = Arc::new(Mutex::new(world));
    router(state)
}

async fn send(app: &Router, req: Request<Body>) -> (StatusCode, Value) {
    let res = app.clone().oneshot(req).await.expect("response");
    let status = res.status();
    let bytes = res.into_body().collect().await.expect("body").to_bytes();
    let body = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes)
            .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into_owned()))
    };
    (status, body)
}

fn req(method: Method, path: &str, body: Option<Value>) -> Request<Body> {
    let mut b = Request::builder().method(method).uri(path);
    let body = match body {
        Some(v) => {
            b = b.header("content-type", "application/json");
            Body::from(serde_json::to_vec(&v).expect("json"))
        }
        None => Body::empty(),
    };
    b.body(body).expect("request")
}

// ---------- health -----------------------------------------------------

#[tokio::test]
async fn health_returns_ok() {
    let app = app();
    let (status, body) = send(&app, req(Method::GET, "/health", None)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, Value::String("ok".into()));
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

// ---------- crew leads -------------------------------------------------

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
        req(
            Method::POST,
            "/crew-leads",
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
        req(
            Method::DELETE,
            "/crew-leads/cl-aria",
            Some(json!({"actor_id": ARIA})),
        ),
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
        req(
            Method::PUT,
            "/crew-leads/cl-aria",
            Some(json!({
                "actor_id": ARIA,
                "new_lead": {"id": "cl-aria2", "name": "Aria 2"}
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, body) = send(&app, req(Method::GET, "/crew-leads", None)).await;
    let ids: Vec<&str> = body
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["id"].as_str().unwrap())
        .collect();
    assert!(ids.contains(&"cl-aria2"));
    assert!(!ids.contains(&"cl-aria"));
}

// ---------- passengers -------------------------------------------------

#[tokio::test]
async fn list_passengers_returns_three_seeded() {
    let app = app();
    let (status, body) = send(&app, req(Method::GET, "/passengers", None)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 3);
}

#[tokio::test]
async fn create_passenger_returns_201() {
    let app = app();
    let (status, body) = send(
        &app,
        req(
            Method::POST,
            "/passengers",
            Some(json!({
                "actor_id": ARIA,
                "id": "ps-new",
                "name": "New",
                "tier": "Gold"
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["id"], "ps-new");
    assert_eq!(body["tier"], "Gold");
}

#[tokio::test]
async fn create_passenger_rejects_unknown_field() {
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
                "tier": "Silver",
                "extra": "nope"
            })),
        ),
    )
    .await;
    // serde deny_unknown_fields => 422 from axum's default rejection.
    assert!(status.is_client_error());
}

#[tokio::test]
async fn get_passenger_returns_seeded_record() {
    let app = app();
    let (status, body) = send(&app, req(Method::GET, "/passengers/ps-001", None)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["id"], "ps-001");
    assert_eq!(body["tier"], "Silver");
}

#[tokio::test]
async fn get_passenger_returns_404_for_unknown() {
    let app = app();
    let (status, body) = send(&app, req(Method::GET, "/passengers/ps-zzz", None)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], "PassengerNotFound");
}

#[tokio::test]
async fn change_passenger_tier_returns_204_and_persists() {
    let app = app();
    let (status, _) = send(
        &app,
        req(
            Method::PATCH,
            "/passengers/ps-001/tier",
            Some(json!({"actor_id": ARIA, "tier": "Platinum"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, body) = send(&app, req(Method::GET, "/passengers/ps-001", None)).await;
    assert_eq!(body["tier"], "Platinum");
}

#[tokio::test]
async fn soft_delete_passenger_drops_from_list_but_get_still_finds_it() {
    let app = app();
    let (status, _) = send(
        &app,
        req(
            Method::DELETE,
            "/passengers/ps-001",
            Some(json!({"actor_id": ARIA})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (_, list) = send(&app, req(Method::GET, "/passengers", None)).await;
    let ids: Vec<&str> = list
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["id"].as_str().unwrap())
        .collect();
    assert!(!ids.contains(&"ps-001"));

    let (status, body) = send(&app, req(Method::GET, "/passengers/ps-001", None)).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["deleted_at"].is_number());
}

// ---------- resources --------------------------------------------------

#[tokio::test]
async fn create_resource_returns_201() {
    let app = app();
    let (status, body) = send(
        &app,
        req(
            Method::POST,
            "/resources",
            Some(json!({
                "actor_id": ARIA,
                "id": "res-new",
                "name": "New",
                "category": "test",
                "min_tier": "Gold"
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["id"], "res-new");
    assert_eq!(body["min_tier"], "Gold");
}

#[tokio::test]
async fn list_accessible_resources_returns_only_silver_for_silver_actor() {
    let app = app();
    let (status, body) = send(
        &app,
        req(Method::GET, "/resources/accessible?tier=Silver", None),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let arr = body.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["id"], "res-lounge");
}

#[tokio::test]
async fn list_accessible_resources_returns_all_for_platinum_actor() {
    let app = app();
    let (_, body) = send(
        &app,
        req(Method::GET, "/resources/accessible?tier=Platinum", None),
    )
    .await;
    assert_eq!(body.as_array().unwrap().len(), 3);
}

#[tokio::test]
async fn change_resource_min_tier_persists() {
    let app = app();
    let (status, _) = send(
        &app,
        req(
            Method::PATCH,
            "/resources/res-lounge/min-tier",
            Some(json!({"actor_id": ARIA, "tier": "Platinum"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, body) = send(&app, req(Method::GET, "/resources/res-lounge", None)).await;
    assert_eq!(body["min_tier"], "Platinum");
}

#[tokio::test]
async fn soft_delete_resource_returns_204() {
    let app = app();
    let (status, _) = send(
        &app,
        req(
            Method::DELETE,
            "/resources/res-lounge",
            Some(json!({"actor_id": ARIA})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

// ---------- access -----------------------------------------------------

#[tokio::test]
async fn access_allowed_returns_event() {
    let app = app();
    let (status, body) = send(
        &app,
        req(
            Method::POST,
            "/access",
            Some(json!({"passenger_id": "ps-001", "resource_id": "res-lounge"})),
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
        req(
            Method::POST,
            "/access",
            Some(json!({"passenger_id": "ps-001", "resource_id": "res-bridge"})),
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

// ---------- audit + reports -------------------------------------------

#[tokio::test]
async fn audit_endpoint_lists_bootstrap_and_seed_events() {
    let app = app();
    let (status, body) = send(&app, req(Method::GET, "/audit", None)).await;
    assert_eq!(status, StatusCode::OK);
    let arr = body.as_array().unwrap();
    // 1 bootstrap + 3 passengers + 3 resources = 7
    assert_eq!(arr.len(), 7);
    assert_eq!(arr[0]["action"], "CrewLeadBootstrapped");
}

#[tokio::test]
async fn report_by_tier_returns_all_three_tiers_in_order() {
    let app = app();
    let (status, body) = send(&app, req(Method::GET, "/reports/by-tier", None)).await;
    assert_eq!(status, StatusCode::OK);
    let arr = body.as_array().unwrap();
    let tiers: Vec<&str> = arr.iter().map(|r| r["tier"].as_str().unwrap()).collect();
    assert_eq!(tiers, vec!["Silver", "Gold", "Platinum"]);
}

#[tokio::test]
async fn report_top_resources_respects_n_query() {
    let app = app();
    // First: log a Gold passenger using lounge twice and spa once.
    for _ in 0..2 {
        let _ = send(
            &app,
            req(
                Method::POST,
                "/access",
                Some(json!({"passenger_id": "ps-002", "resource_id": "res-lounge"})),
            ),
        )
        .await;
    }
    let _ = send(
        &app,
        req(
            Method::POST,
            "/access",
            Some(json!({"passenger_id": "ps-002", "resource_id": "res-spa"})),
        ),
    )
    .await;
    let (status, body) = send(&app, req(Method::GET, "/reports/top-resources?n=1", None)).await;
    assert_eq!(status, StatusCode::OK);
    let arr = body.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["resource_id"], "res-lounge");
    assert_eq!(arr[0]["allowed_count"], 2);
}

#[tokio::test]
async fn report_personal_history_filters_by_passenger() {
    let app = app();
    let _ = send(
        &app,
        req(
            Method::POST,
            "/access",
            Some(json!({"passenger_id": "ps-002", "resource_id": "res-lounge"})),
        ),
    )
    .await;
    let _ = send(
        &app,
        req(
            Method::POST,
            "/access",
            Some(json!({"passenger_id": "ps-003", "resource_id": "res-spa"})),
        ),
    )
    .await;
    let (_, body) = send(&app, req(Method::GET, "/reports/history/ps-002", None)).await;
    let arr = body.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["passenger_id"], "ps-002");
}

// ---------- reset ------------------------------------------------------

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

    // Reset (auth required).
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
