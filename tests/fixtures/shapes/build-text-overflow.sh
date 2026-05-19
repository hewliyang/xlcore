#!/usr/bin/env bash
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
F="${1:-$HERE/text-overflow.xlsx}"
rm -f "$F"
hsx create "$F" >/dev/null

hsx eval "$F" - <<'JS'
const sht = workbook.getSheet(0);
const T = GC.Spread.Sheets.Shapes.AutoShapeType;

for (let c = 0; c < 16; c++) sht.setColumnWidth(c, 80);
for (let r = 0; r < 16; r++) sht.setRowHeight(r, 22);

const W = 160, H = 60, GX = 22, GY = 28;
const X0 = 14, Y0 = 14;

const MULTI =
  "Line one of the overflowing label.\n" +
  "Line two continues the discussion.\n" +
  "Line three pushes past the bottom.\n" +
  "Line four should be hidden by clip.\n" +
  "Line five trails off into nothing.";

const SINGLE =
  "Single line label that is far too long to fit horizontally inside the body rect.";

function place(name, col, row, text) {
  const x = X0 + col * (W + GX);
  const y = Y0 + row * (H + GY);
  const s = sht.shapes.add(name, T.rectangle, x, y, W, H);
  s.text(text);
  return s;
}

place("vOverflow", 0, 0, MULTI);
place("vClip",     1, 0, MULTI);
place("vEllipsis", 2, 0, MULTI);

place("hOverflow", 0, 1, SINGLE);
place("hClip",     1, 1, SINGLE);
place("hEllipsis", 2, 1, SINGLE);
JS

hsx daemon flush >/dev/null 2>&1 || true
python3 "$HERE/_patch_text_overflow.py" "$F"
echo "wrote $F"
