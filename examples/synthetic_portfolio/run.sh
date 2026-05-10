#!/usr/bin/env bash
# Run the spike example end-to-end: simulate every scenario in scenarios.json
# and produce a signed attestation record for each.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

mkdir -p out

python3 "$ROOT/examples/synthetic_portfolio/generate.py"

SCENARIOS=$(jq -r '.[].scenario_id' examples/synthetic_portfolio/scenarios.json)

for s in $SCENARIOS; do
  echo "--- scenario: $s ---"
  cargo run --quiet --release -p tyche-cli -- simulate \
    --portfolio examples/synthetic_portfolio/portfolio.json \
    --scenarios-file examples/synthetic_portfolio/scenarios.json \
    --scenario "$s" \
    --paths 10000

  cargo run --quiet --release -p tyche-cli -- attest \
    --portfolio examples/synthetic_portfolio/portfolio.json \
    --scenarios-file examples/synthetic_portfolio/scenarios.json \
    --scenario "$s" \
    --paths 10000 \
    --out "out/record-$s.json"

  cargo run --quiet --release -p tyche-cli -- verify --record "out/record-$s.json"
done

echo
echo "All scenarios attested. Records under out/."
