//! Shared helpers for the per-aggregate `tests/http_*.rs` files.
//!
//! Each test binary does `mod http_common;` to pull these in; Cargo
//! does not pick up files inside subdirectories of `tests/` as their
//! own test binaries, so this stays a private helper module.

#![cfg(feature = "http")]
#![allow(dead_code)] // each test binary uses a subset of these helpers.

use std::sync::{Arc, Mutex};

use axum::{
    Router,
    body::Body,
    http::{Method, Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

use passenger_resource_management::interface::composition_root::build_demo_world;
use passenger_resource_management::interface::http::router;

pub const ARIA: &str = "cl-aria";

pub fn app() -> Router {
    let world = build_demo_world().expect("bootstrap");
    let state = Arc::new(Mutex::new(world));
    router(state)
}

pub async fn send(app: &Router, req: Request<Body>) -> (StatusCode, Value) {
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

pub fn req(method: Method, path: &str, body: Option<Value>) -> Request<Body> {
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
