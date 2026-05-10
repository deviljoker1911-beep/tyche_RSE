//! Python bindings for `tyche-sim`.
//!
//! Exposes a single function, `simulate`, that takes JSON-serialised
//! portfolio and scenario inputs (so we don't have to mirror every domain
//! type into PyO3) and returns a Python `dict`.

#![forbid(unsafe_code)]
#![allow(clippy::missing_errors_doc)]

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use tyche_sim::{SimConfig, simulate as core_simulate};
use tyche_types::{MacroScenario, Portfolio};

/// Simulate a Tyche portfolio under a scenario.
///
/// Args:
///     portfolio_json: canonical-JSON encoding of `tyche_types::Portfolio`.
///     scenario_json:  canonical-JSON encoding of `tyche_types::MacroScenario`.
///     n_paths:        number of Monte Carlo paths.
///     seed:           optional reproducibility seed.
///
/// Returns:
///     A dict mirroring `tyche_types::RiskMetrics`.
#[pyfunction]
#[pyo3(signature = (portfolio_json, scenario_json, n_paths, seed=None))]
fn simulate(
    py: Python<'_>,
    portfolio_json: &str,
    scenario_json: &str,
    n_paths: u64,
    seed: Option<u64>,
) -> PyResult<Py<PyDict>> {
    let portfolio: Portfolio = serde_json::from_str(portfolio_json)
        .map_err(|e| PyValueError::new_err(format!("portfolio: {e}")))?;
    let scenario: MacroScenario = serde_json::from_str(scenario_json)
        .map_err(|e| PyValueError::new_err(format!("scenario: {e}")))?;
    let cfg = SimConfig {
        n_paths,
        seed: seed.unwrap_or_else(|| SimConfig::default().seed),
        ..SimConfig::default()
    };
    let metrics = core_simulate(&portfolio, &scenario, cfg)
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    let s = serde_json::to_string(&metrics).map_err(|e| PyValueError::new_err(e.to_string()))?;
    let json_module = py.import_bound("json")?;
    let parsed = json_module.call_method1("loads", (s,))?;
    let dict: Py<PyDict> = parsed.extract()?;
    Ok(dict)
}

/// Module entry point.
#[pymodule]
fn tyche_sim(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(simulate, m)?)?;
    Ok(())
}
