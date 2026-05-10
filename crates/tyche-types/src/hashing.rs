//! Hashing helpers built on canonical JSON.

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::TypesError;
use crate::canonical::canonical_json;

/// Domain separation tag prepended to every Tyche content hash.
///
/// Rationale: ensures that a hash of a portfolio cannot be confused with a
/// hash of, e.g., an attestation record, even if their canonical-JSON forms
/// happen to collide byte-for-byte. The tag includes a version so we can
/// rotate it across breaking schema changes.
pub const HASH_DOMAIN: &[u8] = b"tyche/v0.1/object";

/// SHA-256 over arbitrary bytes, returned as lowercase hex.
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

/// Hash any serialisable value via canonical JSON + SHA-256, with the Tyche
/// object domain tag. Returns lowercase hex.
pub fn hash_object<T: Serialize>(value: &T) -> Result<String, TypesError> {
    let body = canonical_json(value)?;
    let mut hasher = Sha256::new();
    hasher.update(HASH_DOMAIN);
    hasher.update([0u8]); // separator byte between tag and body
    hasher.update(&body);
    Ok(hex::encode(hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn sha256_matches_known_vector() {
        // SHA-256("abc") = ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn hash_is_stable_across_key_order() {
        let a = json!({"a": 1, "b": 2});
        let b = json!({"b": 2, "a": 1});
        assert_eq!(hash_object(&a).unwrap(), hash_object(&b).unwrap());
    }

    #[test]
    fn hash_is_unique_across_distinct_inputs() {
        let a = json!({"a": 1});
        let b = json!({"a": 2});
        assert_ne!(hash_object(&a).unwrap(), hash_object(&b).unwrap());
    }

    #[test]
    fn hash_includes_domain_tag() {
        // A bare canonical-JSON SHA-256 should not collide with the
        // domain-tagged version, otherwise the tag is being ignored.
        let v = json!({"a": 1});
        let body = canonical_json(&v).unwrap();
        let untagged = sha256_hex(&body);
        let tagged = hash_object(&v).unwrap();
        assert_ne!(untagged, tagged);
    }
}
