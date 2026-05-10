#!/usr/bin/env bash
# Render every .xlsx under ../ib-bench/ to a standalone HTML preview using xlcore.
# Usage: ./ib-bench-render.sh [SRC_DIR] [OUT_DIR]
set -euo pipefail

SRC="${1:-../ib-bench}"
OUT="${2:-/tmp/xlcore-html}"
BIN="$(dirname "$0")/target/release/xlcore"

if [[ ! -x "$BIN" ]]; then
  echo "building xlcore..." >&2
  cargo build --release -p xlcore-cli
fi

mkdir -p "$OUT"

ok=0; fail=0
while IFS= read -r f; do
  rel="${f#"$SRC"/}"
  # flatten path: a/b/c.xlsx -> a__b__c.html
  name="${rel%.xlsx}"
  name="${name//\//__}.html"
  if "$BIN" preview "$f" -o "$OUT/$name" >/dev/null 2>&1; then
    ok=$((ok+1))
  else
    fail=$((fail+1))
    echo "FAIL: $f" >&2
  fi
done < <(find "$SRC" -name '*.xlsx' -type f)

echo "rendered $ok ok, $fail failed -> $OUT"
