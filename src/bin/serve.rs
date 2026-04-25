//! `serve` — start the HTTP adapter on `127.0.0.1:8080`.
//!
//! Built only when the `http` feature is enabled. The composition root
//! seeds an in-memory demo world; state is process-local and resets on
//! restart (mirrors the TS demo's behaviour).

use std::process::ExitCode;
use std::sync::{Arc, Mutex};

use passenger_resource_management::interface::composition_root::build_demo_world;
use passenger_resource_management::interface::http::router;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> ExitCode {
    // RUST_LOG=info,tower_http=debug for verbose.
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let world = match build_demo_world() {
        Ok(w) => w,
        Err(e) => {
            eprintln!("failed to bootstrap demo world: {e}");
            return ExitCode::from(1);
        }
    };
    let state = Arc::new(Mutex::new(world));

    let app = router(state).layer(TraceLayer::new_for_http());

    let addr: std::net::SocketAddr = match "127.0.0.1:8080".parse() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("invalid bind address: {e}");
            return ExitCode::from(1);
        }
    };
    tracing::info!(%addr, "PRMS HTTP server listening");

    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("failed to bind {addr}: {e}");
            return ExitCode::from(1);
        }
    };

    if let Err(e) = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
    {
        eprintln!("server error: {e}");
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("shutting down");
}
