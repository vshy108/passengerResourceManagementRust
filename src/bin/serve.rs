//! `serve` — start the HTTP adapter.
//!
//! Built only when the `http` feature is enabled. The composition root
//! seeds an in-memory demo world; state is process-local and resets on
//! restart (mirrors the TS demo's behaviour).
//!
//! Configure via flags or the matching environment variables:
//! - `--bind` / `PRMS_BIND` (default `127.0.0.1:8080`)
//! - `--cors-origins` / `PRMS_CORS_ORIGINS` — comma-separated list of
//!   allowed origins. When unset, CORS allows any origin (dev default).
//! - `--shutdown-grace-secs` / `PRMS_SHUTDOWN_GRACE_SECS` (default 10)

use std::net::SocketAddr;
use std::process::ExitCode;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::http::HeaderValue;
use clap::Parser;
use passenger_resource_management::interface::composition_root::build_demo_world;
use passenger_resource_management::interface::http::{CorsOrigins, router_with};
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(name = "serve", about = "PRMS HTTP server")]
struct Args {
    /// Address to bind, e.g. `127.0.0.1:8080` or `0.0.0.0:8080`.
    #[arg(long, env = "PRMS_BIND", default_value = "127.0.0.1:8080")]
    bind: SocketAddr,

    /// Comma-separated list of allowed CORS origins. Unset means `Any`.
    #[arg(long, env = "PRMS_CORS_ORIGINS")]
    cors_origins: Option<String>,

    /// Maximum seconds to wait for in-flight requests to drain after
    /// SIGINT before forcibly exiting.
    #[arg(long, env = "PRMS_SHUTDOWN_GRACE_SECS", default_value_t = 10)]
    shutdown_grace_secs: u64,
}

#[tokio::main]
async fn main() -> ExitCode {
    // RUST_LOG=info,tower_http=debug for verbose.
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let args = Args::parse();

    let cors = match args.cors_origins.as_deref() {
        None | Some("") => CorsOrigins::Any,
        Some(list) => {
            let mut parsed: Vec<HeaderValue> = Vec::new();
            for origin in list.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                match HeaderValue::from_str(origin) {
                    Ok(v) => parsed.push(v),
                    Err(e) => {
                        eprintln!("invalid CORS origin {origin:?}: {e}");
                        return ExitCode::from(1);
                    }
                }
            }
            CorsOrigins::List(parsed)
        }
    };

    let world = match build_demo_world() {
        Ok(w) => w,
        Err(e) => {
            eprintln!("failed to bootstrap demo world: {e}");
            return ExitCode::from(1);
        }
    };
    let state = Arc::new(Mutex::new(world));

    let app = router_with(state, cors).layer(TraceLayer::new_for_http());

    let addr = args.bind;
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("failed to bind {addr}: {e}");
            return ExitCode::from(1);
        }
    };
    tracing::info!(%addr, "PRMS HTTP server listening");

    // The shutdown-signal future also notifies a watch channel so the
    // drain timeout can begin counting from the moment ctrl-c arrived.
    let (signal_tx, signal_rx) = tokio::sync::oneshot::channel::<()>();
    let serve_fut = axum::serve(listener, app).with_graceful_shutdown(async move {
        let _ = tokio::signal::ctrl_c().await;
        tracing::info!("shutdown signal received; draining in-flight requests");
        let _ = signal_tx.send(());
    });

    let grace = Duration::from_secs(args.shutdown_grace_secs);
    let force_exit_fut = async move {
        // Wait until the signal future fires, then start the grace
        // timer. If the timer elapses before `serve_fut` returns, we
        // force-exit on a stuck connection.
        if signal_rx.await.is_ok() {
            tokio::time::sleep(grace).await;
        } else {
            // Channel dropped without sending — server exited normally.
            std::future::pending::<()>().await;
        }
    };

    tokio::select! {
        biased;
        res = serve_fut => match res {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("server error: {e}");
                ExitCode::from(1)
            }
        },
        () = force_exit_fut => {
            tracing::warn!(
                grace_secs = grace.as_secs(),
                "graceful shutdown timed out; forcing exit"
            );
            ExitCode::from(1)
        }
    }
}
