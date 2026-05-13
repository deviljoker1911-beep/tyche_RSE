//! Binary SHA-256 Merkle trees with deterministic odd-leaf duplication.
//!
//! Construction:
//!
//! - `leaf(x)   = SHA-256(DOMAIN_MERKLE_LEAF || 0x00 || x)`
//! - `node(l,r) = SHA-256(DOMAIN_MERKLE_NODE || 0x00 || l || r)`
//! - When a level has an odd number of nodes, the last node is duplicated
//!   (paired with itself) before hashing the next level.
//!
//! Domain separation between leaves and internal nodes prevents
//! second-preimage attacks where an attacker re-interprets an internal node
//! as a leaf. Both tags are defined in [`crate`].
//!
//! # Verifier compatibility
//!
//! The Solidity `AttestationRegistry` uses the OpenZeppelin `MerkleProof`
//! library, which expects raw `keccak256` hashes without a domain prefix.
//! Anchored roots therefore use this Rust tree's *raw* leaf hashes (i.e. the
//! caller is expected to feed already-hashed leaves into the on-chain
//! verifier and to commit to a Merkle tree built with matching hash and
//! ordering rules). The on-chain side of that integration lives in
//! `tyche-attest::onchain`.

use sha2::{Digest, Sha256};

use crate::{CryptoError, DOMAIN_MERKLE_LEAF, DOMAIN_MERKLE_NODE};

/// A Merkle tree.
///
/// Construction is eager: levels are precomputed at `MerkleTree::new` time.
/// This is fine for the spike — even a 100k-leaf tree uses <10MB of memory.
#[derive(Debug, Clone)]
pub struct MerkleTree {
    /// `levels[0]` is the leaf level, `levels[levels.len()-1]` is `[root]`.
    levels: Vec<Vec<[u8; 32]>>,
}

/// An inclusion proof for a single leaf.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MerkleProof {
    /// The leaf bytes (pre-hash). The verifier hashes these with the leaf
    /// domain tag.
    pub leaf: Vec<u8>,
    /// Index of the leaf in the original `leaves` slice.
    pub index: usize,
    /// Sibling hashes, ordered from leaf-level to root-level.
    pub siblings: Vec<[u8; 32]>,
}

impl MerkleTree {
    /// Build a tree from raw leaf bytes. The empty case yields a single
    /// `SHA-256(DOMAIN_MERKLE_LEAF || 0x00)` root, matching the convention
    /// that an empty root is still a well-defined value.
    #[must_use]
    pub fn new(leaves: &[Vec<u8>]) -> Self {
        let leaf_hashes: Vec<[u8; 32]> = leaves.iter().map(|l| hash_leaf(l)).collect();
        let mut levels: Vec<Vec<[u8; 32]>> = Vec::new();
        levels.push(if leaf_hashes.is_empty() {
            vec![hash_leaf(&[])]
        } else {
            leaf_hashes
        });

        while levels.last().map_or(0, Vec::len) > 1 {
            let prev = levels.last().expect("non-empty by loop guard");
            let mut next: Vec<[u8; 32]> = Vec::with_capacity(prev.len().div_ceil(2));
            let mut i = 0;
            while i < prev.len() {
                let l = prev[i];
                let r = if i + 1 < prev.len() { prev[i + 1] } else { l };
                next.push(hash_node(&l, &r));
                i += 2;
            }
            levels.push(next);
        }

        Self { levels }
    }

    /// 32-byte root hash.
    #[must_use]
    pub fn root(&self) -> [u8; 32] {
        *self
            .levels
            .last()
            .and_then(|l| l.first())
            .expect("tree always has at least one level")
    }

    /// Lowercase hex root.
    #[must_use]
    pub fn root_hex(&self) -> String {
        hex::encode(self.root())
    }

    /// Number of leaves in the original input.
    #[must_use]
    pub fn leaf_count(&self) -> usize {
        self.levels.first().map_or(0, Vec::len)
    }

    /// Build an inclusion proof for the leaf at `index`.
    ///
    /// Returns `None` if `index` is out of range.
    #[must_use]
    pub fn proof(&self, leaf: &[u8], index: usize) -> Option<MerkleProof> {
        if index >= self.leaf_count() {
            return None;
        }
        let mut siblings = Vec::with_capacity(self.levels.len().saturating_sub(1));
        let mut idx = index;
        for level in 0..self.levels.len() - 1 {
            let row = &self.levels[level];
            let sibling_idx = if idx % 2 == 0 {
                if idx + 1 < row.len() { idx + 1 } else { idx }
            } else {
                idx - 1
            };
            siblings.push(row[sibling_idx]);
            idx /= 2;
        }
        Some(MerkleProof {
            leaf: leaf.to_vec(),
            index,
            siblings,
        })
    }
}

/// Verify an inclusion proof against `root`.
pub fn verify_merkle(root: &[u8; 32], proof: &MerkleProof) -> Result<(), CryptoError> {
    let mut hash = hash_leaf(&proof.leaf);
    let mut idx = proof.index;
    for sibling in &proof.siblings {
        let (l, r) = if idx % 2 == 0 {
            (hash, *sibling)
        } else {
            (*sibling, hash)
        };
        hash = hash_node(&l, &r);
        idx /= 2;
    }
    if hash == *root {
        Ok(())
    } else {
        Err(CryptoError::MerkleVerification)
    }
}

fn hash_leaf(bytes: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(DOMAIN_MERKLE_LEAF);
    h.update([0u8]);
    h.update(bytes);
    h.finalize().into()
}

fn hash_node(l: &[u8; 32], r: &[u8; 32]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(DOMAIN_MERKLE_NODE);
    h.update([0u8]);
    h.update(l);
    h.update(r);
    h.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn leaves_n(n: usize) -> Vec<Vec<u8>> {
        (0..n).map(|i| format!("leaf-{i}").into_bytes()).collect()
    }

    #[test]
    fn single_leaf_tree_has_leaf_hash_as_root() {
        let leaves = leaves_n(1);
        let t = MerkleTree::new(&leaves);
        assert_eq!(t.root(), hash_leaf(&leaves[0]));
    }

    #[test]
    fn root_is_stable() {
        let a = MerkleTree::new(&leaves_n(8)).root_hex();
        let b = MerkleTree::new(&leaves_n(8)).root_hex();
        assert_eq!(a, b);
    }

    #[test]
    fn changing_a_leaf_changes_root() {
        let mut leaves = leaves_n(8);
        let a = MerkleTree::new(&leaves).root();
        leaves[3] = b"different".to_vec();
        let b = MerkleTree::new(&leaves).root();
        assert_ne!(a, b);
    }

    #[test]
    fn odd_leaf_duplication_works() {
        // A 3-leaf tree should still produce a well-formed root and all three
        // leaves should verify against it.
        let leaves = leaves_n(3);
        let t = MerkleTree::new(&leaves);
        for (i, l) in leaves.iter().enumerate() {
            let p = t.proof(l, i).unwrap();
            verify_merkle(&t.root(), &p).unwrap();
        }
    }

    #[test]
    fn proof_for_every_leaf_in_1024_tree() {
        let leaves = leaves_n(1024);
        let t = MerkleTree::new(&leaves);
        let root = t.root();
        for (i, l) in leaves.iter().enumerate() {
            let p = t.proof(l, i).unwrap();
            verify_merkle(&root, &p).expect("proof for leaf should verify");
        }
    }

    #[test]
    fn tampered_proof_fails() {
        let leaves = leaves_n(16);
        let t = MerkleTree::new(&leaves);
        let mut p = t.proof(&leaves[5], 5).unwrap();
        p.siblings[0][0] ^= 0xFF;
        assert!(verify_merkle(&t.root(), &p).is_err());
    }

    proptest! {
        // Bumped to 10_000 cases per M1A. Merkle inclusion is critical for
        // attestation verification; we want every off-by-one and odd-leaf
        // corner case shaken out.
        #![proptest_config(ProptestConfig {
            cases: if std::env::var("TYCHE_PROPTEST_FAST").is_ok() { 32 } else { 10_000 },
            .. ProptestConfig::default()
        })]

        #[test]
        fn proofs_verify_for_arbitrary_size(n in 1usize..256) {
            let leaves = leaves_n(n);
            let t = MerkleTree::new(&leaves);
            let root = t.root();
            for (i, l) in leaves.iter().enumerate() {
                let p = t.proof(l, i).unwrap();
                prop_assert!(verify_merkle(&root, &p).is_ok());
            }
        }
    }
}
