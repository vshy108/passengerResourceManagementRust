// HTTP integration tests for /passengers endpoints (CRUD + tier
// updates). Of note: `create_passenger_rejects_unknown_field` proves
// the `#[serde(deny_unknown_fields)]` on the request DTO actually
// fires — important boundary validation.
// See `tests/http_health.rs` for harness details.
#![cfg(feature = "http")]

mod http_common;

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
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

#[tokio::test]
async fn create_passenger_missing_token_returns_401() {
    let app = app();
    // No Authorization header — the AuthActor extractor should reject.
    let (status, body) = send(
        &app,
        req(
            Method::POST,
            "/passengers",
            Some(json!({"id": "ps-new", "name": "New", "tier": "Silver"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["code"], "Unauthorized");
}

#[tokio::test]
async fn create_passenger_empty_id_returns_400() {
    let app = app();
    // Empty id field should be rejected at the interface boundary.
    let (status, body) = send(
        &app,
        auth_req(
            Method::POST,
            "/passengers",
            CL_TOKEN,
            Some(json!({"id": "", "name": "Valid Name", "tier": "Silver"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "InvalidInput");
}

#[tokio::test]
async fn create_passenger_oversized_id_returns_400() {
    let app = app();
    // FIX: `CreatePassengerReq::validate()` rejects id longer than 255 chars.
    // This exercises the `id.len() > 255` return branch (dto.rs line 168).
    let long_id = "p".repeat(256);
    let (status, body) = send(
        &app,
        auth_req(
            Method::POST,
            "/passengers",
            CL_TOKEN,
            Some(json!({"id": long_id, "name": "Valid Name", "tier": "Silver"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "InvalidInput");
}

#[tokio::test]
async fn create_passenger_empty_name_returns_400() {
    let app = app();
    // FIX: `CreatePassengerReq::validate()` rejects an empty name.
    // This exercises the `name.is_empty()` return branch (dto.rs line 171).
    let (status, body) = send(
        &app,
        auth_req(
            Method::POST,
            "/passengers",
            CL_TOKEN,
            Some(json!({"id": "ps-noname", "name": "", "tier": "Silver"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "InvalidInput");
}

#[tokio::test]
async fn create_passenger_oversized_name_returns_400() {
    let app = app();
    // Name > 255 chars should be rejected at the interface boundary.
    let long_name = "x".repeat(256);
    let (status, body) = send(
        &app,
        auth_req(
            Method::POST,
            "/passengers",
            CL_TOKEN,
            Some(json!({"id": "ps-long", "name": long_name, "tier": "Silver"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "InvalidInput");
}

#[tokio::test]
async fn list_passengers_pagination_offset_and_limit() {
    let app = app();
    // Seeded world has 3 passengers. offset=1&limit=1 should return exactly 1.
    let (status, body) = send(&app, req(Method::GET, "/passengers?offset=1&limit=1", None)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn create_passenger_idempotency_key_deduplicates_retry() {
    // A single app instance shares AppState (Arc<RwLock<World>> + idempotency cache)
    // across cloned Router calls — so the second request sees the cached response.
    let app = app();
    let payload = json!({"id": "ps-idem", "name": "Idem Test", "tier": "Silver"});

    // Helper that builds a POST /passengers request with an Idempotency-Key header.
    let make_req = || {
        Request::builder()
            .method(Method::POST)
            .uri("/passengers")
            .header("authorization", format!("Bearer {CL_TOKEN}"))
            .header("content-type", "application/json")
            .header("idempotency-key", "idem-key-ps-idem")
            .body(Body::from(serde_json::to_vec(&payload).expect("json")))
            .expect("request")
    };

    // First call — creates the passenger and caches the response.
    let (s1, b1) = send(&app, make_req()).await;
    assert_eq!(s1, StatusCode::CREATED);
    assert_eq!(b1["id"], "ps-idem");

    // Second call with the SAME key — must return the cached 201, not 409 Conflict.
    // This proves the idempotency cache is checked before domain logic runs.
    let (s2, b2) = send(&app, make_req()).await;
    assert_eq!(
        s2,
        StatusCode::CREATED,
        "retry with same key must return 201, not 409"
    );
    assert_eq!(
        b1, b2,
        "retry response body must be identical to first response"
    );

    // Confirm only ONE passenger was created (domain logic ran exactly once).
    let (_, list) = send(&app, req(Method::GET, "/passengers", None)).await;
    assert_eq!(
        list.as_array().unwrap().len(),
        4, // 3 seeded + 1 created via idempotent POST
        "duplicate domain execution would yield 5 rows"
    );
}

#[tokio::test]
async fn create_passenger_different_idempotency_key_is_independent() {
    // Two requests with DIFFERENT idempotency keys must be treated as two
    // separate operations — the second should get 409 (duplicate passenger id).
    let app = app();
    let payload = json!({"id": "ps-idem2", "name": "Idem Test 2", "tier": "Gold"});

    let make_req = |key: &str| {
        let key = key.to_owned();
        Request::builder()
            .method(Method::POST)
            .uri("/passengers")
            .header("authorization", format!("Bearer {CL_TOKEN}"))
            .header("content-type", "application/json")
            .header("idempotency-key", key)
            .body(Body::from(serde_json::to_vec(&payload).expect("json")))
            .expect("request")
    };

    let (s1, _) = send(&app, make_req("key-a")).await;
    assert_eq!(s1, StatusCode::CREATED);

    // Different key → cache miss → domain logic runs → duplicate passenger → 409.
    let (s2, body) = send(&app, make_req("key-b")).await;
    assert_eq!(s2, StatusCode::CONFLICT);
    assert_eq!(body["code"], "PassengerAlreadyExists");
}
