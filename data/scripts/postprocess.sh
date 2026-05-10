#!/usr/bin/env bash
# data/scripts/postprocess.sh
# Unzip BIS / SEC bundles, gunzip Eurostat, generate a manifest.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
RAW="$ROOT/data/raw"
PROC="$ROOT/data/processed"
mkdir -p "$PROC"

unzip_to() {
  local zip="$1" dest="$2"
  if [[ -f "$zip" ]]; then
    mkdir -p "$dest"
    if unzip -oq "$zip" -d "$dest"; then
      echo "  unzipped: $(basename "$zip") -> $dest"
    else
      echo "  FAIL unzip: $(basename "$zip")" >&2
    fi
  fi
}

echo "=== BIS zips ==="
for z in "$RAW"/bis/*.zip; do
  [[ -f "$z" ]] || continue
  unzip_to "$z" "$PROC/bis/$(basename "${z%.zip}")"
done

echo "=== SEC EDGAR zips ==="
for z in "$RAW"/sec_edgar/*.zip; do
  [[ -f "$z" ]] || continue
  unzip_to "$z" "$PROC/sec_edgar/$(basename "${z%.zip}")"
done

echo "=== CFPB / loan-level zips ==="
for z in "$RAW"/loan_level/*.zip; do
  [[ -f "$z" ]] || continue
  unzip_to "$z" "$PROC/loan_level/$(basename "${z%.zip}")"
done

echo "=== gunzip Eurostat ==="
mkdir -p "$PROC/eurostat"
for gz in "$RAW"/eurostat/*.csv.gz; do
  [[ -f "$gz" ]] || continue
  out="$PROC/eurostat/$(basename "${gz%.gz}")"
  if [[ ! -f "$out" || "$gz" -nt "$out" ]]; then
    gunzip -kc "$gz" > "$out"
    echo "  gunzipped: $(basename "$gz")"
  fi
done

echo
echo "=== File catalogue ==="
{
  echo "# Tyche dataset catalogue"
  echo "Generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo
  echo "## Raw downloads"
  echo
  find "$RAW" -type f -not -name '.gitkeep' \
    | sort \
    | while read -r f; do
        sz=$(stat -f%z "$f" 2>/dev/null || stat -c%s "$f")
        rel="${f#$ROOT/}"
        printf -- "- \`%s\` — %s\n" "$rel" "$(numfmt --to=iec "$sz" 2>/dev/null || echo "$sz B")"
      done
  echo
  echo "## Processed (extracted) "
  echo
  find "$PROC" -type f -not -name '.gitkeep' \
    | sort | head -200 \
    | while read -r f; do
        sz=$(stat -f%z "$f" 2>/dev/null || stat -c%s "$f")
        rel="${f#$ROOT/}"
        printf -- "- \`%s\` — %s\n" "$rel" "$(numfmt --to=iec "$sz" 2>/dev/null || echo "$sz B")"
      done
} > "$ROOT/data/MANIFEST.md"
echo "wrote data/MANIFEST.md"
