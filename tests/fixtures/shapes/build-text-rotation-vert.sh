#!/usr/bin/env bash
# Fixture: `<a:bodyPr rot>` and `<a:bodyPr vert>` variants.
#
# Targets P1 #8 in `docs/parity-shapes.md`:
#   - `<a:bodyPr rot="…">` rotates the *body* (text + layout) inside
#     the shape rect; distinct from `<a:xfrm rot>` which rotates the
#     whole shape.
#   - `<a:bodyPr vert="…">` selects a perpendicular reading direction
#     (`vert` = top-to-bottom, `vert270` = bottom-to-top, plus the
#     East-Asian / Mongolian / WordArt variants). The painter
#     collapses every perpendicular variant onto a flat ±90° rotation.
#
# Layout: a 3×3 grid of identically-sized rectangles, each carrying
# the same multi-line label so deltas isolate cleanly when one of
# those attributes regresses.
#
#   row 0 — bodyPr@rot:  0°  /  +45°  /  +90°
#   row 1 — bodyPr@rot:  +180° / -90° / -45°
#   row 2 — bodyPr@vert: vert / vert270 / eaVert
#
# SpreadJS won't emit `<a:bodyPr rot>` or `<a:bodyPr vert>` through
# its public API — after `hsx` lays the workbook down we splice
# those attrs directly into `xl/drawings/drawing1.xml`. Without
# that step every row of this fixture is visually identical.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
F="${1:-$HERE/text-rotation-vert.xlsx}"
rm -f "$F"
hsx create "$F" >/dev/null

hsx eval "$F" - <<'JS'
const sht = workbook.getSheet(0);
const T = GC.Spread.Sheets.Shapes.AutoShapeType;

for (let c = 0; c < 16; c++) sht.setColumnWidth(c, 80);
for (let r = 0; r < 24; r++) sht.setRowHeight(r, 22);

// Square-ish so the swap between portrait and landscape layout is
// visually obvious on the perpendicular cases.
const W = 150, H = 130, GX = 26, GY = 28;
const X0 = 14, Y0 = 14;

const LABEL = "Quarterly\nrevenue\nreview";

function place(name, col, row) {
  const x = X0 + col * (W + GX);
  const y = Y0 + row * (H + GY);
  const s = sht.shapes.add(name, T.rectangle, x, y, W, H);
  s.text(LABEL);
  return s;
}

// Row 0 — body rotation 0 / +45 / +90.
place("rot0",     0, 0);
place("rotP45",   1, 0);
place("rotP90",   2, 0);

// Row 1 — body rotation +180 / -90 / -45.
place("rotP180",  0, 1);
place("rotN90",   1, 1);
place("rotN45",   2, 1);

// Row 2 — vert tokens.
place("vert",     0, 2);
place("vert270",  1, 2);
place("eaVert",   2, 2);
JS

# hsx eval can return before the xlsx is fully flushed (per
# docs/TESTING.md “Why some builders patch the zip directly”).
hsx daemon flush >/dev/null 2>&1 || true
python3 "$HERE/_patch_text_rotation_vert.py" "$F"
echo "wrote $F"
