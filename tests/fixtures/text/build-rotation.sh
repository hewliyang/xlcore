#!/usr/bin/env bash
# Builds a workbook exercising text rotation via SpreadJS's
# `textOrientation` (which writes to OOXML `cellXfs/.../alignment/textRotation`).
#
# OOXML `textRotation` semantics:
#   0       horizontal
#   1..90   N° counterclockwise (90 = reads bottom-to-top)
#   91..180 (value-90)° clockwise (180 = reads top-to-bottom)
#   255     stacked (chars upright, vertically arranged)
#
# SpreadJS `textOrientation` is signed (-90..90):
#   +N => OOXML N        (CCW)
#   -N => OOXML (90+N)   (CW)
#   "vertical text" via the `isVerticalText` style flag => OOXML 255.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
F="${1:-$HERE/rotation.xlsx}"
rm -f "$F"
hsx create "$F" >/dev/null

hsx eval "$F" - <<'JS'
// Wide row so rotated labels fit; tall enough to show 90° readouts.
sheet.setRowHeight(0, 110);
for (let c = 0; c < 9; c++) sheet.setColumnWidth(c, 70);

// Row 0: rotated headers at -90, -45, -30, 0, 30, 45, 90, plus stacked.
const angles = [-90, -45, -30, 0, 30, 45, 90];
for (let i = 0; i < angles.length; i++) {
  const cell = sheet.getCell(0, i);
  cell.value(`Rot ${angles[i]}°`);
  cell.textOrientation(angles[i]);
  cell.hAlign(GC.Spread.Sheets.HorizontalAlign.center);
  cell.vAlign(GC.Spread.Sheets.VerticalAlign.bottom);
  cell.font("bold 11pt Calibri");
}
// Stacked. Set via a fresh Style on the cell range.
const sty = new GC.Spread.Sheets.Style();
sty.isVerticalText = true;
sty.hAlign = GC.Spread.Sheets.HorizontalAlign.center;
sty.vAlign = GC.Spread.Sheets.VerticalAlign.center;
sty.font = "bold 11pt Calibri";
sheet.setStyle(0, 7, sty);
sheet.getCell(0, 7).value("STACK");

// Row 1: filler numeric data so headers look real.
sheet.setRowHeight(1, 22);
for (let c = 0; c < 8; c++) sheet.getCell(1, c).value((c + 1) * 10);
JS

echo "wrote $F"
