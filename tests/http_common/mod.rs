//! Shared helpers for the per-aggregate `tests/http_*.rs` files.
//!
//! Each test binary does `mod http_common;` to pull these in; Cargo
//! does not pick up files inside subdirectories of `tests/` as their
//! own test binaries, so this stays a private helper module.

// HOW INTEGRATION TESTS WORK IN RUST:
// - Every `.rs` file directly in `tests/` becomes its own *separate*
//   test binary (linked against the library).
// - Files in `tests/http_common/` are NOT auto-discovered, which is why
//   each test file does `mod http_common;` to include this as a private
//   module instead of getting a duplicate test binary.
// - Integration tests can only see `pub` items from the crate (just
//   like a real downstream consumer would).

// File-level attributes (note the `!`):
// - `#![cfg(feature = "http")]` — the entire file is compiled only when
//   the `http` feature is on. Without it, all the http_* tests vanish.
// - `#![allow(dead_code)]` — each individual test binary uses only some
//   of these helpers, so unused ones would otherwise warn.
#![cfg(feature = "http")]
#![allow(dead_code)] // each test binary uses a subset of these helpers.

use std::sync::{Arc, Mutex};

use axum::{
    Router,
    body::Body,
    http::{Method, Request, StatusCode},
};
// `http_body_util::BodyExt` is an extension trait that adds `.collect()`
// to response bodies. Importing it brings the method into scope.
use http_body_util::BodyExt;
// `serde_json::Value` is a dynamically-typed JSON tree (any JSON shape).
// Useful in tests where we don't want to define a struct per response.
use serde_json::Value;
// `tower::ServiceExt` provides `.oneshot(request)` — sends a single
// request through a tower Service (axum's Router IS one) and returns
// the response, without spinning up a real HTTP server.
use tower::ServiceExt;

use passenger_resource_management::interface::composition_root::build_demo_world;
use passenger_resource_management::interface::http::{router_with, CorsOrigins};

// `pub const` — a compile-time constant, inlined at every use site.
// Convention: SCREAMING_SNAKE_CASE.
pub const ARIA: &str = "cl-aria";

/// Build a fresh app with the demo world. Each test gets its OWN world
/// so tests cannot interfere with each other (no shared global state).
pub fn app() -> Router {
    let world = build_demo_world().expect("bootstrap");
    let state = Arc::new(Mutex::new(world));
    // FIX: router() now defaults to enable_reset=false; tests need /reset
    // to exercise the endpoint, so we call router_with explicitly.
    router_with(state, CorsOrigins::Any, true)
}

/// Send a request through the router in-process and return (status, body).
// `async fn` because axum/tower futures are async. Tests that call this
// must be `#[tokio::test]` (see individual test files).
pub async fn send(app: &Router, req: Request<Body>) -> (StatusCode, Value) {
    // `.clone()` because `oneshot` consumes `self`. Router is cheaply
    // cloneable (Arc internally).
    let res = app.clone().oneshot(req).await.expect("response");
    let status = res.status();
    // Drain the streaming body into bytes. `.collect().await` aggregates
    // all chunks; `.to_bytes()` flattens them into a single `Bytes`.
    let bytes = res.into_body().collect().await.expect("body").to_bytes();
    // Empty body (e.g. 204 No Content) -> JSON null. Otherwise try to
    // parse as JSON; if parsing fails, fall back to the raw text — this
    // way the helper works for both JSON endpoints and `/health` which
    // returns plain text.
    let body = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes)
            // `String::from_utf8_lossy` replaces invalid UTF-8 with `�`
            // instead of panicking — safe even on malformed responses.
            .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into_owned()))
    };
    (status, body)
}

/// Build a `Request<Body>` from a method, path, and optional JSON body.
pub fn req(method: Method, path: &str, body: Option<Value>) -> Request<Body> {
    // Builder pattern: `Request::builder()` returns a builder we
    // gradually configure, then call `.body(...)` to finalise.
    let mut b = Request::builder().method(method).uri(path);
    let body = match body {
        Some(v) => {
            // Reassign `b` because each builder method takes `self` by
            // value and returns a new builder (consuming style).
            b = b.header("content-type", "application/json");
            // Serialise the JSON Value to bytes for the request body.
            Body::from(serde_json::to_vec(&v).expect("json"))
        }
        None => Body::empty(),
    };
    b.body(body).expect("request")
}
