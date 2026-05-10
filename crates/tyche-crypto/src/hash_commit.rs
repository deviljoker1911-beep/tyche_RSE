//! Hash commitments.
//!
//! A hash commitment binds a committer to a value `v` while hiding it from a
//! verifier. The committer reveals `(v, r)` later; the verifier recomputes the
//! commitment and checks for equality.
//!
//! Construction: `c = SHA-256(domain || 0x00 || r || 0x00 || v)` where `r` is
//! a 32-byte blinding factor. If the caller does not provide `r`, one is
//! sampled from the OS RNG.
//!
//! Equality of commitments is checked with [`subtle::ConstantTimeEq`] to avoid
//! timing leaks on the verifier side.

use rand::RngCore;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

use crate::{CryptoError, DOMAIN_HASH_COMMIT};

/// Produce a hash commitment to `value` with the given (or random) blinding.
///
/// Returns `(commitment_hex, blinding_bytes)`. The caller must persist
/// `blinding_bytes` to later open the commitment.
#[must_use]
pub fn commit_hash(value: &[u8], blinding: Option<[u8; 32]>) -> (String, [u8; 32]) {
    let r = blinding.unwrap_or_else(|| {
        let mut buf = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut buf);
        buf
    });
    let mut hasher = Sha256::new();
    hasher.update(DOMAIN_HASH_COMMIT);
    hasher.update([0u8]);
    hasher.update(r);
    hasher.update([0u8]);
    hasher.update(value);
    (hex::encode(hasher.finalize()), r)
}

/// Verify a hash commitment in constant time.
pub fn verify_hash_commitment(
    commitment_hex: &str,
    value: &[u8],
    blinding: &[u8; 32],
) -> Result<bool, CryptoError> {
    let claimed = hex::decode(commitment_hex)?;
    if claimed.len() != 32 {
        return Err(CryptoError::InvalidLength {
            expected: 32,
            got: claimed.len(),
        });
    }
    let (recomputed_hex, _) = commit_hash(value, Some(*blinding));
    let recomputed = hex::decode(recomputed_hex)?;
    Ok(recomputed.ct_eq(&claimed).into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commitment_round_trips() {
        let v = b"net_leverage=4.5";
        let (c, r) = commit_hash(v, None);
        assert!(verify_hash_commitment(&c, v, &r).unwrap());
    }

    #[test]
    fn commitment_with_explicit_blinding_is_deterministic() {
        let v = b"x";
        let r = [7u8; 32];
        let (c1, _) = commit_hash(v, Some(r));
        let (c2, _) = commit_hash(v, Some(r));
        assert_eq!(c1, c2);
    }

    #[test]
    fn tamper_with_value_fails_verification() {
        let v = b"x";
        let (c, r) = commit_hash(v, None);
        assert!(!verify_hash_commitment(&c, b"y", &r).unwrap());
    }

    #[test]
    fn tamper_with_blinding_fails_verification() {
        let v = b"x";
        let (c, mut r) = commit_hash(v, None);
        r[0] ^= 0xFF;
        assert!(!verify_hash_commitment(&c, v, &r).unwrap());
    }

    #[test]
    fn malformed_hex_fails_cleanly() {
        assert!(verify_hash_commitment("zz", b"x", &[0u8; 32]).is_err());
    }
}
