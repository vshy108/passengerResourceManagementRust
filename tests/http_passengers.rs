// HTTP integration tests for /passengers endpoints (CRUD + tier
// updates). Of note: `create_passenger_rejects_unknown_field` proves
// the `#[serde(deny_unknown_fields)]` on the request DTO actually
// fires — important boundary validation.
// See `tests/http_health.rs` for harness details.
#![cfg(feature = "http")]

mod http_common;

use axum::http::{Method, StatusCode};
use serde_json::json;

use http_common::{CL_TOKEN, app, auth_req, req, send};

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
        auth_req(
            Method::POST,
            "/passengers",
            CL_TOKEN,
            Some(json!({
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
        auth_req(
            Method::POST,
            "/passengers",
            CL_TOKEN,
            Some(json!({
                "id": "ps-x",
                "name": "X",
                "tier": "Silver",
                "extra": "nope"
            })),
        ),
    )
    .await;
    assert!(status.is_client_error());
}

#[tokio::test]
async fn create_passenger_duplicate_id_returns_409() {
    let app = app();
    let (status, body) = send(
        &app,
        auth_req(
            Method::POST,
            "/passengers",
            CL_TOKEN,
            Some(json!({
                "id": "ps-001",
                "name": "Dup",
                "tier": "Silver"
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "PassengerAlreadyExists");
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
        auth_req(
            Method::PATCH,
            "/passengers/ps-001/tier",
            CL_TOKEN,
            Some(json!({"tier": "Platinum"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, body) = send(&app, req(Method::GET, "/passengers/ps-001", None)).await;
    assert_eq!(body["tier"], "Platinum");
}

#[tokio::test]
async fn change_tier_unknown_passenger_returns_404() {
    let app = app();
    let (status, body) = send(
        &app,
        auth_req(
            Method::PATCH,
            "/passengers/ps-zzz/tier",
            CL_TOKEN,
            Some(json!({"tier": "Gold"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], "PassengerNotFound");
}

#[tokio::test]
async fn soft_delete_passenger_drops_from_list_but_get_still_finds_it() {
    let app = app();
    let (status, _) = send(
        &app,
        auth_req(Method::DELETE, "/passengers/ps-001", CL_TOKEN, None),
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

#[tokio::test]
async fn delete_unknown_passenger_returns_404() {
    let app = app();
    let (status, body) = send(
        &app,
        auth_req(Method::DELETE, "/passengers/ps-zzz", CL_TOKEN, None),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], "PassengerNotFound");
}
