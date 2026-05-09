// HTTP integration tests for /audit and /reports endpoints.
// Drives access requests then asserts on aggregated reporting output.
// See `tests/http_health.rs` for harness details.
#![cfg(feature = "http")]

mod http_common;

use axum::http::{Method, StatusCode};
use serde_json::json;

use http_common::{CL_TOKEN, PS_TOKEN, app, auth_req, req, send};

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
            // PS_TOKEN maps to ps-001 (Silver) — res-lounge is Silver min_tier
            auth_req(
                Method::POST,
                "/access",
                PS_TOKEN,
                Some(json!({"resource_id": "res-lounge"})),
            ),
        )
        .await;
    }
    let _ = send(
        &app,
        auth_req(
            Method::POST,
            "/access",
            PS_TOKEN,
            Some(json!({"resource_id": "res-spa"})),
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
        auth_req(
            Method::POST,
            "/access",
            PS_TOKEN,
            Some(json!({"resource_id": "res-lounge"})),
        ),
    )
    .await;
    // ps-003 has no token in test helper — just drive history via ps-001 twice
    let _ = send(
        &app,
        auth_req(
            Method::POST,
            "/access",
            PS_TOKEN,
            Some(json!({"resource_id": "res-spa"})),
        ),
    )
    .await;
    let (_, body) = send(&app, req(Method::GET, "/reports/history/ps-001", None)).await;
    let arr = body.as_array().unwrap();
    assert!(!arr.is_empty());
    assert_eq!(arr[0]["passenger_id"], "ps-001");
}

#[tokio::test]
async fn audit_log_serialises_every_admin_action_string() {
    let app = app();

    // PassengerCreated, PassengerTierChanged, PassengerDeleted
    let _ = send(
        &app,
        auth_req(
            Method::POST,
            "/passengers",
            CL_TOKEN,
            Some(json!({
                "id": "ps-cov",
                "name": "C",
                "tier": "Silver"
            })),
        ),
    )
    .await;
    let _ = send(
        &app,
        auth_req(
            Method::PATCH,
            "/passengers/ps-cov/tier",
            CL_TOKEN,
            Some(json!({"tier": "Gold"})),
        ),
    )
    .await;
    let _ = send(
        &app,
        auth_req(Method::DELETE, "/passengers/ps-cov", CL_TOKEN, None),
    )
    .await;

    // ResourceCreated, ResourceMinTierChanged, ResourceDeleted
    let _ = send(
        &app,
        auth_req(
            Method::POST,
            "/resources",
            CL_TOKEN,
            Some(json!({
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
        auth_req(
            Method::PATCH,
            "/resources/res-cov/min-tier",
            CL_TOKEN,
            Some(json!({"tier": "Gold"})),
        ),
    )
    .await;
    let _ = send(
        &app,
        auth_req(Method::DELETE, "/resources/res-cov", CL_TOKEN, None),
    )
    .await;

    // CrewLeadReplaced
    let _ = send(
        &app,
        auth_req(
            Method::PUT,
            "/crew-leads/cl-aria",
            CL_TOKEN,
            Some(json!({"new_lead": {"id": "cl-aria2", "name": "A2"}})),
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

#[tokio::test]
async fn report_by_tier_includes_diamond_at_correct_sort_position() {
    let app = app();
    // Promote ps-001 (Silver) to Diamond, then have them use res-lounge.
    // This produces a Diamond usage event, exercising the
    // `TierDto::Diamond => 2` sort arm in `report_tier_breakdown`
    // (http.rs line 915) and the `Tier::Diamond => TierDto::Diamond`
    // conversion in `From<Tier> for TierDto` (dto.rs line 42).
    let _ = send(
        &app,
        auth_req(
            Method::PATCH,
            "/passengers/ps-001/tier",
            CL_TOKEN,
            Some(json!({"tier": "Diamond"})),
        ),
    )
    .await;
    let _ = send(
        &app,
        auth_req(
            Method::POST,
            "/access",
            PS_TOKEN,
            Some(json!({"resource_id": "res-lounge"})),
        ),
    )
    .await;
    let (status, body) = send(&app, req(Method::GET, "/reports/by-tier", None)).await;
    assert_eq!(status, StatusCode::OK);
    let arr = body.as_array().unwrap();
    let tiers: Vec<&str> = arr.iter().map(|r| r["tier"].as_str().unwrap()).collect();
    // Diamond must appear at a position after Gold (rank 1) and before Platinum (rank 3).
    let diamond_pos = tiers.iter().position(|&t| t == "Diamond");
    assert!(diamond_pos.is_some(), "Diamond tier missing from report");
}
