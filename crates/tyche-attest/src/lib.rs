//! Attestation records, signatures, and Merkle batching.
//!
//! See the crate-level README for the high-level shape. This module exposes:
//!
//! - [`AttestationRecord`] — the canonical attestation type.
//! - [`build_attestation`] — produce a signed record from a portfolio,
//!   scenario, and metrics.
//! - [`verify_attestation`] — verify a record's signature and cross-field
//!   consistency.
//! - [`AttestBatch`] — a Merkle batch of records suitable for on-chain
//!   anchoring.

#![forbid(unsafe_code)]

use std::time::{SystemTime, UNIX_EPOCH};

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tyche_crypto::merkle::MerkleTree;
use tyche_types::{MacroScenario, Portfolio, RiskMetrics, canonical_json, hash_object};

/// Domain tag used when hashing the canonical record body for signature.
pub const DOMAIN_RECORD_SIG: &[u8] = b"tyche/v0.1/attestation-record";

/// Errors raised by `tyche-attest`.
#[derive(Debug, thiserror::Error)]
pub enum AttestError {
    /// Hex decoding of a hex-encoded field failed.
    #[error("hex decode: {0}")]
    Hex(#[from] hex::FromHexError),

    /// Canonicalisation of a record body failed.
    #[error("canonicalisation: {0}")]
    Canon(#[from] tyche_types::TypesError),

    /// JSON encoding/decoding failed.
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),

    /// The signer's public key was malformed.
    #[error("invalid public key")]
    InvalidPublicKey,

    /// The signature was malformed.
    #[error("invalid signature encoding")]
    InvalidSignature,

    /// Signature verification failed.
    #[error("signature did not verify")]
    SignatureVerification,

    /// A record's cross-field consistency check failed (e.g. recomputed
    /// hashes don't match stored ones).
    #[error("record consistency: {0}")]
    Consistency(String),
}

/// A signed attestation of one simulation run.
///
/// The struct is `Serialize + Deserialize`. Wire format is JSON in the spike;
/// a binary CBOR encoding will land in Phase 2.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AttestationRecord {
    /// SHA-256 hash of the firm identifier (no cleartext).
    pub firm_id_hash: String,
    /// Hash commitment to the canonical portfolio JSON.
    pub portfolio_commit: String,
    /// Scenario identifier (matches `MacroScenario::scenario_id`).
    pub scenario_id: String,
    /// Version string for the simulation model. Bump on any breaking change.
    pub model_version: String,
    /// Hash of the canonical JSON inputs (portfolio + scenario + config).
    pub input_hash: String,
    /// Hash of the canonical JSON output metrics.
    pub result_hash: String,
    /// Wall-clock attestation time in UNIX milliseconds.
    pub timestamp_unix_ms: u64,
    /// Ed25519 signer public key, hex-encoded.
    pub signer_pubkey: String,
    /// Ed25519 signature over the body, hex-encoded.
    pub signature: String,
}

/// The body of a record that gets signed: every field except `signature`.
///
/// We sign the canonical JSON of this struct, prefixed by [`DOMAIN_RECORD_SIG`].
#[derive(Debug, Serialize)]
struct RecordBody<'a> {
    firm_id_hash: &'a str,
    portfolio_commit: &'a str,
    scenario_id: &'a str,
    model_version: &'a str,
    input_hash: &'a str,
    result_hash: &'a str,
    timestamp_unix_ms: u64,
    signer_pubkey: &'a str,
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

/// Construct a signed attestation record for one simulation run.
///
/// The `firm_id` and `portfolio_blinding` arguments are kept distinct from
/// the public record so that the firm identifier is committed-to but never
/// leaked, even if a record is published in cleartext.
pub fn build_attestation(
    portfolio: &Portfolio,
    portfolio_blinding: &[u8; 32],
    scenario: &MacroScenario,
    metrics: &RiskMetrics,
    model_version: &str,
    signer: &SigningKey,
) -> Result<AttestationRecord, AttestError> {
    let firm_id_hash = sha256_hex(portfolio.firm_id.as_bytes());
    let (portfolio_commit, _) = tyche_crypto::commit_hash(
        canonical_json(portfolio)?.as_slice(),
        Some(*portfolio_blinding),
    );
    let input_hash = hash_object(&InputBundle {
        portfolio,
        scenario,
    })?;
    let result_hash = hash_object(metrics)?;
    let timestamp_unix_ms = now_unix_ms();
    let signer_pubkey = hex::encode(signer.verifying_key().to_bytes());

    let body = RecordBody {
        firm_id_hash: &firm_id_hash,
        portfolio_commit: &portfolio_commit,
        scenario_id: &scenario.scenario_id,
        model_version,
        input_hash: &input_hash,
        result_hash: &result_hash,
        timestamp_unix_ms,
        signer_pubkey: &signer_pubkey,
    };
    let sig = sign_body(signer, &body)?;
    Ok(AttestationRecord {
        firm_id_hash,
        portfolio_commit,
        scenario_id: scenario.scenario_id.clone(),
        model_version: model_version.to_string(),
        input_hash,
        result_hash,
        timestamp_unix_ms,
        signer_pubkey,
        signature: hex::encode(sig.to_bytes()),
    })
}

fn sign_body(signer: &SigningKey, body: &RecordBody<'_>) -> Result<Signature, AttestError> {
    let body_bytes = canonical_json(body)?;
    let mut h = Sha256::new();
    h.update(DOMAIN_RECORD_SIG);
    h.update([0u8]);
    h.update(&body_bytes);
    let digest = h.finalize();
    Ok(signer.sign(&digest))
}

#[derive(Serialize)]
struct InputBundle<'a> {
    portfolio: &'a Portfolio,
    scenario: &'a MacroScenario,
}

/// Verify the signature on a record, plus light cross-field invariants
/// (hex lengths, presence of all required fields). The caller is responsible
/// for re-running the simulation and comparing `result_hash` if they want to
/// verify the result content as well.
pub fn verify_attestation(record: &AttestationRecord) -> Result<(), AttestError> {
    let pk_bytes = hex::decode(&record.signer_pubkey)?;
    let pk_arr: [u8; 32] = pk_bytes
        .as_slice()
        .try_into()
        .map_err(|_| AttestError::InvalidPublicKey)?;
    let pk = VerifyingKey::from_bytes(&pk_arr).map_err(|_| AttestError::InvalidPublicKey)?;
    let sig_bytes = hex::decode(&record.signature)?;
    let sig_arr: [u8; 64] = sig_bytes
        .as_slice()
        .try_into()
        .map_err(|_| AttestError::InvalidSignature)?;
    let sig = Signature::from_bytes(&sig_arr);

    let body = RecordBody {
        firm_id_hash: &record.firm_id_hash,
        portfolio_commit: &record.portfolio_commit,
        scenario_id: &record.scenario_id,
        model_version: &record.model_version,
        input_hash: &record.input_hash,
        result_hash: &record.result_hash,
        timestamp_unix_ms: record.timestamp_unix_ms,
        signer_pubkey: &record.signer_pubkey,
    };
    let body_bytes = canonical_json(&body)?;
    let mut h = Sha256::new();
    h.update(DOMAIN_RECORD_SIG);
    h.update([0u8]);
    h.update(&body_bytes);
    let digest = h.finalize();
    pk.verify(&digest, &sig)
        .map_err(|_| AttestError::SignatureVerification)?;
    Ok(())
}

/// 32-byte content hash of a record, used as a Merkle leaf.
pub fn record_leaf_hash(record: &AttestationRecord) -> Result<[u8; 32], AttestError> {
    let bytes = canonical_json(record)?;
    let mut h = Sha256::new();
    h.update(b"tyche/v0.1/attestation-leaf");
    h.update([0u8]);
    h.update(&bytes);
    Ok(h.finalize().into())
}

/// A Merkle batch of attestation records.
///
/// Construction: each record is canonicalised, hashed with a leaf domain tag,
/// and inserted as a leaf. The root of the resulting tree is anchored on
/// chain.
pub struct AttestBatch {
    /// The records included in this batch, in insertion order.
    pub records: Vec<AttestationRecord>,
    tree: MerkleTree,
}

impl AttestBatch {
    /// Build a batch from a sequence of records.
    pub fn new(records: Vec<AttestationRecord>) -> Result<Self, AttestError> {
        let leaves: Vec<Vec<u8>> = records
            .iter()
            .map(|r| Ok::<_, AttestError>(record_leaf_hash(r)?.to_vec()))
            .collect::<Result<_, _>>()?;
        let tree = MerkleTree::new(&leaves);
        Ok(Self { records, tree })
    }

    /// 32-byte Merkle root.
    #[must_use]
    pub fn root(&self) -> [u8; 32] {
        self.tree.root()
    }

    /// Hex root.
    #[must_use]
    pub fn root_hex(&self) -> String {
        self.tree.root_hex()
    }

    /// Number of records in the batch.
    #[must_use]
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Whether the batch is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Inclusion proof for record `i`, suitable for off-chain or on-chain
    /// verification.
    pub fn inclusion_proof(
        &self,
        i: usize,
    ) -> Result<tyche_crypto::merkle::MerkleProof, AttestError> {
        let leaf = record_leaf_hash(
            self.records
                .get(i)
                .ok_or_else(|| AttestError::Consistency(format!("index {i} out of range")))?,
        )?;
        self.tree
            .proof(&leaf, i)
            .ok_or_else(|| AttestError::Consistency(format!("proof index {i} unavailable")))
    }
}

/// SHA-256 of arbitrary bytes, hex-encoded. Re-export for convenience so
/// callers don't need a direct `sha2` dependency.
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    hex::encode(h.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;
    use std::collections::BTreeMap;
    use tyche_types::{
        Covenant, CovenantDirection, Geography, Loan, MacroScenario, Portfolio, RiskMetrics,
        Sector, SectorContribution, Seniority,
    };

    fn signing_key() -> SigningKey {
        let mut csprng = OsRng;
        SigningKey::generate(&mut csprng)
    }

    fn portfolio() -> Portfolio {
        Portfolio {
            firm_id: "F-1".into(),
            as_of: "2026-03-31".into(),
            loans: vec![Loan {
                loan_id: "L1".into(),
                issuer: "I1".into(),
                sector: Sector::Healthcare,
                geography: Geography::EuCore,
                seniority: Seniority::SeniorSecured,
                principal: 10.0,
                coupon: 0.07,
                maturity_years: 5.0,
                leverage: 4.0,
                asset_volatility: 0.30,
                covenants: vec![Covenant {
                    name: "x".into(),
                    threshold: 5.0,
                    direction: CovenantDirection::LeqThreshold,
                    cushion: 0.2,
                }],
                collateral_coverage: 1.4,
            }],
        }
    }

    fn scenario() -> MacroScenario {
        MacroScenario {
            scenario_id: "baseline".into(),
            name: "Baseline".into(),
            gdp_shock: -0.01,
            rate_shock_bps: 50.0,
            sector_shocks: BTreeMap::new(),
        }
    }

    fn metrics() -> RiskMetrics {
        let mut sectors = BTreeMap::new();
        sectors.insert(
            Sector::Healthcare,
            SectorContribution {
                expected_loss: 0.5,
                share: 1.0,
            },
        );
        RiskMetrics {
            expected_loss: 0.5,
            var_95: 1.0,
            var_99: 1.5,
            expected_shortfall_975: 1.7,
            sector_contributions: sectors,
            n_paths: 1_000,
        }
    }

    #[test]
    fn record_round_trip_and_verifies() {
        let sk = signing_key();
        let r = build_attestation(&portfolio(), &[7u8; 32], &scenario(), &metrics(), "v0.1.0", &sk)
            .unwrap();
        verify_attestation(&r).unwrap();
        let s = serde_json::to_string(&r).unwrap();
        let r2: AttestationRecord = serde_json::from_str(&s).unwrap();
        assert_eq!(r, r2);
    }

    #[test]
    fn tampering_breaks_signature() {
        let sk = signing_key();
        let mut r = build_attestation(
            &portfolio(),
            &[7u8; 32],
            &scenario(),
            &metrics(),
            "v0.1.0",
            &sk,
        )
        .unwrap();
        r.input_hash = "0".repeat(64);
        assert!(verify_attestation(&r).is_err());
    }

    #[test]
    fn batch_roots_match_inclusion() {
        let sk = signing_key();
        let mut records = Vec::new();
        for _ in 0..7 {
            records.push(
                build_attestation(
                    &portfolio(),
                    &[1u8; 32],
                    &scenario(),
                    &metrics(),
                    "v0.1.0",
                    &sk,
                )
                .unwrap(),
            );
        }
        let batch = AttestBatch::new(records).unwrap();
        let root = batch.root();
        for i in 0..batch.len() {
            let proof = batch.inclusion_proof(i).unwrap();
            tyche_crypto::merkle::verify_merkle(&root, &proof).unwrap();
        }
    }
}
