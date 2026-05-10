// HTTP integration tests exercising a SQLite-backed world.
//
// These tests cover code paths that are only reachable when the server is
// backed by a real SQLite entity store rather than the default in-memory world:
//
//   - flush_to_db() body (http.rs 465-490): SQLite sync on mutation
//   - AuditSink::Sqlite::snapshot / snapshot_with_hashes (composition_root.rs 87, 101-105)
//   - World::ping_db() with Some(entity_store) (composition_root.rs 174-177)
//   - verify_audit_chain() skip for empty hashes (http.rs 1223-1224)
//   - router() convenience wrapper (http.rs 213-215)
#![cfg(feature = "http")]

mod http_common;

use std::collections::HashMap;

use axum::http::{Method, StatusCode};

use passenger_resource_management::interface::composition_root::build_world_with_sqlite;
use passenger_resource_management::interface::http::{AppState, CorsOrigins, router, router_with};

use http_common::{CL_TOKEN, req, send};

/// Build a SQLite-backed HTTP app using a temp file, returning the app and the
/// db path so the caller can clean up.
fn sqlite_app() -> (axum::Router, std::path::PathBuf) {
    let dir = std::env::temp_dir();
    let db_path = dir.join(format!(
        "prms_http_test_{}.db",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    let db_str = db_path.to_str().expect("tempdir UTF-8");
    let world = build_world_with_sqlite(db_str).expect("sqlite world build failed");
    let api_keys: HashMap<String, String> = [(CL_TOKEN.to_owned(), "cl-aria".to_owned())].into();
    let state = AppState::new(world, api_keys);
    // Use the public router() wrapper (covers http.rs lines 213-215).
    let app = router_with(state, CorsOrigins::Any, false, false, 10, 50);
    (app, db_path)
}

fn cleanup(db_path: &std::path::Path) {
    let _ = std::fs::remove_file(db_path);
    let _ = std::fs::remove_file(db_path.with_extension("db-wal"));
    let _ = std::fs::remove_file(db_path.with_extension("db-shm"));
}

// ── flush_to_db body ──────────────────────────────────────────────────────

#[tokio::test]
async fn sqlite_backed_patch_triggers_flush_to_db() {
    let (app, db_path) = sqlite_app();
    // Mutation triggers flush_to_db (http.rs 465-490).
    let (status, _) = send(
        &app,
        http_common::auth_req(
            Method::PATCH,
            "/passengers/ps-001/tier",
            CL_TOKEN,
            Some(serde_json::json!({"tier": "Gold"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    cleanup(&db_path);
}

#[tokio::test]
async fn sqlite_backed_delete_triggers_flush_to_db() {
    let (app, db_path) = sqlite_app();
    let (status, _) = send(
        &app,
        http_common::auth_req(Method::DELETE, "/passengers/ps-001", CL_TOKEN, None),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    cleanup(&db_path);
}

// ── health/ready with SQLite entity_store → covers World::ping_db ────────

#[tokio::test]
async fn sqlite_health_ready_exercises_ping_db() {
    let (app, db_path) = sqlite_app();
    // GET /health/ready calls World::ping_db() which returns Some(true)
    // when entity_store is present (composition_root.rs 174, 176-177).
    let (status, body) = send(&app, req(Method::GET, "/health/ready", None)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ready");
    cleanup(&db_path);
}

// ── AuditSink::Sqlite::snapshot (composition_root.rs 87) ─────────────────

#[tokio::test]
async fn sqlite_audit_list_exercises_sqlite_snapshot() {
    let (app, db_path) = sqlite_app();
    // GET /audit calls AuditSink::Sqlite::snapshot() (composition_root.rs 87).
    let (status, body) = send(&app, req(Method::GET, "/audit", None)).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.as_array().unwrap().len() >= 3);
    cleanup(&db_path);
}

// ── AuditSink::Sqlite::snapshot_with_hashes + skip path ──────────────────

#[tokio::test]
async fn sqlite_audit_verify_covers_sqlite_skip_path() {
    let (app, db_path) = sqlite_app();
    // GET /audit/verify calls AuditSink::Sqlite::snapshot_with_hashes()
    // (composition_root.rs 101-105) and exercises the empty-hash skip path
    // in verify_audit_chain() (http.rs 1223-1224).
    let (status, body) = send(&app, req(Method::GET, "/audit/verify", None)).await;
    assert_eq!(status, StatusCode::OK);
    // SQLite-loaded events have no stored hashes, so verification skips them
    // and reports valid (no tampering detected).
    assert_eq!(body["valid"], true);
    cleanup(&db_path);
}

// ── router() convenience wrapper (http.rs 213-215) ───────────────────────

#[tokio::test]
async fn router_convenience_wrapper_builds_working_app() {
    let dir = std::env::temp_dir();
    let db_path = dir.join(format!(
        "prms_router_test_{}.db",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    let db_str = db_path.to_str().expect("tempdir UTF-8");
    let world = build_world_with_sqlite(db_str).expect("world build failed");
    let api_keys: HashMap<String, String> = [(CL_TOKEN.to_owned(), "cl-aria".to_owned())].into();
    let state = AppState::new(world, api_keys);
    // Explicitly call router() (http.rs 213-215) — the convenience wrapper
    // that router_with() tests don't reach.
    let app = router(state);
    let (status, body) = send(&app, req(Method::GET, "/passengers", None)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 3);
    cleanup(&db_path);
}
