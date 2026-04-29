// HTTP integration tests for /resources endpoints. Mirrors the
// passenger tests; verifies tier minimums, duplicates, and updates.
// See `tests/http_health.rs` for harness details.
#![cfg(feature = "http")]

mod http_common;

use axum::http::{Method, StatusCode};
use serde_json::json;

use http_common::{ARIA, app, req, send};

#[tokio::test]
async fn list_resources_returns_three_seeded() {
    let app = app();
    let (status, body) = send(&app, req(Method::GET, "/resources", None)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 3);
}

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
async fn create_resource_duplicate_id_returns_409() {
    let app = app();
    let (status, body) = send(
        &app,
        req(
            Method::POST,
            "/resources",
            Some(json!({
                "actor_id": ARIA,
                "id": "res-lounge",
                "name": "Dup",
                "category": "test",
                "min_tier": "Silver"
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "ResourceAlreadyExists");
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
async fn get_resource_returns_404_for_unknown() {
    let app = app();
    let (status, body) = send(&app, req(Method::GET, "/resources/res-zzz", None)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], "ResourceNotFound");
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
async fn change_min_tier_unknown_resource_returns_404() {
    let app = app();
    let (status, body) = send(
        &app,
        req(
            Method::PATCH,
            "/resources/res-zzz/min-tier",
            Some(json!({"actor_id": ARIA, "tier": "Gold"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], "ResourceNotFound");
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

#[tokio::test]
async fn delete_unknown_resource_returns_404() {
    let app = app();
    let (status, body) = send(
        &app,
        req(
            Method::DELETE,
            "/resources/res-zzz",
            Some(json!({"actor_id": ARIA})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], "ResourceNotFound");
}
