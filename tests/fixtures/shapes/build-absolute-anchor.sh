#!/usr/bin/env bash
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
F="${1:-$HERE/absolute-anchor.xlsx}"
rm -f "$F"
hsx create "$F" >/dev/null

hsx eval "$F" - <<'JS'
const sht = workbook.getSheet(0);
const T = GC.Spread.Sheets.Shapes.AutoShapeType;
for (let c = 0; c < 8; c++) sht.setColumnWidth(c, 90);
for (let r = 0; r < 8; r++) sht.setRowHeight(r, 28);

// Cell-anchored reference (row 0)
const ref = sht.shapes.add("cellAnchored", T.rectangle, 120, 40, 140, 70);
ref.text("cell anchor");

// Will be rewritten to absoluteAnchor by the Python patch (row 1)
const abs = sht.shapes.add("absolutePos", T.rectangle, 320, 160, 160, 90);
abs.text("absolute");
JS

hsx daemon flush >/dev/null 2>&1 || true
python3 "$HERE/_patch_absolute_anchor.py" "$F"
echo "wrote $F"
