// HTTP integration tests for /resources endpoints. Mirrors the
// passenger tests; verifies tier minimums, duplicates, and updates.
// See `tests/http_health.rs` for harness details.
#![cfg(feature = "http")]

mod http_common;

use axum::http::{Method, StatusCode};
use serde_json::json;

use http_common::{CL_TOKEN, app, auth_req, req, send};

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
        auth_req(
            Method::POST,
            "/resources",
            CL_TOKEN,
            Some(json!({
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
        auth_req(
            Method::POST,
            "/resources",
            CL_TOKEN,
            Some(json!({
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
        auth_req(
            Method::PATCH,
            "/resources/res-lounge/min-tier",
            CL_TOKEN,
            Some(json!({"tier": "Platinum"})),
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
        auth_req(
            Method::PATCH,
            "/resources/res-zzz/min-tier",
            CL_TOKEN,
            Some(json!({"tier": "Gold"})),
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
        auth_req(Method::DELETE, "/resources/res-lounge", CL_TOKEN, None),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn delete_unknown_resource_returns_404() {
    let app = app();
    let (status, body) = send(
        &app,
        auth_req(Method::DELETE, "/resources/res-zzz", CL_TOKEN, None),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], "ResourceNotFound");
}

#[tokio::test]
async fn list_resources_pagination_offset_and_limit() {
    let app = app();
    // Seeded world has 3 resources. offset=1&limit=1 should return exactly 1.
    let (status, body) = send(
        &app,
        req(Method::GET, "/resources?offset=1&limit=1", None),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn create_resource_empty_id_returns_400() {
    let app = app();
    // FIX: `CreateResourceReq::validate()` rejects an empty `id`.
    // This exercises the `id.is_empty()` return branch (dto.rs line 224).
    let (status, body) = send(
        &app,
        auth_req(
            Method::POST,
            "/resources",
            CL_TOKEN,
            Some(json!({"id": "", "name": "Valid", "category": "lounge", "min_tier": "Silver"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "InvalidInput");
}

#[tokio::test]
async fn create_resource_oversized_id_returns_400() {
    let app = app();
    // FIX: `CreateResourceReq::validate()` rejects id longer than 255 chars.
    // This exercises the `id.len() > 255` return branch (dto.rs line 227).
    let long_id = "r".repeat(256);
    let (status, body) = send(
        &app,
        auth_req(
            Method::POST,
            "/resources",
            CL_TOKEN,
            Some(json!({"id": long_id, "name": "Valid", "category": "lounge", "min_tier": "Silver"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "InvalidInput");
}

#[tokio::test]
async fn create_resource_empty_name_returns_400() {
    let app = app();
    // FIX: `CreateResourceReq::validate()` rejects an empty name.
    // This exercises the `name.is_empty()` return branch (dto.rs line 230).
    let (status, body) = send(
        &app,
        auth_req(
            Method::POST,
            "/resources",
            CL_TOKEN,
            Some(json!({"id": "res-noname", "name": "", "category": "lounge", "min_tier": "Silver"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "InvalidInput");
}

#[tokio::test]
async fn create_resource_empty_category_returns_400() {
    let app = app();
    // FIX: `CreateResourceReq::validate()` rejects empty category.
    // This exercises the `category.is_empty()` branch (dto.rs line 233).
    let (status, body) = send(
        &app,
        auth_req(
            Method::POST,
            "/resources",
            CL_TOKEN,
            Some(json!({"id": "res-x", "name": "X", "category": "", "min_tier": "Silver"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "InvalidInput");
}

#[tokio::test]
async fn create_resource_oversized_name_returns_400() {
    let app = app();
    // FIX: `CreateResourceReq::validate()` rejects names longer than 255 chars.
    // This exercises the `name.len() > 255` branch (dto.rs line 227).
    let long_name = "a".repeat(256);
    let (status, body) = send(
        &app,
        auth_req(
            Method::POST,
            "/resources",
            CL_TOKEN,
            Some(json!({"id": "res-x", "name": long_name, "category": "lounge", "min_tier": "Silver"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "InvalidInput");
}

#[tokio::test]
async fn create_resource_oversized_category_returns_400() {
    let app = app();
    // FIX: `CreateResourceReq::validate()` rejects categories longer than 255 chars.
    // This exercises the `category.len() > 255` branch (dto.rs line 236).
    let long_cat = "c".repeat(256);
    let (status, body) = send(
        &app,
        auth_req(
            Method::POST,
            "/resources",
            CL_TOKEN,
            Some(json!({"id": "res-x", "name": "X", "category": long_cat, "min_tier": "Silver"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "InvalidInput");
}

#[tokio::test]
async fn create_resource_platinum_min_tier_accepted() {
    let app = app();
    // Exercises the `TierDto::Platinum => Tier::Platinum` branch (dto.rs line 53)
    // which is triggered when parsing a "Platinum" tier from a JSON request body.
    let (status, body) = send(
        &app,
        auth_req(
            Method::POST,
            "/resources",
            CL_TOKEN,
            Some(json!({"id": "res-plat", "name": "Plat Zone", "category": "lounge", "min_tier": "Platinum"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["min_tier"], "Platinum");
}

#[tokio::test]
async fn create_resource_diamond_min_tier_roundtrip() {
    let app = app();
    // Exercises the `Tier::Diamond => TierDto::Diamond` branch (dto.rs line 42)
    // which is triggered when converting a domain `Tier::Diamond` value into a DTO
    // for JSON serialisation (e.g. in the GET /resources response body).
    let (status, _) = send(
        &app,
        auth_req(
            Method::POST,
            "/resources",
            CL_TOKEN,
            Some(json!({"id": "res-dia", "name": "Diamond Lounge", "category": "lounge", "min_tier": "Diamond"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let (get_status, get_body) = send(&app, req(Method::GET, "/resources/res-dia", None)).await;
    assert_eq!(get_status, StatusCode::OK);
    // `min_tier` serialised as `TierDto::Diamond` must round-trip back to "Diamond".
    assert_eq!(get_body["min_tier"], "Diamond");
}
