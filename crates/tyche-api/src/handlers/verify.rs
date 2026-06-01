//! `POST /v1/verify` — verify an attestation record's signature.

use axum::Json;
use serde::{Deserialize, Serialize};
use tyche_attest::{AttestationRecord, verify_attestation};

use crate::metrics::names;

/// Request body for `/v1/verify`.
#[derive(Deserialize)]
pub struct VerifyRequest {
    record: AttestationRecord,
}

/// Response body for `/v1/verify`.
#[derive(Serialize)]
pub struct VerifyResponse {
    verified: bool,
    detail: Option<String>,
}

/// Handler. Always 200 — a failed *signature* check is a valid result, not an
/// HTTP error. Only a malformed request body yields a 4xx (handled upstream by
/// the JSON extractor).
pub async fn handler(Json(req): Json<VerifyRequest>) -> Json<VerifyResponse> {
    metrics::counter!(names::VERIFICATIONS).increment(1);
    match verify_attestation(&req.record) {
        Ok(()) => Json(VerifyResponse {
            verified: true,
            detail: None,
        }),
        Err(e) => {
            metrics::counter!(names::VERIFICATION_FAILURES).increment(1);
            Json(VerifyResponse {
                verified: false,
                detail: Some(e.to_string()),
            })
        }
    }
}
