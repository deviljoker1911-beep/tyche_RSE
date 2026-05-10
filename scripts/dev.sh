#!/usr/bin/env bash
# scripts/dev.sh — bring up the local Tyche stack.
#
# - boots an Anvil chain on :8545
# - deploys AttestationRegistry and FederationAggregator
# - rebuilds the wasm module into apps/web/public/wasm
# - starts the Next.js dev server on :3000
#
# Requires: anvil, forge, cargo, wasm-pack, pnpm, jq.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

require() { command -v "$1" >/dev/null 2>&1 || { echo "missing: $1" >&2; exit 1; }; }
for tool in anvil forge cargo wasm-pack pnpm jq; do require "$tool"; done

mkdir -p apps/web/public/wasm

ANVIL_PID=""
NEXT_PID=""
cleanup() {
  [[ -n "$ANVIL_PID" ]] && kill "$ANVIL_PID" 2>/dev/null || true
  [[ -n "$NEXT_PID" ]] && kill "$NEXT_PID" 2>/dev/null || true
}
trap cleanup EXIT

# --- 1. anvil ---
echo "==> starting anvil on 127.0.0.1:8545"
anvil --silent --port 8545 &
ANVIL_PID=$!
# Wait for chain.
for _ in {1..40}; do
  if cast block-number --rpc-url http://127.0.0.1:8545 >/dev/null 2>&1; then break; fi
  sleep 0.25
done

# --- 2. deploy contracts ---
echo "==> deploying contracts"
pushd contracts >/dev/null
DEPLOY_OUT=$(forge script script/Deploy.s.sol:Deploy \
  --rpc-url http://127.0.0.1:8545 \
  --broadcast --silent \
  --private-key 0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80)
popd >/dev/null
REGISTRY=$(echo "$DEPLOY_OUT" | grep "AttestationRegistry:" | awk '{print $2}')
AGG=$(echo "$DEPLOY_OUT" | grep "FederationAggregator:" | awk '{print $2}')
cat > apps/web/public/contracts.json <<EOF
{
  "AttestationRegistry": "$REGISTRY",
  "FederationAggregator": "$AGG"
}
EOF
echo "    AttestationRegistry  $REGISTRY"
echo "    FederationAggregator $AGG"

# --- 3. wasm ---
echo "==> building wasm module"
wasm-pack build --target web --out-dir "$ROOT/apps/web/public/wasm" --release crates/tyche-wasm

# --- 4. web dev server ---
echo "==> starting next.js on 3000"
(cd apps/web && pnpm install --silent && pnpm dev) &
NEXT_PID=$!

echo
echo "Tyche stack up. Press Ctrl-C to stop."
wait
