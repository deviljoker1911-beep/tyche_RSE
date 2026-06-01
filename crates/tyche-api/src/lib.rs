//! `tyche-api` — HTTP front-end to the Tyche core.
//!
//! The crate is split so the router is testable without binding a socket:
//! [`build_app`] returns a fully-layered [`axum::Router`] that
//! `tower::ServiceExt::oneshot` can drive directly. The binary
//! (`src/main.rs`) is a thin wrapper that installs the metrics recorder,
//! constructs [`state::AppState`], and serves [`build_app`].
//!
//! # Request lifecycle (outer → inner)
//!
//! ```text
//! SetRequestId → PropagateRequestId → Trace → track_http_metrics
//!   → resilience(timeout 30s, load-shed, concurrency-limit)
//!   → rate-limit (per-tenant token bucket)
//!   → body-limit (1 MiB)
//!   → handler
//! ```
//!
//! `track_http_metrics` sits *outside* the resilience and rate-limit layers
//! so the recorded status reflects 429 / 503 / 504 responses they generate.

#![forbid(unsafe_code)]

pub mod error;
pub mod handlers;
pub mod metrics;
pub mod middleware;
pub mod rate_limit;
pub mod state;

use std::time::Duration;

use axum::Router;
use axum::error_handling::HandleErrorLayer;
use axum::extract::DefaultBodyLimit;
use axum::middleware::{from_fn, from_fn_with_state};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use tower::{BoxError, ServiceBuilder};
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::trace::TraceLayer;

use crate::state::AppState;

/// Maximum request body (1 MiB). A 50-loan portfolio is ~30 KiB; 1 MiB covers
/// books into the thousands of loans with headroom. Larger books use the
/// chunked async-submission path (M2).
const MAX_BODY_BYTES: usize = 1024 * 1024;

/// Default per-request hard time ceiling. Beyond this the request is abandoned
/// with 504. Longer jobs move to queued execution in M2.
const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Default in-flight concurrency ceiling before load-shedding with 503.
const DEFAULT_MAX_INFLIGHT: usize = 256;

/// Build the fully-layered application router.
pub fn build_app(state: AppState) -> Router {
    let timeout = Duration::from_secs(read_env_usize(
        "TYCHE_TIMEOUT_SECS",
        DEFAULT_TIMEOUT_SECS as usize,
    ) as u64);
    let max_inflight = read_env_usize("TYCHE_MAX_INFLIGHT", DEFAULT_MAX_INFLIGHT);

    // Resilience stack. HandleErrorLayer must be outermost so it catches the
    // errors that `timeout` (Elapsed) and `load_shed` (Overloaded) raise and
    // converts them into clean HTTP responses.
    let resilience = ServiceBuilder::new()
        .layer(HandleErrorLayer::new(handle_resilience_error))
        .timeout(timeout)
        .load_shed()
        .concurrency_limit(max_inflight);

    Router::new()
        .route("/healthz", get(handlers::health::healthz))
        .route("/readyz", get(handlers::health::readyz))
        .route("/version", get(handlers::health::version))
        .route("/metrics", get(handlers::health::metrics))
        .route("/v1/simulate", post(handlers::simulate::handler))
        .route("/v1/attest", post(handlers::attest::handler))
        .route("/v1/verify", post(handlers::verify::handler))
        // Layers below are listed innermost-first: each `.layer` wraps the
        // previous, so the LAST call is the outermost in the onion.
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .layer(from_fn_with_state(state.clone(), middleware::rate_limit))
        .layer(resilience)
        .layer(from_fn(middleware::track_http_metrics))
        .layer(TraceLayer::new_for_http())
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .with_state(state)
}

/// Map resilience-layer errors to HTTP responses.
async fn handle_resilience_error(err: BoxError) -> Response {
    if err.is::<tower::timeout::error::Elapsed>() {
        error::ApiError::Timeout.into_response()
    } else if err.is::<tower::load_shed::error::Overloaded>() {
        error::ApiError::Overloaded.into_response()
    } else {
        error::ApiError::Internal(err.to_string()).into_response()
    }
}

fn read_env_usize(key: &str, default: usize) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}
