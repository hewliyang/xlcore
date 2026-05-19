#!/usr/bin/env bash
# Fixture: direct `<a:ln>` cap / join / prstDash on **non-connector**
# autoshapes. Until P1 #5 shipped we only honored these on
# connectors / lines — direct dash on a rectangle outline was dropped
# at extraction and the painter hardcoded `lineCap=butt, lineJoin=miter`
# regardless of what the spec wrote.
#
# Layout (4 rows × ≤4 shapes):
#   row 1 — dash variants (solid / dot / dash / lgDash / dashDot /
#           lgDashDot / lgDashDotDot) on a wide rectangle.
#   row 2 — cap variants (flat / square / round) on a thick dashed
#           horizontal line preset. Each cap renders the dash-segment
#           endcap differently — dot becomes a square or filled circle.
#   row 3 — join variants (miter / bevel / round) on a thick-stroked
#           rectangle. Corner shape is the visible signal.
#   row 4 — dash on roundRect (preset that today only honored dash
#           on the connector code path).
#
# Catches regressions in:
#   (a) `line_dash_token` / `line_cap_token` / `line_join_token` extraction
#       in `crates/xlcore-export/src/shapes.rs` (direct `<a:ln>` path).
#   (b) `drawShape` honoring `node.lineDash` / `node.lineCap` /
#       `node.lineJoin` in `packages/xlsx-preview/src/shape.ts`.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
F="${1:-$HERE/line-cap-join-dash.xlsx}"
rm -f "$F"
hsx create "$F" >/dev/null

hsx eval "$F" - <<'JS'
const sht = workbook.getSheet(0);
const T = GC.Spread.Sheets.Shapes.AutoShapeType;
const DASH = GC.Spread.Sheets.Shapes.PresetLineDashStyle;
const CAP = GC.Spread.Sheets.Shapes.LineCapStyle;
const JOIN = GC.Spread.Sheets.Shapes.LineJoinStyle;

for (let c = 0; c < 14; c++) sht.setColumnWidth(c, 88);
for (let r = 0; r < 22; r++) sht.setRowHeight(r, 22);

const W = 130, H = 56, GX = 18, GY = 22;
const X0 = 12, Y0 = 14;

function setLine(shape, props) {
  const st = shape.style();
  Object.assign(st.line, props);
  shape.style(st);
}

// Row 1: dash tokens (rectangle, thick stroke).
const dashRow = [
  ["solid",        DASH.solid],
  ["squareDot",    DASH.squareDot],
  ["dash",         DASH.dash],
  ["longDash",     DASH.longDash],
  ["dashDot",      DASH.dashDot],
  ["longDashDot",  DASH.longDashDot],
  ["lDashDotDot",  DASH.longDashDotDot],
];
for (let i = 0; i < dashRow.length; i++) {
  const [label, d] = dashRow[i];
  const x = X0 + i * (W + GX), y = Y0;
  const s = sht.shapes.add("dash_" + label, T.rectangle, x, y, W, H);
  s.text(label);
  setLine(s, { color: "rgb(31,78,121)", width: 3, lineStyle: d });
}

// Row 2: cap variants on a thick dashed horizontal line preset.
const capRow = [
  ["cap=flat",   CAP.flat],
  ["cap=square", CAP.square],
  ["cap=round",  CAP.round],
];
for (let i = 0; i < capRow.length; i++) {
  const [label, cap] = capRow[i];
  const x = X0 + i * (W + GX), y = Y0 + (H + GY) + 12;
  const s = sht.shapes.add("cap_" + label, T.line, x, y, W, 0);
  setLine(s, { color: "rgb(192,0,0)", width: 8, lineStyle: DASH.dash, capType: cap });
  // Caption box (no outline) below the line.
  const lbl = sht.shapes.add("cap_lbl_" + label, T.rectangle, x, y + 10, W, 22);
  lbl.text(label);
  setLine(lbl, { color: "rgb(255,255,255)", width: 0 });
}

// Row 3: join variants on a thick-stroked rectangle.
const joinRow = [
  ["join=miter", JOIN.miter],
  ["join=bevel", JOIN.bevel],
  ["join=round", JOIN.round],
];
for (let i = 0; i < joinRow.length; i++) {
  const [label, join] = joinRow[i];
  const x = X0 + i * (W + GX), y = Y0 + 2 * (H + GY) + 12;
  const s = sht.shapes.add("join_" + label, T.rectangle, x, y, W, H);
  s.text(label);
  setLine(s, { color: "rgb(56,87,35)", width: 8, joinType: join });
}

// Row 4: dash on roundRect.
const dashRoundRow = [
  ["rR/squareDot", DASH.squareDot],
  ["rR/dash",      DASH.dash],
  ["rR/longDash",  DASH.longDash],
];
for (let i = 0; i < dashRoundRow.length; i++) {
  const [label, d] = dashRoundRow[i];
  const x = X0 + i * (W + GX), y = Y0 + 3 * (H + GY) + 12;
  const s = sht.shapes.add("rr_" + label, T.roundedRectangle, x, y, W, H);
  s.text(label);
  setLine(s, { color: "rgb(112,48,160)", width: 3, lineStyle: d });
}
JS

echo "wrote $F"
