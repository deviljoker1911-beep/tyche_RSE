//! Pedersen commitments over Ristretto255.
//!
//! Construction: `C = v·G + r·H`, where:
//!
//! - `G` is the canonical Ristretto255 basepoint.
//! - `H` is a second generator derived deterministically by hashing the
//!   domain tag [`crate::DOMAIN_PEDERSEN_H`] to a Ristretto group element via
//!   `RistrettoPoint::hash_from_bytes::<Sha512>`. This guarantees the
//!   discrete-log relation between `G` and `H` is unknown.
//!
//! The commitment is **homomorphic**:
//!
//! ```text
//! commit(v1, r1) + commit(v2, r2) == commit(v1 + v2, r1 + r2)
//! ```
//!
//! This enables network-level aggregation of metric commitments without ever
//! revealing the per-firm contributions. Range proofs are deferred to Phase 2
//! (`bulletproofs`); the spike layer only verifies sums against a known total.

use curve25519_dalek::constants::RISTRETTO_BASEPOINT_POINT;
use curve25519_dalek::ristretto::{CompressedRistretto, RistrettoPoint};
use curve25519_dalek::scalar::Scalar;
use rand::RngCore;
use sha2::Sha512;

use crate::{CryptoError, DOMAIN_PEDERSEN_H};

/// Lazily-evaluated Pedersen second generator `H`.
///
/// Derived from [`DOMAIN_PEDERSEN_H`] via `hash_from_bytes::<Sha512>` so that
/// nobody (including the implementer) knows `log_G(H)`.
fn h_generator() -> RistrettoPoint {
    // SECURITY: hash_from_bytes::<Sha512> uses Elligator2-style mapping that
    // produces a uniformly distributed Ristretto element. We re-derive on
    // every call rather than caching to avoid sharing mutable state across
    // threads — the cost is one SHA-512 + one curve op, both cheap.
    RistrettoPoint::hash_from_bytes::<Sha512>(DOMAIN_PEDERSEN_H)
}

/// A Pedersen commitment together with the secrets needed to open it.
///
/// In a real protocol the public side passes the compressed point alone; the
/// committer keeps `value` and `blinding`.
#[derive(Debug, Clone)]
pub struct PedersenCommitment {
    /// The commitment point `C = v·G + r·H`.
    pub c: RistrettoPoint,
    /// The committed value (committer-side only).
    pub value: Scalar,
    /// The blinding factor (committer-side only).
    pub blinding: Scalar,
}

impl PedersenCommitment {
    /// Compressed 32-byte encoding of the commitment point.
    #[must_use]
    pub fn to_bytes(&self) -> [u8; 32] {
        self.c.compress().to_bytes()
    }
}

/// Encode a `u64` value as a Pedersen-friendly scalar.
#[must_use]
pub fn scalar_from_u64(v: u64) -> Scalar {
    Scalar::from(v)
}

/// Sample a uniformly random Ristretto scalar.
fn random_scalar() -> Scalar {
    let mut bytes = [0u8; 64];
    rand::thread_rng().fill_bytes(&mut bytes);
    Scalar::from_bytes_mod_order_wide(&bytes)
}

/// Commit to `value` with an optional explicit `blinding`.
///
/// If `blinding` is `None`, a fresh random scalar is sampled.
#[must_use]
pub fn pedersen_commit(value: Scalar, blinding: Option<Scalar>) -> PedersenCommitment {
    let r = blinding.unwrap_or_else(random_scalar);
    let h = h_generator();
    let c = RISTRETTO_BASEPOINT_POINT * value + h * r;
    PedersenCommitment {
        c,
        value,
        blinding: r,
    }
}

/// Sum a list of commitment points. Useful for aggregating contributions
/// across firms in the federation layer.
#[must_use]
pub fn pedersen_add_commitments(points: &[RistrettoPoint]) -> RistrettoPoint {
    points
        .iter()
        .copied()
        .fold(RistrettoPoint::default(), |acc, p| acc + p)
}

/// Verify that a commitment `c` opens to `(value, blinding)`.
#[must_use]
pub fn pedersen_verify(c: RistrettoPoint, value: Scalar, blinding: Scalar) -> bool {
    let h = h_generator();
    let recomputed = RISTRETTO_BASEPOINT_POINT * value + h * blinding;
    recomputed == c
}

/// Decompress a 32-byte compressed Ristretto point.
pub fn decompress_point(bytes: &[u8; 32]) -> Result<RistrettoPoint, CryptoError> {
    CompressedRistretto::from_slice(bytes)
        .map_err(|_| CryptoError::InvalidPoint)?
        .decompress()
        .ok_or(CryptoError::InvalidPoint)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn commit_and_verify() {
        let cm = pedersen_commit(Scalar::from(42u64), None);
        assert!(pedersen_verify(cm.c, cm.value, cm.blinding));
    }

    #[test]
    fn different_blindings_give_different_commitments() {
        let v = Scalar::from(7u64);
        let c1 = pedersen_commit(v, Some(Scalar::from(11u64)));
        let c2 = pedersen_commit(v, Some(Scalar::from(13u64)));
        assert_ne!(c1.c, c2.c);
    }

    #[test]
    fn h_is_not_basepoint() {
        // Sanity: H != G. If this is ever false, the hash-to-curve construction
        // is silently broken and the Pedersen scheme is no longer hiding.
        assert_ne!(h_generator(), RISTRETTO_BASEPOINT_POINT);
    }

    #[test]
    fn compress_decompress_round_trip() {
        let cm = pedersen_commit(Scalar::from(3u64), None);
        let bytes = cm.to_bytes();
        let p = decompress_point(&bytes).unwrap();
        assert_eq!(p, cm.c);
    }

    proptest! {
        // Bumped to 10_000 cases per M1A. Pedersen homomorphism is the
        // load-bearing invariant of the federation layer; we want a tight
        // probabilistic guarantee, not just a smoke test.
        // The TYCHE_PROPTEST_FAST=1 escape lets local dev cut this back.
        #![proptest_config(ProptestConfig {
            cases: if std::env::var("TYCHE_PROPTEST_FAST").is_ok() { 64 } else { 10_000 },
            .. ProptestConfig::default()
        })]

        #[test]
        fn homomorphism(a in 0u64..1_000_000, b in 0u64..1_000_000) {
            let r1 = random_scalar();
            let r2 = random_scalar();
            let c1 = pedersen_commit(Scalar::from(a), Some(r1));
            let c2 = pedersen_commit(Scalar::from(b), Some(r2));
            let summed = c1.c + c2.c;
            let combined = pedersen_commit(Scalar::from(a + b), Some(r1 + r2));
            prop_assert_eq!(summed, combined.c);
            prop_assert!(pedersen_verify(summed, Scalar::from(a + b), r1 + r2));
        }
    }
}
