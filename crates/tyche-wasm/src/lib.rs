//! Browser bindings for the Tyche core.
//!
//! All inputs cross the JS boundary as plain JSON (string), not as
//! serde-wasm-bindgen objects: the JS-side overhead of nested object
//! marshalling is significant on portfolios with hundreds of loans, and the
//! string round-trip is straightforward. Outputs return as objects so the JS
//! side can index into them without a second `JSON.parse`.

#![allow(clippy::missing_errors_doc)]
#![forbid(unsafe_code)]

use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use serde::Deserialize;
use serde_wasm_bindgen::to_value;
use tyche_attest::{AttestationRecord, build_attestation};
use tyche_sim::{SimConfig, simulate};
use tyche_types::{MacroScenario, Portfolio};
use wasm_bindgen::prelude::*;

/// Initialise wasm runtime hooks. Call once at module load time from JS.
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}

#[derive(Deserialize)]
struct SimulateInput {
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
    fn merge(self, base: SimConfig) -> SimConfig {
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

/// Simulate a portfolio under a scenario, returning the resulting metrics
/// as a JS object.
///
/// Input shape: `{ portfolio, scenario, config? }`.
#[wasm_bindgen(js_name = simulate)]
pub fn simulate_js(input_json: &str) -> Result<JsValue, JsError> {
    let parsed: SimulateInput = serde_json::from_str(input_json)
        .map_err(|e| JsError::new(&format!("invalid input: {e}")))?;
    let cfg = parsed
        .config
        .unwrap_or_default()
        .merge(SimConfig::default());
    let metrics = simulate(&parsed.portfolio, &parsed.scenario, cfg)
        .map_err(|e| JsError::new(&e.to_string()))?;
    to_value(&metrics).map_err(|e| JsError::new(&e.to_string()))
}

#[derive(Deserialize)]
struct AttestInput {
    portfolio: Portfolio,
    scenario: MacroScenario,
    metrics: tyche_types::RiskMetrics,
    model_version: String,
    /// Optional 32-byte hex blinding factor for the portfolio commitment.
    /// If absent, a random one is sampled.
    portfolio_blinding_hex: Option<String>,
    /// Optional 32-byte hex Ed25519 secret key. If absent, an ephemeral one
    /// is generated (useful for browser demos that don't manage keys).
    signer_sk_hex: Option<String>,
}

/// Build a signed attestation record for a (portfolio, scenario, metrics)
/// triple. Returns the record as a JS object plus the signing key in hex
/// (caller is responsible for handling the secret).
#[wasm_bindgen(js_name = buildAttestation)]
pub fn build_attestation_js(input_json: &str) -> Result<JsValue, JsError> {
    let parsed: AttestInput = serde_json::from_str(input_json)
        .map_err(|e| JsError::new(&format!("invalid input: {e}")))?;
    let blinding: [u8; 32] = parsed
        .portfolio_blinding_hex
        .map(|h| {
            let v = hex::decode(&h).map_err(|e| JsError::new(&e.to_string()))?;
            v.as_slice()
                .try_into()
                .map_err(|_| JsError::new("blinding must be 32 bytes"))
        })
        .transpose()?
        .unwrap_or_else(|| {
            let mut buf = [0u8; 32];
            rand::RngCore::fill_bytes(&mut OsRng, &mut buf);
            buf
        });
    let sk = if let Some(h) = parsed.signer_sk_hex {
        let v = hex::decode(&h).map_err(|e| JsError::new(&e.to_string()))?;
        let arr: [u8; 32] = v
            .as_slice()
            .try_into()
            .map_err(|_| JsError::new("signer_sk_hex must be 32 bytes"))?;
        SigningKey::from_bytes(&arr)
    } else {
        SigningKey::generate(&mut OsRng)
    };
    let record: AttestationRecord = build_attestation(
        &parsed.portfolio,
        &blinding,
        &parsed.scenario,
        &parsed.metrics,
        &parsed.model_version,
        &sk,
    )
    .map_err(|e| JsError::new(&e.to_string()))?;
    to_value(&record).map_err(|e| JsError::new(&e.to_string()))
}

/// Verify the signature on an attestation record passed as JSON.
#[wasm_bindgen(js_name = verifyAttestation)]
pub fn verify_attestation_js(record_json: &str) -> Result<bool, JsError> {
    let record: AttestationRecord = serde_json::from_str(record_json)
        .map_err(|e| JsError::new(&format!("invalid record: {e}")))?;
    Ok(tyche_attest::verify_attestation(&record).is_ok())
}
