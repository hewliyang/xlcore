#!/usr/bin/env bash
# Fixture: OOXML <vertAlign> on cell fonts and rich-text runs.
#
# Covers:
#   - Cell-font superscript / subscript (whole cell rendered shrunk +
#     raised/dropped).
#   - Rich-text run vertAlign mixed with baseline runs — the realistic
#     usage for chemical formulas (H₂O) and exponents (x², E=mc²).
#
# SpreadJS's public JS API doesn't expose <vertAlign> on either path,
# so we patch styles.xml + the inline-string sheet body directly.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
F="${1:-$HERE/vertalign.xlsx}"
rm -f "$F"
hsx create "$F" >/dev/null
python3 "$HERE/_patch_vertalign.py" "$F"
echo "wrote $F"
