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
//! - `--enable-reset` / `PRMS_ENABLE_RESET` — register the `/reset` route
//!   (default: false). Never enable this in production.
//! - `--shutdown-grace-secs` / `PRMS_SHUTDOWN_GRACE_SECS` (default 10)

use std::net::SocketAddr;
use std::process::ExitCode;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::http::HeaderValue;
// `clap` = command-line argument parser. Deriving `Parser` on a struct
// turns its fields into named CLI flags / env vars. Magic powered by
// proc-macros — see the `#[arg(...)]` attributes below.
use clap::Parser;
// Importing from the LIBRARY crate by its package name (replace
// hyphens with underscores). This binary is separate from `lib.rs` —
// it links against it like any external consumer.
use passenger_resource_management::interface::composition_root::build_demo_world;
use passenger_resource_management::interface::http::{CorsOrigins, router_with};
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(name = "serve", about = "PRMS HTTP server")]
struct Args {
    /// Address to bind, e.g. `127.0.0.1:8080` or `0.0.0.0:8080`.
    // `#[arg(long, env = "...", default_value = "...")]` declares:
    //   long       -> --bind on the CLI (no -b short form)
    //   env        -> falls back to the env var if the flag is absent
    //   default    -> used when both flag and env var are missing
    // `bind: SocketAddr` -> clap parses the string into `SocketAddr`
    // automatically because `SocketAddr` implements `FromStr`.
    #[arg(long, env = "PRMS_BIND", default_value = "127.0.0.1:8080")]
    bind: SocketAddr,

    /// Comma-separated list of allowed CORS origins. Unset means `Any`.
    // No default -> Option<String>. None when the flag and env are absent.
    #[arg(long, env = "PRMS_CORS_ORIGINS")]
    cors_origins: Option<String>,

    /// Register the `/reset` endpoint. NEVER enable in production.
    // `default_value_t = false` makes this opt-in rather than opt-out.
    #[arg(long, env = "PRMS_ENABLE_RESET", default_value_t = false)]
    enable_reset: bool,

    /// Maximum seconds to wait for in-flight requests to drain after
    /// SIGINT before forcibly exiting.
    // `default_value_t = 10` provides a typed (not stringly) default.
    #[arg(long, env = "PRMS_SHUTDOWN_GRACE_SECS", default_value_t = 10)]
    shutdown_grace_secs: u64,
}

// `#[tokio::main]` is an attribute macro that wraps `main` in a tokio
// runtime, so the function can be `async`. Without it, you'd need to
// build the runtime by hand. `ExitCode` lets us return non-zero codes
// without panicking — cleaner than `process::exit`.
#[tokio::main]
async fn main() -> ExitCode {
    // RUST_LOG=info,tower_http=debug for verbose.
    // tracing-subscriber installs a global logger. `EnvFilter` reads
    // `RUST_LOG` (Rust's de-facto log-level env var). Falls back to
    // "info" if that variable is missing or invalid.
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    // `Args::parse()` (from the Parser derive) reads argv + env and
    // exits with a friendly error if anything is malformed.
    let args = Args::parse();

    // `.as_deref()` converts `Option<String>` -> `Option<&str>` so we
    // can match against string slices below.
    let cors = match args.cors_origins.as_deref() {
        // Match BOTH `None` AND `Some("")` in one arm with `|`.
        // WARN: Any origin is allowed — set PRMS_CORS_ORIGINS before exposing
        // the server beyond localhost. This is safe for local dev only.
        None | Some("") => {
            tracing::warn!(
                "CORS is set to Any (all origins allowed). \
                 Set PRMS_CORS_ORIGINS to a comma-separated allow-list before \
                 exposing this server beyond localhost."
            );
            CorsOrigins::Any
        }
        Some(list) => {
            let mut parsed: Vec<HeaderValue> = Vec::new();
            // Iterator pipeline: split on ',', trim each, drop empties.
            // `str::trim` is a *function pointer* (no closure needed).
            for origin in list.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                match HeaderValue::from_str(origin) {
                    Ok(v) => parsed.push(v),
                    Err(e) => {
                        // `eprintln!` -> stderr (vs `println!` -> stdout).
                        // `{origin:?}` uses Debug formatting (adds quotes).
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
    // Wrap the World in Arc<Mutex<...>> for sharing across handlers.
    // See in_memory_admin_event_sink.rs for the pattern explanation.
    let state = Arc::new(Mutex::new(world));

    // Build the router and add request tracing as the OUTERMOST layer
    // (logs every request/response pair).
    let app = router_with(state, cors, args.enable_reset).layer(TraceLayer::new_for_http());
    if args.enable_reset {
        tracing::warn!(
            "The /reset endpoint is enabled. This wipes all state and must \
             never be reachable in production."
        );
    }

    let addr = args.bind;
    // `.await` suspends the async function until the future completes.
    // Only legal inside `async fn` / async blocks.
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("failed to bind {addr}: {e}");
            return ExitCode::from(1);
        }
    };
    // `%addr` formats with Display (=> structured field). `?addr` would
    // use Debug. tracing's macros support both.
    tracing::info!(%addr, "PRMS HTTP server listening");

    // The shutdown-signal future also notifies a watch channel so the
    // drain timeout can begin counting from the moment ctrl-c arrived.
    // `oneshot` = single-producer single-consumer one-shot channel.
    let (signal_tx, signal_rx) = tokio::sync::oneshot::channel::<()>();
    let serve_fut = axum::serve(listener, app).with_graceful_shutdown(async move {
        // `async move { ... }` is an async block that captures
        // surrounding variables BY MOVE (so signal_tx lives long enough).
        // `let _ = expr;` explicitly discards a Result we don't care about.
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
            // `pending()` returns a future that NEVER completes, which
            // effectively disables this branch in the select! below.
            std::future::pending::<()>().await;
        }
    };

    // `tokio::select!` runs MULTIPLE futures concurrently and resolves
    // when ANY ONE completes. `biased;` checks branches in declaration
    // order each poll (default is random — fair scheduling). We bias so
    // the server's exit takes priority over the timeout.
    //
    // Bind the select! result to a typed variable so rust-analyzer's
    // macro expansion infers `ExitCode` unambiguously across both arms.
    let exit: ExitCode = tokio::select! {
        biased;
        // Pattern `res = future` binds the future's output to `res`.
        res = serve_fut => match res {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("server error: {e}");
                ExitCode::from(1)
            }
        },
        // `()` is a unit pattern — force_exit_fut returns `()` on timeout.
        () = force_exit_fut => {
            tracing::warn!(
                grace_secs = grace.as_secs(),
                "graceful shutdown timed out; forcing exit"
            );
            ExitCode::from(1)
        }
    };
    exit
}
