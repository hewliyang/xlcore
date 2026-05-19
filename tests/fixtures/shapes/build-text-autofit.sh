#!/usr/bin/env bash
# Fixture: `<a:bodyPr>` text-autofit variants.
#
# Targets P1 #7 in `docs/parity-shapes.md`:
#   - `<a:normAutofit fontScale="…" lnSpcReduction="…">`
#     (Excel pre-computes both when it shrinks-to-fit a cramped box;
#     the renderer just has to apply the scale at paint time),
#   - `<a:spAutoFit/>` (shape ext was author-resized to fit text —
#     no paint-time scaling needed; we record the marker and the
#     box already has the auto-fitted geometry).
#
# Layout: a 3×2 grid of identically-sized textboxes carrying the
# same overflowing multi-line label. Each column / row applies a
# different autofit choice, so a regression collapses adjacent cells
# to the same picture.
#
#   row 0 — fontScale only: 100% (baseline) / 75% / 50%
#   row 1 — fontScale + lnSpcReduction:
#             50% font + 20% line / 25% font / spAutoFit (marker only)
#
# SpreadJS won't emit `<a:normAutofit>` / `<a:spAutoFit>` through its
# public API — it always writes `<a:noAutofit>` (or omits the choice
# entirely). After `hsx` lays the workbook down we splice the
# autofit elements directly into `xl/drawings/drawing1.xml`. Without
# that step every row of this fixture is visually identical.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
F="${1:-$HERE/text-autofit.xlsx}"
rm -f "$F"
hsx create "$F" >/dev/null

hsx eval "$F" - <<'JS'
const sht = workbook.getSheet(0);
const T = GC.Spread.Sheets.Shapes.AutoShapeType;

for (let c = 0; c < 16; c++) sht.setColumnWidth(c, 80);
for (let r = 0; r < 16; r++) sht.setRowHeight(r, 22);

const W = 160, H = 90, GX = 22, GY = 26;
const X0 = 14, Y0 = 14;

// Long label that would overflow the 160x90 box at 11pt — the
// authoring tool would normally shrink-to-fit by setting fontScale.
const LABEL =
  "Quarterly review meeting notes for the regional sales team " +
  "covering Q1 through Q4 of fiscal year 2026 with detailed " +
  "breakdowns by product line.";

function place(name, col, row) {
  const x = X0 + col * (W + GX);
  const y = Y0 + row * (H + GY);
  const s = sht.shapes.add(name, T.rectangle, x, y, W, H);
  s.text(LABEL);
  return s;
}

// row 0
place("fs100", 0, 0); // baseline — fontScale absent
place("fs75",  1, 0);
place("fs50",  2, 0);

// row 1
place("fs50ln20", 0, 1);
place("fs25",     1, 1);
place("spAuto",   2, 1);
JS

hsx daemon flush >/dev/null 2>&1 || true
python3 "$HERE/_patch_text_autofit.py" "$F"
echo "wrote $F"
