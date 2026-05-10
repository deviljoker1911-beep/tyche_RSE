# tyche-attest

Layer 3 of the Tyche stack: cryptographically-attested simulation records.

An `AttestationRecord` binds together:

- a hash of the firm identifier (`firm_id_hash`),
- a hash commitment to the portfolio inputs (`portfolio_commit`),
- the scenario identifier and model version,
- the input and output content hashes,
- a UNIX-millisecond timestamp,
- the signer's Ed25519 public key,
- and an Ed25519 signature over a canonical-JSON encoding of all of the above.

Records are batched into a Merkle tree (built by `tyche-crypto::merkle`) whose
root is anchored on chain by `AttestationRegistry.sol`. With only the on-chain
root, an auditor with a single `AttestationRecord` and its inclusion proof can
verify it was part of an attested batch — without ever seeing other records.
