//! `tyche-api` binary entry point.
//!
//! Thin wrapper around [`tyche_api::build_app`]: install the metrics recorder,
//! build shared state, spawn the rate-limiter sweeper, bind the socket, and
//! serve with graceful shutdown on SIGINT / SIGTERM.
//!
//! # Run
//!
//! ```sh
//! cargo run -p tyche-api
//! TYCHE_API_ADDR=127.0.0.1:9090 RUST_LOG=info,tyche=debug cargo run -p tyche-api
//! ```

#![forbid(unsafe_code)]

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use tyche_api::rate_limit::RateLimiter;
use tyche_api::state::{AppState, COMMIT_SHA};
use tyche_api::{build_app, metrics};

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let prometheus = metrics::install();
    let limiter = Arc::new(RateLimiter::from_env());
    let state = AppState::new(prometheus, Arc::clone(&limiter));

    // Background sweep so a flood of distinct tenant keys can't grow the
    // rate-limiter map without bound. Evicts buckets idle for 10 minutes.
    spawn_limiter_sweeper(limiter);

    let addr: SocketAddr = std::env::var("TYCHE_API_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:8080".to_string())
        .parse()
        .context("invalid TYCHE_API_ADDR")?;

    let app = build_app(state);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("bind {addr}"))?;
    tracing::info!(%addr, commit = COMMIT_SHA, "tyche-api listening");

    axum::serve(listener, app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("server error")?;
    Ok(())
}

fn init_tracing() {
    use tracing_subscriber::EnvFilter;
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,tyche=info,tyche_api=info"));
    // The API always logs structured JSON — it runs as a service, never on a
    // human's TTY. The CLI is the human-facing surface.
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .json()
        .with_current_span(true)
        .flatten_event(true)
        .init();
}

fn spawn_limiter_sweeper(limiter: Arc<RateLimiter>) {
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(60));
        loop {
            tick.tick().await;
            limiter.sweep_idle(Duration::from_secs(600));
        }
    });
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("install Ctrl+C handler");
    };
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("install SIGTERM handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
    tracing::info!("shutdown signal received");
}
