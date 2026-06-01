//! `POST /v1/simulate` — run a Monte Carlo simulation, return risk metrics.

use std::time::Instant;

use axum::Json;
use serde::Deserialize;
use tyche_sim::{SimConfig, simulate};
use tyche_types::{MacroScenario, Portfolio, RiskMetrics};

use crate::error::ApiError;
use crate::metrics::names;

/// Request body for `/v1/simulate`.
#[derive(Deserialize)]
pub struct SimulateRequest {
    portfolio: Portfolio,
    scenario: MacroScenario,
    #[serde(default)]
    config: Option<SimConfigInput>,
}

/// Optional simulation overrides. Absent fields fall back to `SimConfig::default`.
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

/// Handler.
pub async fn handler(Json(req): Json<SimulateRequest>) -> Result<Json<RiskMetrics>, ApiError> {
    let n_loans = req.portfolio.loans.len();
    let cfg = req.config.unwrap_or_default().merge();
    let n_paths = cfg.n_paths;

    let started = Instant::now();
    let metrics = simulate(&req.portfolio, &req.scenario, cfg)
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    let elapsed = started.elapsed().as_secs_f64();

    // Domain metrics consumed by the simulation-performance dashboard.
    metrics::counter!(names::SIMS).increment(1);
    metrics::histogram!(names::SIM_DURATION).record(elapsed);
    metrics::gauge!(names::SIM_N_PATHS).set(n_paths as f64);
    metrics::gauge!(names::SIM_N_LOANS).set(n_loans as f64);
    metrics::counter!(names::SIM_PATHS_TOTAL).increment(n_paths);

    Ok(Json(metrics))
}
