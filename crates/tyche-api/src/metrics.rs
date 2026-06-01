//! Prometheus metrics wiring.
//!
//! The dashboards under `observability/grafana/dashboards/` select on the
//! metric names emitted here. Keep the names in sync — a rename is a
//! dashboard-breaking change.
//!
//! ## Emitted series
//!
//! | Metric | Type | Labels |
//! |--------|------|--------|
//! | `http_requests_total` | counter | `method`, `route`, `status` |
//! | `http_request_duration_seconds` | histogram | `method`, `route` |
//! | `tyche_simulations_total` | counter | — |
//! | `tyche_simulation_duration_seconds` | histogram | — |
//! | `tyche_simulation_n_paths` | gauge | — |
//! | `tyche_simulation_n_loans` | gauge | — |
//! | `tyche_simulation_paths_total` | counter | — |
//! | `tyche_attestations_signed_total` | counter | — |
//! | `tyche_verifications_total` | counter | — |
//! | `tyche_verification_failures_total` | counter | — |
//!
//! Two global labels (`tyche_component=api`, `tyche_env=$TYCHE_ENV`) are
//! attached to every series by the recorder so the dashboards can filter by
//! environment.

use metrics_exporter_prometheus::{Matcher, PrometheusBuilder, PrometheusHandle};

/// Histogram buckets (seconds) shared by every `_seconds` metric. Tuned for
/// the Tyche latency envelope: sub-millisecond health checks through to the
/// 30 s request ceiling.
const SECONDS_BUCKETS: &[f64] = &[
    0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0,
];

/// Metric names. Centralised so handlers and middleware can't drift from the
/// dashboard selectors.
pub mod names {
    /// HTTP request counter.
    pub const HTTP_REQUESTS: &str = "http_requests_total";
    /// HTTP request duration histogram.
    pub const HTTP_DURATION: &str = "http_request_duration_seconds";
    /// Total simulations run.
    pub const SIMS: &str = "tyche_simulations_total";
    /// Simulation duration histogram.
    pub const SIM_DURATION: &str = "tyche_simulation_duration_seconds";
    /// Last simulation path count.
    pub const SIM_N_PATHS: &str = "tyche_simulation_n_paths";
    /// Last simulation loan count.
    pub const SIM_N_LOANS: &str = "tyche_simulation_n_loans";
    /// Cumulative simulated paths.
    pub const SIM_PATHS_TOTAL: &str = "tyche_simulation_paths_total";
    /// Attestation records signed.
    pub const ATTESTATIONS: &str = "tyche_attestations_signed_total";
    /// Attestation verifications attempted.
    pub const VERIFICATIONS: &str = "tyche_verifications_total";
    /// Attestation verifications that failed.
    pub const VERIFICATION_FAILURES: &str = "tyche_verification_failures_total";
}

/// Install the global Prometheus recorder and return a render handle.
///
/// Call exactly once at process start. Panics if a recorder is already
/// installed (which would indicate a double-bootstrap bug).
#[must_use]
pub fn install() -> PrometheusHandle {
    let env = std::env::var("TYCHE_ENV").unwrap_or_else(|_| "dev".to_string());
    PrometheusBuilder::new()
        .set_buckets_for_metric(Matcher::Suffix("_seconds".to_string()), SECONDS_BUCKETS)
        .expect("histogram buckets are valid")
        .add_global_label("tyche_component", "api")
        .add_global_label("tyche_env", env)
        .install_recorder()
        .expect("no global metrics recorder already installed")
}

/// Build a recorder + handle **without** installing it globally.
///
/// Used by tests and by `build_app` callers that don't want to mutate global
/// process state. Metric macros invoked while no global recorder is installed
/// are cheap no-ops, so a handle built this way renders an empty (but valid)
/// exposition document.
#[must_use]
pub fn test_handle() -> PrometheusHandle {
    PrometheusBuilder::new()
        .set_buckets_for_metric(Matcher::Suffix("_seconds".to_string()), SECONDS_BUCKETS)
        .expect("histogram buckets are valid")
        .build_recorder()
        .handle()
}
