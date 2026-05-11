#!/usr/bin/env bash
# Fixture: OOXML <gradientFill> variants. Six cells in a 3x2 grid:
#   linear degree=0 / 90 / 45 / 270, a 3-stop linear, and a path
#   (radial) gradient with a centered inner-convergence rect.
#
# SpreadJS doesn't expose gradient fills on its public style API
# (it only writes solid `patternType="solid"` from `backColor`),
# so we patch the OOXML directly. See `_patch_gradients.py`.
#
# Catches regressions in (a) extractor's gradient_type / degree /
# left|right|top|bottom / GradientStop.position round-trip, (b)
# renderer's multi-stop linear axis projection + path radial.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
F="${1:-$HERE/gradients.xlsx}"
rm -f "$F"
hsx create "$F" >/dev/null
python3 "$HERE/_patch_gradients.py" "$F"
echo "wrote $F"
