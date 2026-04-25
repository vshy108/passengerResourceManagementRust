#![cfg(feature = "http")]

mod http_common;

use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

use http_common::{ARIA, app, req, send};

#[tokio::test]
async fn health_returns_ok() {
    let app = app();
    let (status, body) = send(&app, req(Method::GET, "/health", None)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, Value::String("ok".into()));
}

#[tokio::test]
async fn openapi_json_lists_paths_and_schemas() {
    let app = app();
    let (status, body) = send(&app, req(Method::GET, "/openapi.json", None)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["openapi"].as_str().unwrap_or(""), "3.1.0");
    assert!(body["info"]["title"].as_str().unwrap().contains("PRMS"));
    let paths = body["paths"].as_object().expect("paths object");
    assert!(paths.contains_key("/health"));
    assert!(paths.contains_key("/passengers"));
    assert!(paths.contains_key("/access"));
    let schemas = body["components"]["schemas"]
        .as_object()
        .expect("schemas object");
    assert!(schemas.contains_key("PassengerDto"));
    assert!(schemas.contains_key("TierDto"));
}

#[tokio::test]
async fn request_id_is_assigned_and_propagated() {
    let app = app();
    let r = req(Method::GET, "/health", None);
    let res = app.clone().oneshot(r).await.unwrap();
    let id = res
        .headers()
        .get("x-request-id")
        .expect("x-request-id header")
        .to_str()
        .unwrap()
        .to_string();
    assert!(!id.is_empty());
    // Drain body so the test fully completes the response.
    let _ = res.into_body().collect().await.unwrap();
}

#[tokio::test]
async fn request_id_echoes_client_supplied_value() {
    let app = app();
    let r = Request::builder()
        .method(Method::GET)
        .uri("/health")
        .header("x-request-id", "client-supplied-123")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(r).await.unwrap();
    assert_eq!(
        res.headers().get("x-request-id").unwrap(),
        "client-supplied-123"
    );
    let _ = res.into_body().collect().await.unwrap();
}

#[tokio::test]
async fn unknown_route_returns_404() {
    let app = app();
    let (status, _) = send(&app, req(Method::GET, "/does-not-exist", None)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn malformed_json_body_returns_4xx() {
    let app = app();
    let bad = Request::builder()
        .method(Method::POST)
        .uri("/access")
        .header("content-type", "application/json")
        .body(Body::from("{not-json"))
        .expect("request");
    let (status, _) = send(&app, bad).await;
    assert!(status.is_client_error());
}

#[tokio::test]
async fn unknown_tier_string_is_rejected() {
    let app = app();
    let (status, _) = send(
        &app,
        req(
            Method::POST,
            "/passengers",
            Some(json!({
                "actor_id": ARIA,
                "id": "ps-x",
                "name": "X",
                "tier": "Bronze"
            })),
        ),
    )
    .await;
    assert!(status.is_client_error());
}

#[tokio::test]
async fn oversized_body_is_rejected_with_413() {
    let app = app();
    let huge = "x".repeat(70 * 1024);
    let body = json!({
        "actor_id": ARIA,
        "id": "ps-x",
        "name": huge,
        "tier": "Silver"
    });
    let r = Request::builder()
        .method(Method::POST)
        .uri("/passengers")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();
    let res = app.clone().oneshot(r).await.unwrap();
    assert_eq!(res.status(), StatusCode::PAYLOAD_TOO_LARGE);
}
