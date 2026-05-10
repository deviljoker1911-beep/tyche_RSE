# Contributing to Tyche

Thanks for considering a contribution. This document explains the workflow we
use, the code conventions we hold to, and the bar a change has to clear before
it lands in `main`.

## Sign-off (DCO)

We use the [Developer Certificate of Origin](https://developercertificate.org/).
Every commit must carry a `Signed-off-by:` line. Configure once:

```sh
git config --global user.name  "Your Name"
git config --global user.email "your.email@example.com"
git commit --signoff
```

## Workflow

1. Fork the repo and create a branch off `main`.
2. Land one logical change per PR. Refactors, bug fixes, and feature work go
   in separate PRs unless they are genuinely the same change.
3. Run `scripts/e2e.sh` locally before pushing. CI will run it again.
4. Open a PR with a clear `Summary` and a `Test plan` checklist.
5. Self-review the diff before requesting human review. Most defects get
   caught here.

## Commit messages

Conventional Commits, imperative mood, no emoji, body wraps at 72 columns:

```
feat(sim): add importance sampling behind `is` feature flag

Adds an importance-sampling tail estimator to tyche-sim that biases
draws toward the loss tail and rescales by the likelihood ratio.
Disabled by default; opt in via `--features is`.

Signed-off-by: Your Name <your.email@example.com>
```

## Code conventions

- Rust: `2024` edition, `clippy::pedantic + nursery` clean, no `unwrap` /
  `panic!` in non-test code, public APIs documented with rustdoc + doctests.
- Solidity: `^0.8.24`, NatSpec on every external/public function, custom
  errors over `require` strings, no `tx.origin`, no inline assembly without
  justification.
- TypeScript: strict mode, `noUncheckedIndexedAccess`, no `any` (except for
  genuinely opaque externals, documented inline), Zod schemas at every
  boundary.

See `docs/ARCHITECTURE.md` for the bigger-picture conventions.

## What we will and won't merge

We will merge:

- bug fixes with reproductions in tests,
- new tests for existing code,
- documentation improvements,
- additive crates / modules with a clear use case,
- performance improvements with benchmark deltas.

We will likely **not** merge without discussion first:

- new third-party cryptographic dependencies,
- changes to the canonical-JSON encoding,
- changes to attestation-record fields (these are part of the protocol),
- new Solidity dependencies,
- alternative simulation models (open an RFC issue first).

## Code of conduct

This project follows the [Contributor Covenant 2.1](../CODE_OF_CONDUCT.md).
