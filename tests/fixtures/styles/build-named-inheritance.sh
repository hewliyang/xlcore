#!/usr/bin/env bash
# Build tests/fixtures/styles/named-inheritance.xlsx — exercises
# `cellStyleXf` inheritance via `apply*="0"` flags. See the docstring
# in `_patch_named_inheritance.py` for why we hand-patch the OOXML
# instead of going through `hsx`.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
out="$here/named-inheritance.xlsx"

hsx create "$out"
python3 "$here/_patch_named_inheritance.py" "$out"
echo "wrote $out"
