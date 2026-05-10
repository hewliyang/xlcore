#!/usr/bin/env bash
# Fixture: every OOXML underline variant on a single workbook.
#
# Covers all 4 ST_UnderlineValues (ECMA-376 §18.18.91) plus a "no
# underline" control: none / single / double / singleAccounting /
# doubleAccounting. SpreadJS's public JS API only exposes a single
# boolean `underline()` toggle, so we patch styles.xml directly.
#
# Catches regressions in:
#   (a) extractor's `<u val="...">` parsing — without proper variant
#       capture the renderer treats every underline as `single`;
#   (b) renderer's `paintTextDecorations` — `double` should paint
#       two parallel strokes; accounting variants currently render
#       as their non-accounting siblings (across-cell-width
#       semantics tracked in PARITY.md).
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
F="${1:-$HERE/underline.xlsx}"
rm -f "$F"
hsx create "$F" >/dev/null
python3 "$HERE/_patch_underline.py" "$F"
echo "wrote $F"
