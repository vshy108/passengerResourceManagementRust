//! `serve` — start the HTTP adapter on `127.0.0.1:8080`.
//!
//! Built only when the `http` feature is enabled. The composition root
//! seeds an in-memory demo world; state is process-local and resets on
//! restart (mirrors the TS demo's behaviour).

use std::sync::{Arc, Mutex};

use passenger_resource_management::interface::composition_root::build_demo_world;
use passenger_resource_management::interface::http::router;

#[tokio::main]
async fn main() {
    let world = build_demo_world().expect("demo world bootstrap should succeed");
    let state = Arc::new(Mutex::new(world));

    let app = router(state);

    let addr: std::net::SocketAddr = "127.0.0.1:8080".parse().expect("valid bind address");
    println!("PRMS HTTP server listening on http://{addr}");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("bind tcp listener");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("server error");
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    println!("\nshutting down");
}
