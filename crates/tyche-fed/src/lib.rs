//! Federation aggregator skeleton.
//!
//! See the crate-level README. This module intentionally exposes only the
//! API surface that the spike needs; the heavy cryptographic lifting (range
//! proofs, threshold signatures, peer protocol) is tagged with `TODO(phase-2)`
//! markers so that follow-up issues can be opened without reshaping callers.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use tyche_crypto::pedersen::decompress_point;

/// Errors raised by `tyche-fed`.
#[derive(Debug, thiserror::Error)]
pub enum FedError {
    /// A point in a contribution did not decompress to a valid Ristretto element.
    #[error("contribution carries an invalid commitment point")]
    InvalidCommitment,

    /// Two contributions claimed the same firm identity.
    #[error("duplicate firm contribution: {0}")]
    DuplicateFirm(String),

    /// A range proof was missing or malformed (always raised today; placeholder).
    #[error("range proof verification not implemented in spike phase")]
    RangeProofUnimplemented,
}

/// One firm's contribution to an aggregate metric.
///
/// In the spike, `range_proof_bytes` is opaque — callers may pass an empty
/// vector. `verify_aggregate_consistency` does not require it. Phase 2 will
/// require a Bulletproofs+ encoding here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Contribution {
    /// Hex-encoded SHA-256 hash of the firm identifier.
    pub firm_id_hash: String,
    /// Name of the metric being aggregated (e.g. "expected_loss", "var_99").
    pub metric_name: String,
    /// 32-byte compressed Ristretto255 commitment.
    #[serde(with = "hex::serde")]
    pub commitment_bytes: Vec<u8>,
    /// Range-proof bytes. Empty in the spike.
    #[serde(with = "hex::serde")]
    pub range_proof_bytes: Vec<u8>,
}

/// Aggregate of a set of contributions for a single metric.
///
/// `root_bytes` is the compressed point of `Σ c_i`, where `c_i` is the
/// commitment carried by each contribution. The set of `firm_id_hash`es is
/// recorded to make the aggregate auditable downstream.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AggregateRoot {
    /// The metric being aggregated.
    pub metric_name: String,
    /// 32-byte compressed Ristretto255 sum.
    #[serde(with = "hex::serde")]
    pub root_bytes: Vec<u8>,
    /// Hashes of contributing firms, sorted lexicographically.
    pub contributors: Vec<String>,
}

/// Sum a homogenous set of contributions into an aggregate root.
///
/// All `metric_name` fields must match. Returns the aggregate point and the
/// sorted list of contributing firm-id hashes.
pub fn aggregate(contributions: &[Contribution]) -> Result<AggregateRoot, FedError> {
    if contributions.is_empty() {
        return Ok(AggregateRoot {
            metric_name: String::new(),
            root_bytes: vec![0u8; 32],
            contributors: vec![],
        });
    }
    let metric = &contributions[0].metric_name;
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    let mut points = Vec::with_capacity(contributions.len());
    for c in contributions {
        if c.metric_name != *metric {
            return Err(FedError::DuplicateFirm(format!(
                "mixed metric names: {} vs {}",
                metric, c.metric_name
            )));
        }
        if !seen.insert(c.firm_id_hash.as_str()) {
            return Err(FedError::DuplicateFirm(c.firm_id_hash.clone()));
        }
        let arr: [u8; 32] = c
            .commitment_bytes
            .as_slice()
            .try_into()
            .map_err(|_| FedError::InvalidCommitment)?;
        let p = decompress_point(&arr).map_err(|_| FedError::InvalidCommitment)?;
        points.push(p);
    }
    let sum = tyche_crypto::pedersen_add_commitments(&points);
    let root_bytes = sum.compress().to_bytes().to_vec();
    let mut contributors: Vec<String> = seen.iter().map(|s| (*s).to_string()).collect();
    contributors.sort();
    Ok(AggregateRoot {
        metric_name: metric.clone(),
        root_bytes,
        contributors,
    })
}

/// Structural-only consistency check for an aggregate (spike).
///
/// Today this verifies:
///
/// - every contribution has a non-empty `firm_id_hash`,
/// - every commitment decompresses to a valid Ristretto element,
/// - no firm appears twice.
///
/// Phase 2 will additionally verify that each `range_proof_bytes` proves the
/// contribution lies in `[0, 2^64)`, and that the sum of commitments matches
/// the claimed plaintext aggregate published alongside the root.
pub fn verify_aggregate_consistency(contributions: &[Contribution]) -> Result<(), FedError> {
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for c in contributions {
        if c.firm_id_hash.is_empty() {
            return Err(FedError::DuplicateFirm("empty firm_id_hash".into()));
        }
        if !seen.insert(c.firm_id_hash.as_str()) {
            return Err(FedError::DuplicateFirm(c.firm_id_hash.clone()));
        }
        let arr: [u8; 32] = c
            .commitment_bytes
            .as_slice()
            .try_into()
            .map_err(|_| FedError::InvalidCommitment)?;
        decompress_point(&arr).map_err(|_| FedError::InvalidCommitment)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use curve25519_dalek::scalar::Scalar;
    use tyche_crypto::pedersen::pedersen_commit;

    fn contribution(firm: &str, value: u64) -> Contribution {
        let cm = pedersen_commit(Scalar::from(value), None);
        Contribution {
            firm_id_hash: format!("hash-{firm}"),
            metric_name: "expected_loss".into(),
            commitment_bytes: cm.to_bytes().to_vec(),
            range_proof_bytes: vec![],
        }
    }

    #[test]
    fn aggregate_two_contributions() {
        let cs = vec![contribution("A", 100), contribution("B", 250)];
        let root = aggregate(&cs).unwrap();
        assert_eq!(root.contributors.len(), 2);
        assert_eq!(root.root_bytes.len(), 32);
    }

    #[test]
    fn duplicate_firm_rejected() {
        let cs = vec![contribution("A", 100), contribution("A", 250)];
        assert!(aggregate(&cs).is_err());
    }

    #[test]
    fn empty_aggregate_is_well_defined() {
        let root = aggregate(&[]).unwrap();
        assert_eq!(root.root_bytes.len(), 32);
    }

    #[test]
    fn structural_check_catches_invalid_point() {
        let bad = Contribution {
            firm_id_hash: "x".into(),
            metric_name: "y".into(),
            commitment_bytes: vec![0xFFu8; 32],
            range_proof_bytes: vec![],
        };
        assert!(verify_aggregate_consistency(std::slice::from_ref(&bad)).is_err());
    }
}
