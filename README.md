# Tyche

Federated, cryptographically-attested risk simulation for European private credit.

[![rust ci](https://github.com/tyche-org/tyche/actions/workflows/rust.yml/badge.svg)](https://github.com/tyche-org/tyche/actions/workflows/rust.yml)
[![solidity ci](https://github.com/tyche-org/tyche/actions/workflows/solidity.yml/badge.svg)](https://github.com/tyche-org/tyche/actions/workflows/solidity.yml)
[![web ci](https://github.com/tyche-org/tyche/actions/workflows/web.yml/badge.svg)](https://github.com/tyche-org/tyche/actions/workflows/web.yml)
[![license](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)

Tyche is a local-first, federated risk-simulation engine for private-market
portfolios. Three architectural ideas:

- **Local-first simulation.** Portfolio data never leaves the firm. The
  Monte Carlo core is a Rust crate that runs on a server, in a browser via
  WebAssembly, and (Phase 2) on mobile via native bindings.
- **Cryptographic attestation.** Every simulation run produces a signed,
  hash-committed record. Records are batched into a Merkle tree whose root is
  anchored on a public chain. An auditor with a single record and inclusion
  proof can verify the run took place — without seeing any other firm's data.
- **Federated aggregation.** Pedersen commitments make industry-wide
  aggregates auditable without revealing individual firm contributions.

This repository is the **spike-phase reference implementation** referenced in
the Tyche whitepaper.

## Quick start

```sh
git clone https://github.com/tyche-org/tyche
cd tyche

# 1. dev environment (optional but reproducible)
nix develop  # or install rust, foundry, pnpm, wasm-pack manually

# 2. local stack: anvil + contracts + wasm + web
scripts/dev.sh

# 3. end-to-end checks
scripts/e2e.sh
```

Open <http://localhost:3000> for the dashboard. Drive the CLI with `tyche
--help`.

## Architecture at a glance

| Layer | What it does                                | Crates / paths                             |
|------:|---------------------------------------------|--------------------------------------------|
| L5    | Workflow / UX (web dashboard, CLI)          | `apps/web`, `crates/tyche-cli`             |
| L4    | Federation (cross-firm aggregator)          | `crates/tyche-fed` (skeleton)              |
| L3    | Attestation (records, signatures, Merkle)   | `crates/tyche-attest`, `contracts/`        |
| L2    | Simulation (Monte Carlo, recovery, factors) | `crates/tyche-sim`                         |
| L1    | Data graph (loans, portfolios, scenarios)   | `crates/tyche-types`                       |

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the long form.

## Repository layout

```
tyche/
├── crates/                # Rust workspace (8 crates)
├── contracts/             # Foundry workspace (Solidity)
├── apps/web/              # Next.js 15 dashboard
├── notebooks/             # Quant research (Python)
├── examples/              # Synthetic portfolio + driver scripts
├── scripts/               # dev.sh, e2e.sh
└── docs/                  # ARCHITECTURE, METHODOLOGY, SECURITY, CONTRIBUTING
```

## Status

Spike phase — not production-ready. No mainnet deployment, no audit, no
real range proofs (skeletons only). See [docs/SECURITY.md](docs/SECURITY.md)
for what is and isn't believable in this build.

## License

Apache-2.0. See [LICENSE](LICENSE).
