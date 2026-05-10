#!/usr/bin/env bash
# Fixture: diagonal borders (OOXML `<border diagonalUp/diagonalDown>`).
#
#   diagonalDown = top-left  -> bottom-right slash ( \ )
#   diagonalUp   = bottom-left -> top-right slash  ( / )
#
# Layout (5 cells in row 2):
#
#       B            C            D            E            F
#   2  [ \ thin ]  [ / thin ]  [ X thin ]  [ X thick ]  [ X red dashed ]
#
# We patch the OOXML directly because SpreadJS (hsx) silently drops the
# `borderDiagonalUp` / `borderDiagonalDown` style attrs on xlsx export
# — the resulting `<border>` tags carry only the regular box sides.
# Running the equivalent setStyle through hsx and grepping styles.xml
# confirms: no `diagonal*` attrs reach disk.
#
# Catches regressions in (a) extractor pulling `diagonalUp` /
# `diagonalDown` attrs + the `<diagonal>` child, (b) renderer's
# `drawDiagonalBorders` clipping + style/color reuse with
# `drawBorderLine`.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
F="${1:-$HERE/diagonal.xlsx}"
rm -f "$F"
hsx create "$F" >/dev/null
python3 "$HERE/_patch_diagonal.py" "$F"
echo "wrote $F"
