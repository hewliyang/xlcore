#!/usr/bin/env bash
# Fixture: a grid of commonly-used DrawingML `prstGeom` autoshapes.
#
# Covers the preset shape types that today's renderer (`shape.ts`)
# has at least a path for, plus a handful of the next-20 presets
# `parity-shapes.md` calls out as P1 (chevron / pentagon / hexagon /
# star5 / flowchartProcess / flowchartDecision). Lines (`line` /
# `lineInverse`) are included so we can lock in the "preset=line
# should render as a stroked segment, not a rectangle fallback"
# behavior the parity doc flags.
#
# Each shape gets a short text label in its body so we also exercise
# the txBody path (run color, font size, paragraph alignment, vertical
# anchor) on every preset in one shot.
#
# Catches regressions in:
#   (a) `xdr:sp` extraction from `twoCellAnchor` in
#       `crates/xlcore-export/src/shapes.rs`,
#   (b) `prstGeom` preset dispatch in
#       `packages/xlsx-preview/src/shape.ts` — including the unknown-
#       preset → rectangle fallback (chevron / pentagon / hexagon /
#       star / flowchart symbols today),
#   (c) z-order preservation across many same-anchor shapes (XML
#       traversal order),
#   (d) rotation `xfrm@rot` (last row carries 30° + 90° rotated
#       copies of rectangle / leftArrow / star).
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
F="${1:-$HERE/basic-autoshapes.xlsx}"
rm -f "$F"
hsx create "$F" >/dev/null

hsx eval "$F" - <<'JS'
const sht = workbook.getSheet(0);
const T = GC.Spread.Sheets.Shapes.AutoShapeType;

// Wide canvas so the grid breathes.
for (let c = 0; c < 14; c++) sht.setColumnWidth(c, 88);
for (let r = 0; r < 18; r++) sht.setRowHeight(r, 22);

// Each entry: [name, AutoShapeType, label, rotation°].
const grid = [
  // Row A: basic primitives.
  ["rect",        T.rectangle,         "rect",     0],
  ["rRect",       T.roundedRectangle,  "roundRect",0],
  ["oval",        T.oval,              "ellipse",  0],
  ["tri",         T.isoscelesTriangle, "triangle", 0],
  ["diamond",     T.diamond,           "diamond",  0],
  // Row B: block arrows (cardinal).
  ["right",       T.rightArrow,        "→",        0],
  ["left",        T.leftArrow,         "←",        0],
  ["up",          T.upArrow,           "↑",        0],
  ["down",        T.downArrow,         "↓",        0],
  ["leftRight",   T.leftRightArrow,    "↔",        0],
  // Row C: P1 presets parity-shapes.md flags next.
  ["chevron",     T.chevron,           "chevron",  0],
  ["penta",       T.pentagon,          "pentagon", 0],
  ["hex",         T.hexagon,           "hexagon",  0],
  ["star5",       T.shape5pointStar,   "★",        0],
  ["fcProc",      T.flowchartProcess,  "process",  0],
  // Row D: lines + decision + rotated copies.
  ["line",        T.line,              "",         0],
  ["lineInv",     T.lineInverse,       "",         0],
  ["fcDec",       T.flowchartDecision, "decision", 0],
  ["rectRot30",   T.rectangle,         "rot30",    30],
  ["arrowRot90",  T.rightArrow,        "→ rot90",  90],
];

// 5 shapes per row, 95px wide × 70px tall, 12px gutter.
const W = 95, H = 70, GUT_X = 18, GUT_Y = 24;
const X0 = 12, Y0 = 14;
const COLS = 5;

for (let i = 0; i < grid.length; i++) {
  const [name, type, label, rot] = grid[i];
  const col = i % COLS, row = Math.floor(i / COLS);
  const x = X0 + col * (W + GUT_X);
  const y = Y0 + row * (H + GUT_Y);
  const s = sht.shapes.add(name, type, x, y, W, H);
  if (label) s.text(label);
  if (rot) s.rotate(rot);
}
JS

echo "wrote $F"
