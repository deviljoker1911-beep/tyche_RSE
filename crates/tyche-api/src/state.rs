//! Shared application state.

use std::sync::Arc;

use metrics_exporter_prometheus::PrometheusHandle;

use crate::rate_limit::RateLimiter;

/// Build metadata baked in at compile time (set by the Dockerfile / CI).
pub const COMMIT_SHA: &str = match option_env!("TYCHE_COMMIT_SHA") {
    Some(s) => s,
    None => "unknown",
};

/// Build timestamp baked in at compile time.
pub const BUILD_TIME: &str = match option_env!("TYCHE_BUILD_TIME") {
    Some(s) => s,
    None => "unknown",
};

/// The simulation model version this binary implements. Bumped on any change
/// that alters simulation outputs, so attestation records stay attributable to
/// a specific model.
pub const MODEL_VERSION: &str = "v0.1.0-spike";

/// Cloneable application state injected into handlers.
///
/// Everything inside is cheap to clone (`Arc` / handle types) so axum can
/// clone it per request without cost.
#[derive(Clone)]
pub struct AppState {
    /// Deployment environment label (`dev` / `staging` / `prod`).
    pub env: Arc<str>,
    /// Prometheus render handle, surfaced at `/metrics`.
    pub metrics: PrometheusHandle,
    /// Per-tenant rate limiter.
    pub limiter: Arc<RateLimiter>,
}

impl AppState {
    /// Construct application state.
    #[must_use]
    pub fn new(metrics: PrometheusHandle, limiter: Arc<RateLimiter>) -> Self {
        let env = std::env::var("TYCHE_ENV").unwrap_or_else(|_| "dev".to_string());
        Self {
            env: Arc::from(env.as_str()),
            metrics,
            limiter,
        }
    }
}
