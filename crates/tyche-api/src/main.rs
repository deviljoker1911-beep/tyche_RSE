//! `tyche-api` — HTTP front-end to the Tyche core.
//!
//! Stateless server in the M1C scaffold. State (Postgres / Redis / ClickHouse)
//! lands in M1G; per-tenant rate limits + circuit breakers in M1H.
//!
//! # Run
//!
//! ```sh
//! cargo run -p tyche-api
//! TYCHE_API_ADDR=127.0.0.1:9090 cargo run -p tyche-api
//! ```

#![forbid(unsafe_code)]

use std::net::SocketAddr;
use std::time::Duration;

use anyhow::{Context, Result};
use axum::Json;
use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use ed25519_dalek::SigningKey;
use rand::RngCore;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;
use tyche_attest::{AttestationRecord, build_attestation, verify_attestation};
use tyche_sim::{SimConfig, simulate};
use tyche_types::{MacroScenario, Portfolio, RiskMetrics};

/// Maximum portfolio request body (1 MiB).
const MAX_BODY_BYTES: usize = 1024 * 1024;

/// Build metadata baked at compile time.
const COMMIT_SHA: &str = match option_env!("TYCHE_COMMIT_SHA") {
    Some(s) => s,
    None => "unknown",
};

const BUILD_TIME: &str = match option_env!("TYCHE_BUILD_TIME") {
    Some(s) => s,
    None => "unknown",
};

const MODEL_VERSION: &str = "v0.1.0-spike";

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .json()
        .init();

    let addr: SocketAddr = std::env::var("TYCHE_API_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:8080".to_string())
        .parse()
        .context("invalid TYCHE_API_ADDR")?;

    let app = router();

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("bind {addr}"))?;
    tracing::info!(%addr, commit = COMMIT_SHA, "tyche-api listening");

    axum::serve(listener, app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("server")?;
    Ok(())
}

fn router() -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/version", get(version))
        .route("/v1/simulate", post(simulate_handler))
        .route("/v1/attest", post(attest_handler))
        .route("/v1/verify", post(verify_handler))
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .layer(RequestBodyLimitLayer::new(MAX_BODY_BYTES))
        // 30s hard ceiling on every request. 504 on expiry. Longer jobs
        // move to async/queued execution in M1H.
        .layer(TimeoutLayer::with_status_code(
            StatusCode::GATEWAY_TIMEOUT,
            Duration::from_secs(30),
        ))
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(TraceLayer::new_for_http())
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

// ============================================================================
// Handlers
// ============================================================================

#[derive(Serialize)]
struct HealthBody {
    status: &'static str,
}

async fn healthz() -> impl IntoResponse {
    Json(HealthBody { status: "ok" })
}

async fn readyz() -> impl IntoResponse {
    // M1C: nothing to check yet (stateless). M1G will probe DB / Redis here.
    Json(HealthBody { status: "ready" })
}

#[derive(Serialize)]
struct VersionBody {
    commit: &'static str,
    build_time: &'static str,
    model_version: &'static str,
}

async fn version() -> impl IntoResponse {
    Json(VersionBody {
        commit: COMMIT_SHA,
        build_time: BUILD_TIME,
        model_version: MODEL_VERSION,
    })
}

#[derive(Deserialize)]
struct SimulateRequest {
    portfolio: Portfolio,
    scenario: MacroScenario,
    #[serde(default)]
    config: Option<SimConfigInput>,
}

#[derive(Deserialize, Default)]
struct SimConfigInput {
    n_paths: Option<u64>,
    sector_correlation: Option<f64>,
    market_correlation: Option<f64>,
    seed: Option<u64>,
    chunk_size: Option<usize>,
}

impl SimConfigInput {
    fn merge(self) -> SimConfig {
        let base = SimConfig::default();
        SimConfig {
            n_paths: self.n_paths.unwrap_or(base.n_paths),
            sector_correlation: self.sector_correlation.unwrap_or(base.sector_correlation),
            market_correlation: self.market_correlation.unwrap_or(base.market_correlation),
            seed: self.seed.unwrap_or(base.seed),
            chunk_size: self.chunk_size.unwrap_or(base.chunk_size),
            recovery_model: base.recovery_model,
        }
    }
}

async fn simulate_handler(Json(req): Json<SimulateRequest>) -> Result<Json<RiskMetrics>, ApiError> {
    let cfg = req.config.unwrap_or_default().merge();
    let metrics = simulate(&req.portfolio, &req.scenario, cfg)
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok(Json(metrics))
}

#[derive(Deserialize)]
struct AttestRequest {
    portfolio: Portfolio,
    scenario: MacroScenario,
    metrics: RiskMetrics,
    #[serde(default = "default_model_version")]
    model_version: String,
    /// Optional 32-byte hex Ed25519 secret key. If absent, an ephemeral one
    /// is generated and returned in the response. **Developer convenience —
    /// production deployments must supply a key referenced by the firm's HSM.**
    signer_sk_hex: Option<String>,
}

fn default_model_version() -> String {
    MODEL_VERSION.to_string()
}

#[derive(Serialize)]
struct AttestResponse {
    record: AttestationRecord,
    /// Returned only if no `signer_sk_hex` was supplied — the caller must
    /// persist this themselves. Never logged.
    ephemeral_signer_sk_hex: Option<String>,
}

async fn attest_handler(Json(req): Json<AttestRequest>) -> Result<Json<AttestResponse>, ApiError> {
    let (signer, ephemeral) = if let Some(h) = req.signer_sk_hex {
        let bytes = hex::decode(&h).map_err(|e| ApiError::BadRequest(e.to_string()))?;
        let arr: [u8; 32] = bytes
            .try_into()
            .map_err(|_| ApiError::BadRequest("signer_sk_hex must be 32 bytes".into()))?;
        (SigningKey::from_bytes(&arr), None)
    } else {
        let sk = SigningKey::generate(&mut OsRng);
        let hex = hex::encode(sk.to_bytes());
        (sk, Some(hex))
    };

    let mut blinding = [0u8; 32];
    OsRng.fill_bytes(&mut blinding);

    let record = build_attestation(
        &req.portfolio,
        &blinding,
        &req.scenario,
        &req.metrics,
        &req.model_version,
        &signer,
    )
    .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok(Json(AttestResponse {
        record,
        ephemeral_signer_sk_hex: ephemeral,
    }))
}

#[derive(Deserialize)]
struct VerifyRequest {
    record: AttestationRecord,
}

#[derive(Serialize)]
struct VerifyResponse {
    verified: bool,
    detail: Option<String>,
}

async fn verify_handler(Json(req): Json<VerifyRequest>) -> Json<VerifyResponse> {
    match verify_attestation(&req.record) {
        Ok(()) => Json(VerifyResponse {
            verified: true,
            detail: None,
        }),
        Err(e) => Json(VerifyResponse {
            verified: false,
            detail: Some(e.to_string()),
        }),
    }
}

// ============================================================================
// Errors
// ============================================================================

#[derive(Debug, thiserror::Error)]
enum ApiError {
    #[error("bad request: {0}")]
    BadRequest(String),
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, error) = match self {
            ApiError::BadRequest(e) => (StatusCode::BAD_REQUEST, e),
        };
        (status, Json(ErrorBody { error })).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::util::ServiceExt;

    #[tokio::test]
    async fn healthz_returns_ok() {
        let app = router();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/healthz")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn readyz_returns_ready() {
        let app = router();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/readyz")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn version_returns_metadata() {
        let app = router();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/version")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(v.get("model_version").is_some());
    }
}
