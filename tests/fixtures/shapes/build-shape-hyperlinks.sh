#!/usr/bin/env bash
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
F="${1:-$HERE/shape-hyperlinks.xlsx}"
rm -f "$F"
hsx create "$F" >/dev/null

hsx eval "$F" - <<'JS'
const sht = workbook.getSheet(0);
const T = GC.Spread.Sheets.Shapes.AutoShapeType;
for (let c = 0; c < 8; c++) sht.setColumnWidth(c, 100);
for (let r = 0; r < 6; r++) sht.setRowHeight(r, 32);

sht.shapes.add("externalLink", T.rectangle, 40, 40, 180, 70).text("external");
sht.shapes.add("internalLink", T.rectangle, 260, 40, 180, 70).text("internal");
sht.shapes.add("plain", T.rectangle, 40, 140, 180, 70).text("no link");
JS

hsx daemon flush >/dev/null 2>&1 || true
python3 "$HERE/_patch_shape_hyperlinks.py" "$F"
echo "wrote $F"
