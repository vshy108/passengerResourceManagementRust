// HTTP integration tests for /audit and /reports endpoints.
// Drives access requests then asserts on aggregated reporting output.
// See `tests/http_health.rs` for harness details.
#![cfg(feature = "http")]

mod http_common;

use axum::http::{Method, StatusCode};
use serde_json::json;

use http_common::{ARIA, app, req, send};

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

#[tokio::test]
async fn audit_log_serialises_every_admin_action_string() {
    let app = app();

    // PassengerCreated, PassengerTierChanged, PassengerDeleted
    let _ = send(
        &app,
        req(
            Method::POST,
            "/passengers",
            Some(json!({
                "actor_id": ARIA,
                "id": "ps-cov",
                "name": "C",
                "tier": "Silver"
            })),
        ),
    )
    .await;
    let _ = send(
        &app,
        req(
            Method::PATCH,
            "/passengers/ps-cov/tier",
            Some(json!({"actor_id": ARIA, "tier": "Gold"})),
        ),
    )
    .await;
    let _ = send(
        &app,
        req(
            Method::DELETE,
            "/passengers/ps-cov",
            Some(json!({"actor_id": ARIA})),
        ),
    )
    .await;

    // ResourceCreated, ResourceMinTierChanged, ResourceDeleted
    let _ = send(
        &app,
        req(
            Method::POST,
            "/resources",
            Some(json!({
                "actor_id": ARIA,
                "id": "res-cov",
                "name": "C",
                "category": "x",
                "min_tier": "Silver"
            })),
        ),
    )
    .await;
    let _ = send(
        &app,
        req(
            Method::PATCH,
            "/resources/res-cov/min-tier",
            Some(json!({"actor_id": ARIA, "tier": "Gold"})),
        ),
    )
    .await;
    let _ = send(
        &app,
        req(
            Method::DELETE,
            "/resources/res-cov",
            Some(json!({"actor_id": ARIA})),
        ),
    )
    .await;

    // CrewLeadReplaced
    let _ = send(
        &app,
        req(
            Method::PUT,
            "/crew-leads/cl-aria",
            Some(json!({
                "actor_id": ARIA,
                "new_lead": {"id": "cl-aria2", "name": "A2"}
            })),
        ),
    )
    .await;

    let (status, body) = send(&app, req(Method::GET, "/audit", None)).await;
    assert_eq!(status, StatusCode::OK);
    let actions: Vec<&str> = body
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["action"].as_str().unwrap())
        .collect();
    for expected in [
        "CrewLeadBootstrapped",
        "PassengerCreated",
        "PassengerTierChanged",
        "PassengerDeleted",
        "ResourceCreated",
        "ResourceMinTierChanged",
        "ResourceDeleted",
        "CrewLeadReplaced",
    ] {
        assert!(actions.contains(&expected), "missing {expected}");
    }
}
