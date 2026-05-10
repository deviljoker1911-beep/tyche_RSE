# tyche-crypto

The cryptographic primitives Tyche depends on:

- **`hash_commit`** — domain-separated SHA-256 hash commitments with optional 32-byte blinding.
- **`pedersen`** — Pedersen commitments over the Ristretto255 group, with a hash-to-curve generator `H`. Provides homomorphic addition.
- **`merkle`** — SHA-256 binary Merkle trees with deterministic odd-leaf duplication and inclusion-proof verification.

This crate intentionally restricts itself to audited primitives from
`curve25519-dalek` and `sha2`. It does not roll its own field arithmetic.
