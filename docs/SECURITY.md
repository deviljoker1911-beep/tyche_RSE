# Tyche security policy

This document describes the security posture of the Tyche spike. It is
deliberately blunt about what the spike is and is not.

## What the spike is

- A reference implementation of L1 (data graph), L2 (simulation), L3
  (attestation), with skeletons for L4 (federation) and L5 (UX).
- Compiled from audited cryptographic libraries (`curve25519-dalek`,
  `ed25519-dalek`, `sha2`) and a hand-rolled SHA-256 Merkle tree with
  domain-separated leaf and node hashes.
- Wired to a local Anvil chain. There is no mainnet deployment.

## What the spike is **not**

- **Not audited.** The whitepaper anticipates a Trail of Bits or Zellic audit
  before any mainnet deployment. Until that audit lands, do not use Tyche to
  attest production data.
- **Not range-proven.** The federation aggregator (`tyche-fed`) is a
  skeleton. `Contribution.range_proof_bytes` is not verified. A malicious
  firm could publish a commitment to a value outside `[0, 2^64)` and
  corrupt aggregates. Phase 2 will integrate Bulletproofs+.
- **Not threshold-signed.** The on-chain registry uses single-publisher
  access control. Phase 2 will add FROST/Ed25519 threshold signing.
- **Not GDPR-reviewed.** Even though `firm_id_hash` is a hash and portfolio
  data never leaves the firm in our flows, organisations adopting Tyche
  must run their own DPIA before deploying.

## Cryptographic primitives

| Primitive             | Library                        | Domain tag                    |
|-----------------------|--------------------------------|-------------------------------|
| SHA-256               | `sha2`                         | (per use, see below)          |
| Hash commitment       | `tyche-crypto::hash_commit`    | `tyche/v0.1/hash-commit`      |
| Pedersen commitment   | `curve25519-dalek` (Ristretto) | `tyche/v0.1/pedersen-h` (gen) |
| Merkle leaf           | SHA-256                        | `tyche/v0.1/merkle-leaf`      |
| Merkle internal node  | SHA-256                        | `tyche/v0.1/merkle-node`      |
| Attestation signature | `ed25519-dalek`                | `tyche/v0.1/attestation-record` |
| Object hash           | SHA-256 over canonical JSON    | `tyche/v0.1/object`           |

Domain separation is mandatory in every hash. A change of tag is a breaking
change and requires a `model_version` bump on attestation records.

## Threat model (high level)

1. **Tampering with a published attestation record.** Mitigation: Ed25519
   signature over the canonical body, anchored Merkle root.
2. **Replay across firms.** Mitigation: `firm_id_hash` plus `signer_pubkey`
   are part of the signed body.
3. **Cross-domain hash collision (e.g. portfolio hash equals attestation
   hash).** Mitigation: domain-separated hash inputs everywhere.
4. **Federation aggregate manipulation by a malicious firm.** Partially
   mitigated by Pedersen homomorphism + `verify_aggregate_consistency`;
   **not** fully mitigated without range proofs.
5. **Smart-contract compromise.** Mitigation: minimal contract surface, OZ
   `AccessControl`, Foundry tests, Slither static analysis in CI. Audit
   required before mainnet.

## Reporting a vulnerability

Email `security@tyche.network` with a description and (if you have one) a
proof of concept. We aim to acknowledge within 48 hours. Please **do not**
open a public issue for security findings.

## Disclosure policy

We follow a coordinated-disclosure model. We will:

- acknowledge within 48 hours,
- assign a severity (CVSSv3.1) within 7 days,
- patch and publish a security advisory before public disclosure.
