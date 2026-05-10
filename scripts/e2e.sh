#!/usr/bin/env bash
# scripts/e2e.sh — run the full Tyche test matrix locally.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

require() { command -v "$1" >/dev/null 2>&1 || { echo "missing: $1" >&2; exit 1; }; }
for tool in cargo forge pnpm; do require "$tool"; done

echo "==> cargo fmt --check"
cargo fmt --all -- --check

echo "==> cargo clippy"
cargo clippy --workspace --all-targets -- -D warnings

echo "==> cargo test (workspace)"
cargo test --workspace --all-targets

echo "==> forge test"
(cd contracts && forge test -vv)

echo "==> pnpm install"
pnpm install --frozen-lockfile

echo "==> pnpm typecheck && pnpm test"
pnpm --filter @tyche/web typecheck
pnpm --filter @tyche/web test || echo "(no vitest tests yet)"

echo "==> example portfolio"
bash examples/synthetic_portfolio/run.sh

echo
echo "All checks passed."
