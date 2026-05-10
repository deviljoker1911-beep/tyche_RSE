# Tyche architecture

Tyche is organised in five layers. Each layer makes a clean assumption about
the layer beneath it and exposes a narrow, typed contract upward.

```
┌───────────────────────────────────────────────────────────────┐
│ L5  Workflow / UX                                              │
│     Web dashboard, CLI, mobile (Phase 2)                       │
├───────────────────────────────────────────────────────────────┤
│ L4  Federation                                                 │
│     Cross-firm aggregator with Pedersen commitments            │
├───────────────────────────────────────────────────────────────┤
│ L3  Attestation                                                │
│     Signed records, Merkle batching, on-chain anchor           │
├───────────────────────────────────────────────────────────────┤
│ L2  Simulation                                                 │
│     Structural Monte Carlo, factor model, recovery model       │
├───────────────────────────────────────────────────────────────┤
│ L1  Data graph                                                 │
│     Loans, covenants, portfolios, scenarios, canonical hashes  │
└───────────────────────────────────────────────────────────────┘
```

## L1 — data graph (`tyche-types`)

Defines the wire types: `Loan`, `Covenant`, `Portfolio`, `MacroScenario`,
`RiskMetrics`. All types are `Serialize + Deserialize`. The crate provides a
`canonical_json` encoder (sorted keys, no whitespace, deterministic float
formatting) and a `hash_object` helper that produces a SHA-256 commitment to
any value with a domain-separated tag.

Every other crate consumes these types. They are the schema that downstream
hashes commit to; a breaking change requires a `model_version` bump on the
attestation side.

## L2 — simulation (`tyche-sim`)

A structural-credit Monte Carlo. Default occurs when the standardised firm
asset innovation crosses a leverage-derived threshold (Merton-style). A
covenant-breach channel models acceleration when scenario shocks erode
cushion past zero. Recovery is seniority-floored and adjusted for collateral
coverage and the scenario's asset shock.

Correlation is induced through a three-factor decomposition:

```
z_i = sqrt(rho_m) z_market + sqrt(rho_s) z_sector + sqrt(1 - rho_m - rho_s) z_idio
```

The simulator is **deterministic in `(portfolio, scenario, config)`**: same
seed and same canonical-JSON inputs produce byte-identical metrics. This is
what makes attestation possible.

## L3 — attestation (`tyche-attest` + `contracts/`)

`AttestationRecord` binds:

- `firm_id_hash` — SHA-256 of firm identifier
- `portfolio_commit` — domain-separated hash commitment with a 32-byte blinding
- `scenario_id`, `model_version`
- `input_hash` — content hash of the canonical-JSON inputs
- `result_hash` — content hash of the canonical-JSON outputs
- `timestamp_unix_ms`
- `signer_pubkey`, `signature` (Ed25519 over the canonical body)

Records are batched. Each batch is a SHA-256 binary Merkle tree (with odd-leaf
duplication and leaf/node domain separation). The 32-byte root is anchored
on chain by `AttestationRegistry.publishRoot`.

A verifier with the on-chain root and an inclusion proof can confirm a
particular record was part of an attested batch — without seeing any of the
other records in the batch.

## L4 — federation (`tyche-fed`, skeleton)

Per-firm contributions to a network metric (e.g. industry-wide expected loss)
are encoded as Pedersen commitments on Ristretto255. The aggregator sums the
commitment points (homomorphic addition) and publishes the resulting
`AggregateRoot`. In Phase 2 each contribution will additionally carry a
Bulletproofs+ range proof so the aggregate is meaningful without trust.

## L5 — workflow / UX (`apps/web` + `tyche-cli`)

The web dashboard runs the simulation entirely in-browser via the WASM
binding to `tyche-sim`. The CLI is the canonical reference surface — the WASM
and Python bindings expose strict subsets of the same API.

## Data flow at run time

```
portfolio.json ──┐
scenario.json   ──┼──► simulate ──► RiskMetrics
config           ┘                       │
                                          ├──► attestation record (signed)
                                          │         │
                                          │         └──► batch ──► merkle root ──► chain
                                          │
                                          └──► federation contribution (committed)
                                                    │
                                                    └──► aggregate ──► aggregate root
```

## Reproducibility

The system stands or falls on reproducibility. The contract is:

- `canonical_json` is total and stable across runs and across map insertion
  order.
- `tyche-sim::simulate` is deterministic in `(portfolio, scenario, config)`.
- `tyche-crypto::merkle::MerkleTree` is deterministic in its leaves.
- `tyche-attest::build_attestation` is deterministic except for
  `timestamp_unix_ms` and the random blinding (both injected by the caller
  for reproducible runs).

If any of these breaks, attestation breaks. Property tests under each crate's
`tests/` directory and `#[cfg(test)] mod tests` blocks are the regression
line.
