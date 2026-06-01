//! `POST /v1/attest` — build a signed attestation record.

use axum::Json;
use ed25519_dalek::SigningKey;
use rand::RngCore;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use tyche_attest::{AttestationRecord, build_attestation};
use tyche_types::{MacroScenario, Portfolio, RiskMetrics};

use crate::error::ApiError;
use crate::metrics::names;
use crate::state::MODEL_VERSION;

/// Request body for `/v1/attest`.
#[derive(Deserialize)]
pub struct AttestRequest {
    portfolio: Portfolio,
    scenario: MacroScenario,
    metrics: RiskMetrics,
    #[serde(default = "default_model_version")]
    model_version: String,
    /// Optional 32-byte hex Ed25519 secret key.
    ///
    /// If absent an ephemeral key is generated and returned in the response.
    /// **Developer convenience only** — production firms supply a key
    /// referenced by their HSM and never let the server mint one.
    signer_sk_hex: Option<String>,
}

fn default_model_version() -> String {
    MODEL_VERSION.to_string()
}

/// Resolve a signing key from the optional hex secret. When `None`, mint an
/// ephemeral key and return its hex so the caller can persist it. The ephemeral
/// path is developer convenience only — production firms always supply a key.
fn resolve_signer(sk_hex: Option<&str>) -> Result<(SigningKey, Option<String>), ApiError> {
    if let Some(h) = sk_hex {
        let bytes = hex::decode(h).map_err(|e| ApiError::BadRequest(e.to_string()))?;
        let arr: [u8; 32] = bytes
            .try_into()
            .map_err(|_| ApiError::BadRequest("signer_sk_hex must be 32 bytes".into()))?;
        Ok((SigningKey::from_bytes(&arr), None))
    } else {
        let sk = SigningKey::generate(&mut OsRng);
        let sk_hex = hex::encode(sk.to_bytes());
        Ok((sk, Some(sk_hex)))
    }
}

/// Response body for `/v1/attest`.
#[derive(Serialize)]
pub struct AttestResponse {
    record: AttestationRecord,
    /// Present only when the server generated an ephemeral key. Never logged.
    ephemeral_signer_sk_hex: Option<String>,
}

/// Handler.
pub async fn handler(Json(req): Json<AttestRequest>) -> Result<Json<AttestResponse>, ApiError> {
    let (signer, ephemeral) = resolve_signer(req.signer_sk_hex.as_deref())?;

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

    metrics::counter!(names::ATTESTATIONS).increment(1);

    Ok(Json(AttestResponse {
        record,
        ephemeral_signer_sk_hex: ephemeral,
    }))
}
