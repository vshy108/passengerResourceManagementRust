// HTTP integration tests covering features added in the code-review sprint:
//   #3 - Optimistic concurrency (If-Match / ETag)
//   #4 - Soft-delete querying (?include_deleted=true)
//   #7 - Audit hash chain (GET /audit/verify)
//   #9 - API versioning (/v1/* dual-routing)
//
// Each test is hermetic: `app()` builds a fresh in-memory world.
#![cfg(feature = "http")]

mod http_common;

use axum::http::{Method, StatusCode};
use serde_json::json;

use http_common::{
    ARIA, CL_TOKEN, PS_TOKEN, app, auth_req, auth_req_if_match, req, send, send_full,
};

// ── #9 API versioning ────────────────────────────────────────────────────────

#[tokio::test]
async fn v1_passengers_route_mirrors_root() {
    let app = app();
    let (status, body) = send(&app, req(Method::GET, "/v1/passengers", None)).await;
    assert_eq!(status, StatusCode::OK);
    // Same three seeded passengers as /passengers
    assert_eq!(body.as_array().unwrap().len(), 3);
}

#[tokio::test]
async fn v1_resources_route_mirrors_root() {
    let app = app();
    let (status, body) = send(&app, req(Method::GET, "/v1/resources", None)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 3);
}

#[tokio::test]
async fn v1_audit_route_mirrors_root() {
    let app = app();
    let (status, body) = send(&app, req(Method::GET, "/v1/audit", None)).await;
    assert_eq!(status, StatusCode::OK);
    // Seeded world emits admin events during bootstrap
    assert!(body.as_array().unwrap().len() >= 3);
}

// ── #3 ETag on GET ───────────────────────────────────────────────────────────

#[tokio::test]
async fn get_passenger_returns_etag_header() {
    let app = app();
    let (status, _body, headers) =
        send_full(&app, req(Method::GET, "/passengers/ps-001", None)).await;
    assert_eq!(status, StatusCode::OK);
    // ETag must be present and contain the version number wrapped in quotes
    let etag = headers.get("etag").expect("ETag header missing");
    let etag_str = etag.to_str().unwrap();
    assert!(
        etag_str.starts_with('"') && etag_str.ends_with('"'),
        "ETag={etag_str}"
    );
}

#[tokio::test]
async fn get_resource_returns_etag_header() {
    let app = app();
    let (status, _body, headers) =
        send_full(&app, req(Method::GET, "/resources/res-lounge", None)).await;
    assert_eq!(status, StatusCode::OK);
    let etag = headers.get("etag").expect("ETag header missing");
    let etag_str = etag.to_str().unwrap();
    assert!(
        etag_str.starts_with('"') && etag_str.ends_with('"'),
        "ETag={etag_str}"
    );
}

#[tokio::test]
async fn get_passenger_version_field_is_zero_initially() {
    let app = app();
    let (status, body) = send(&app, req(Method::GET, "/passengers/ps-001", None)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["version"], 0);
}

#[tokio::test]
async fn get_resource_version_field_is_zero_initially() {
    let app = app();
    let (status, body) = send(&app, req(Method::GET, "/resources/res-lounge", None)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["version"], 0);
}

// ── #3 If-Match version check on PATCH passenger ─────────────────────────────

#[tokio::test]
async fn change_passenger_tier_with_correct_if_match_succeeds() {
    let app = app();
    // Version starts at 0; If-Match: "0" should pass.
    let (status, _) = send(
        &app,
        auth_req_if_match(
            Method::PATCH,
            "/passengers/ps-001/tier",
            CL_TOKEN,
            0,
            Some(json!({"tier": "Gold"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn change_passenger_tier_with_stale_if_match_returns_412() {
    let app = app();
    // Version is 0; If-Match: "99" should fail with 412.
    let (status, body) = send(
        &app,
        auth_req_if_match(
            Method::PATCH,
            "/passengers/ps-001/tier",
            CL_TOKEN,
            99,
            Some(json!({"tier": "Gold"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::PRECONDITION_FAILED);
    assert_eq!(body["code"], "VersionConflict");
}

#[tokio::test]
async fn change_passenger_tier_without_if_match_succeeds_unconditionally() {
    let app = app();
    // No If-Match header → optimistic check is skipped, mutates freely.
    let (status, _) = send(
        &app,
        auth_req(
            Method::PATCH,
            "/passengers/ps-001/tier",
            CL_TOKEN,
            Some(json!({"tier": "Diamond"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn version_increments_after_tier_change() {
    let app = app();
    send(
        &app,
        auth_req(
            Method::PATCH,
            "/passengers/ps-001/tier",
            CL_TOKEN,
            Some(json!({"tier": "Gold"})),
        ),
    )
    .await;
    let (_, body) = send(&app, req(Method::GET, "/passengers/ps-001", None)).await;
    assert_eq!(body["version"], 1);
}

// ── #3 If-Match on DELETE passenger ──────────────────────────────────────────

#[tokio::test]
async fn delete_passenger_with_stale_if_match_returns_412() {
    let app = app();
    let (status, body) = send(
        &app,
        auth_req_if_match(Method::DELETE, "/passengers/ps-001", CL_TOKEN, 42, None),
    )
    .await;
    assert_eq!(status, StatusCode::PRECONDITION_FAILED);
    assert_eq!(body["code"], "VersionConflict");
}

#[tokio::test]
async fn delete_passenger_with_correct_if_match_succeeds() {
    let app = app();
    let (status, _) = send(
        &app,
        auth_req_if_match(Method::DELETE, "/passengers/ps-001", CL_TOKEN, 0, None),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

// ── #3 If-Match on PATCH/DELETE resource ─────────────────────────────────────

#[tokio::test]
async fn change_resource_min_tier_with_stale_if_match_returns_412() {
    let app = app();
    let (status, body) = send(
        &app,
        auth_req_if_match(
            Method::PATCH,
            "/resources/res-lounge/min-tier",
            CL_TOKEN,
            42,
            Some(json!({"tier": "Gold"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::PRECONDITION_FAILED);
    assert_eq!(body["code"], "VersionConflict");
}

#[tokio::test]
async fn change_resource_min_tier_with_correct_if_match_succeeds() {
    let app = app();
    let (status, _) = send(
        &app,
        auth_req_if_match(
            Method::PATCH,
            "/resources/res-lounge/min-tier",
            CL_TOKEN,
            0,
            Some(json!({"tier": "Gold"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn delete_resource_with_stale_if_match_returns_412() {
    let app = app();
    let (status, body) = send(
        &app,
        auth_req_if_match(Method::DELETE, "/resources/res-lounge", CL_TOKEN, 99, None),
    )
    .await;
    assert_eq!(status, StatusCode::PRECONDITION_FAILED);
    assert_eq!(body["code"], "VersionConflict");
}

#[tokio::test]
async fn delete_resource_with_correct_if_match_succeeds() {
    let app = app();
    let (status, _) = send(
        &app,
        auth_req_if_match(Method::DELETE, "/resources/res-lounge", CL_TOKEN, 0, None),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

// ── #4 Soft-delete querying ───────────────────────────────────────────────────

#[tokio::test]
async fn list_passengers_include_deleted_shows_deleted_records() {
    let app = app();
    // Delete one passenger.
    send(
        &app,
        auth_req(Method::DELETE, "/passengers/ps-001", CL_TOKEN, None),
    )
    .await;
    // Without flag: only 2 active.
    let (_, active) = send(&app, req(Method::GET, "/passengers", None)).await;
    assert_eq!(active.as_array().unwrap().len(), 2);
    // With flag: 3 total (2 active + 1 deleted).
    let (status, all) = send(
        &app,
        req(Method::GET, "/passengers?include_deleted=true", None),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(all.as_array().unwrap().len(), 3);
    // The deleted record should appear with a non-null deleted_at.
    let deleted_count = all
        .as_array()
        .unwrap()
        .iter()
        .filter(|p| !p["deleted_at"].is_null())
        .count();
    assert_eq!(deleted_count, 1);
}

#[tokio::test]
async fn list_resources_include_deleted_shows_deleted_records() {
    let app = app();
    send(
        &app,
        auth_req(Method::DELETE, "/resources/res-lounge", CL_TOKEN, None),
    )
    .await;
    let (_, active) = send(&app, req(Method::GET, "/resources", None)).await;
    assert_eq!(active.as_array().unwrap().len(), 2);

    let (status, all) = send(
        &app,
        req(Method::GET, "/resources?include_deleted=true", None),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(all.as_array().unwrap().len(), 3);
}

// ── #7 Audit verify ───────────────────────────────────────────────────────────

#[tokio::test]
async fn audit_verify_returns_valid_for_fresh_world() {
    let app = app();
    let (status, body) = send(&app, req(Method::GET, "/audit/verify", None)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["valid"], true);
    assert!(body["length"].as_u64().unwrap() >= 3);
    assert!(body["broken_at"].is_null());
}

#[tokio::test]
async fn audit_verify_via_v1_also_works() {
    let app = app();
    let (status, body) = send(&app, req(Method::GET, "/v1/audit/verify", None)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["valid"], true);
}

/// Cover the `broken_at` branch in `verify_audit_chain` (http.rs lines 1226-1227).
///
/// Strategy: clone the `InMemoryAdminEventSink` Arc *before* moving the `World`
/// into `AppState`. Both handles share the same underlying `Arc<Mutex<…>>`, so
/// corrupting a hash through the clone is immediately visible to the handler.
#[tokio::test]
async fn audit_verify_detects_tampered_hash() {
    use std::collections::HashMap;

    use passenger_resource_management::infrastructure::in_memory_admin_event_sink::InMemoryAdminEventSink;
    use passenger_resource_management::interface::composition_root::{AuditSink, build_demo_world};
    use passenger_resource_management::interface::http::{AppState, CorsOrigins, router_with};

    let world = build_demo_world().expect("bootstrap");

    // Clone the sink *before* consuming `world`. Arc clone is a pointer bump —
    // both handles share the same backing store.
    let sink_clone: InMemoryAdminEventSink = match &world.audit_sink {
        AuditSink::InMemory(s) => s.clone(),
        AuditSink::Sqlite(_) => panic!("expected InMemory audit sink in demo world"),
    };

    let api_keys: HashMap<String, String> = [(CL_TOKEN.to_owned(), ARIA.to_owned())].into();
    let state = AppState::new(world, api_keys);
    let app = router_with(state, CorsOrigins::Any, false, false, 10, 50);

    // Sanity-check: chain is valid before tampering.
    let (_, body) = send(&app, req(Method::GET, "/audit/verify", None)).await;
    assert_eq!(
        body["valid"], true,
        "chain should be valid before tampering"
    );
    let chain_len = body["length"].as_u64().unwrap();
    assert!(chain_len >= 1);

    // Corrupt the stored hash of the first event so the verifier disagrees.
    // FIX: corrupt_hash_at is a #[cfg(test)]-only method on InMemoryAdminEventSink.
    // It writes directly to the shared Arc<Mutex<Vec<String>>>, so the next
    // call to verify_audit_chain (which reads via snapshot_with_hashes) sees
    // the corrupted value immediately.
    sink_clone.corrupt_hash_at(0, "deadbeefdeadbeef");

    // Chain must now be reported as invalid.
    let (status, body) = send(&app, req(Method::GET, "/audit/verify", None)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body["valid"], false,
        "chain should be invalid after tampering"
    );
    assert_eq!(
        body["broken_at"].as_u64(),
        Some(0),
        "broken_at should point to the first corrupted event"
    );
}

#[tokio::test]
async fn admin_events_include_event_hash_field() {
    let app = app();
    let (status, body) = send(&app, req(Method::GET, "/audit", None)).await;
    assert_eq!(status, StatusCode::OK);
    let events = body.as_array().unwrap();
    assert!(!events.is_empty());
    // Every event should have an event_hash field (non-empty for in-memory world).
    for ev in events {
        assert!(ev["event_hash"].is_string(), "missing event_hash: {ev}");
        assert!(!ev["event_hash"].as_str().unwrap().is_empty());
    }
}

// ── ErrorCode typed responses ─────────────────────────────────────────────────

#[tokio::test]
async fn passenger_not_found_returns_typed_error_code() {
    let app = app();
    let (_, body) = send(&app, req(Method::GET, "/passengers/nope", None)).await;
    assert_eq!(body["code"], "PassengerNotFound");
}

#[tokio::test]
async fn resource_not_found_returns_typed_error_code() {
    let app = app();
    let (_, body) = send(&app, req(Method::GET, "/resources/nope", None)).await;
    assert_eq!(body["code"], "ResourceNotFound");
}

#[tokio::test]
async fn unauthorized_request_returns_typed_error_code() {
    let app = app();
    let (status, body) = send(
        &app,
        auth_req(
            Method::POST,
            "/passengers",
            "bad-token",
            Some(json!({"id":"x","name":"x","tier":"Silver"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["code"], "Unauthorized");
}

#[tokio::test]
async fn access_endpoint_rejects_invalid_resource_id_with_400() {
    let app = app();
    // Empty resource_id should hit the bad_request() path in use_resource.
    let (status, body) = send(
        &app,
        auth_req(
            Method::POST,
            "/access",
            PS_TOKEN,
            Some(json!({"resource_id": ""})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "InvalidInput");
}

// ── health/ready ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn health_ready_returns_200_for_in_memory_world() {
    let app = app();
    let (status, body) = send(&app, req(Method::GET, "/health/ready", None)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ready");
    assert_eq!(body["crew_leads"], 3);
}
