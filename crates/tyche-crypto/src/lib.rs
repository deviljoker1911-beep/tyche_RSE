//! Cryptographic primitives used by the Tyche stack.
//!
//! This crate provides three submodules:
//!
//! - [`hash_commit`] — SHA-256 hash commitments with optional 32-byte blinding.
//! - [`pedersen`] — Pedersen commitments on Ristretto255 with homomorphic
//!   addition.
//! - [`merkle`] — Binary SHA-256 Merkle trees with inclusion proofs.
//!
//! All hashes are domain-separated. See [`DOMAIN_HASH_COMMIT`],
//! [`DOMAIN_PEDERSEN_H`], [`DOMAIN_MERKLE_LEAF`], and [`DOMAIN_MERKLE_NODE`].

#![forbid(unsafe_code)]

pub mod hash_commit;
pub mod merkle;
pub mod pedersen;

/// Domain tag for a hash commitment.
pub const DOMAIN_HASH_COMMIT: &[u8] = b"tyche/v0.1/hash-commit";

/// Domain tag used to derive the Pedersen second generator `H`.
pub const DOMAIN_PEDERSEN_H: &[u8] = b"tyche/v0.1/pedersen-h";

/// Domain tag prepended to every Merkle leaf hash.
pub const DOMAIN_MERKLE_LEAF: &[u8] = b"tyche/v0.1/merkle-leaf";

/// Domain tag prepended to every Merkle internal node hash.
pub const DOMAIN_MERKLE_NODE: &[u8] = b"tyche/v0.1/merkle-node";

/// Errors raised by `tyche-crypto`.
#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    /// Hex decoding failed.
    #[error("hex decode error: {0}")]
    Hex(#[from] hex::FromHexError),

    /// A byte slice did not have the expected length for a primitive.
    #[error("invalid length: expected {expected}, got {got}")]
    InvalidLength {
        /// Expected number of bytes.
        expected: usize,
        /// Actual number of bytes.
        got: usize,
    },

    /// A point was not a valid Ristretto255 group element.
    #[error("invalid ristretto point")]
    InvalidPoint,

    /// A scalar was not canonical.
    #[error("invalid scalar")]
    InvalidScalar,

    /// A Merkle proof did not authenticate its claimed root.
    #[error("merkle verification failed")]
    MerkleVerification,
}

pub use crate::hash_commit::{commit_hash, verify_hash_commitment};
pub use crate::merkle::{MerkleProof, MerkleTree, verify_merkle};
pub use crate::pedersen::{
    PedersenCommitment, pedersen_add_commitments, pedersen_commit, pedersen_verify,
};
