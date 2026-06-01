//! Liveness, readiness, version, and metrics endpoints.

use axum::Json;
use axum::extract::State;
use axum::response::IntoResponse;
use serde::Serialize;

use crate::state::{AppState, BUILD_TIME, COMMIT_SHA, MODEL_VERSION};

#[derive(Serialize)]
struct Health {
    status: &'static str,
}

/// `GET /healthz` — process liveness. Always 200 if the event loop is alive.
pub async fn healthz() -> impl IntoResponse {
    Json(Health { status: "ok" })
}

/// `GET /readyz` — readiness to serve traffic.
///
/// Stateless today, so readiness == liveness. M1G+ will probe Postgres /
/// Redis / chain-RPC reachability here and return 503 until dependencies are
/// healthy, so Kubernetes holds traffic off a pod that can't serve it.
pub async fn readyz() -> impl IntoResponse {
    Json(Health { status: "ready" })
}

#[derive(Serialize)]
// `model_version` is the wire field name consumed by clients; renaming to
// satisfy the lint would be a breaking API change.
#[allow(clippy::struct_field_names)]
struct Version {
    commit: &'static str,
    build_time: &'static str,
    model_version: &'static str,
}

/// `GET /version` — build provenance for support + audit.
pub async fn version() -> impl IntoResponse {
    Json(Version {
        commit: COMMIT_SHA,
        build_time: BUILD_TIME,
        model_version: MODEL_VERSION,
    })
}

/// `GET /metrics` — Prometheus exposition. Scraped by the cluster Prometheus
/// via the `ServiceMonitor` in the Helm chart.
pub async fn metrics(State(state): State<AppState>) -> impl IntoResponse {
    state.metrics.render()
}
