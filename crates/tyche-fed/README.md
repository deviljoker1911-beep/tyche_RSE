# tyche-fed

Federation-layer skeleton.

In the spike phase this crate exposes the *shape* of the cross-firm aggregator
without the full machinery. It defines:

- `Contribution`: a per-firm contribution carrying a Pedersen commitment and
  a placeholder for a range proof.
- `aggregate(...)`: sums contribution points into an aggregate root, exploiting
  the homomorphism of `tyche-crypto::pedersen`.
- `verify_aggregate_consistency(...)`: stub that today only checks structural
  invariants (firm uniqueness, well-formed points).

Phase 2 will add:

- Bulletproofs+ range proofs (`bulletproofs` crate).
- Threshold signing of the aggregate root (`frost-ed25519`).
- A network protocol for streaming contributions.
