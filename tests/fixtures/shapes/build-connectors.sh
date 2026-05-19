#!/usr/bin/env bash
# Fixture: DrawingML `xdr:cxnSp` connectors (straight + elbow)
# carrying arrowheads, dash patterns, and stroke widths.
#
# Today's extractor (`crates/xlcore-export/src/shapes.rs`) ignores
# `cxnSp` entirely — it walks `EG_ObjectChoices::Sp` / `GrpSp` only.
# Renderer (`packages/xlsx-preview/src/shape.ts`) has no connector
# path. Net effect: every connector in this fixture should currently
# vanish from `ours.png`. That makes this the cleanest "❌ all-or-
# nothing" wedge fixture for the connector P0 milestone in
# `docs/parity-shapes.md`.
#
# Layout: 4 rectangles act as "nodes" arranged in two columns; the
# connectors hop between them. Each connector picks a different
# combination of (straight/elbow, arrowheads, dash, width) so the
# fixture stresses every axis at once.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
F="${1:-$HERE/connectors.xlsx}"
rm -f "$F"
hsx create "$F" >/dev/null

hsx eval "$F" - <<'JS'
const sht = workbook.getSheet(0);
const T  = GC.Spread.Sheets.Shapes.AutoShapeType;
const CT = GC.Spread.Sheets.Shapes.ConnectorType;
const AS = GC.Spread.Sheets.Shapes.ArrowheadStyle;
const PD = GC.Spread.Sheets.Shapes.PresetLineDashStyle;

for (let c = 0; c < 16; c++) sht.setColumnWidth(c, 80);
for (let r = 0; r < 20; r++) sht.setRowHeight(r, 22);

// Four box "nodes" — two columns, two rows.
const boxes = [
  ["A", 30, 30],   // top-left
  ["B", 360, 30],  // top-right
  ["C", 30, 220],  // bottom-left
  ["D", 360, 220], // bottom-right
];
const handles = {};
for (const [name, x, y] of boxes) {
  const s = sht.shapes.add(name, T.rectangle, x, y, 130, 60);
  s.text(name);
  handles[name] = s;
}

function conn(name, type, x1, y1, x2, y2, mutate) {
  const s = sht.shapes.addConnector(name, type, x1, y1, x2, y2);
  if (mutate) mutate(s);
  return s;
}

// 1. Straight A → B with default (no) arrowheads.
conn("c1", CT.straight, 160, 60, 360, 60);

// 2. Straight B → C diagonal with arrowhead on the end.
conn("c2", CT.straight, 360, 90, 160, 220, s => {
  try {
    s.style({ line: { width: 2, endArrowheadStyle: AS.triangle } });
  } catch(_){}
});

// 3. Elbow A → D with arrowheads on both ends.
conn("c3", CT.elbow, 95, 90, 425, 220, s => {
  try {
    s.style({
      line: {
        width: 1.5,
        beginArrowheadStyle: AS.oval,
        endArrowheadStyle:   AS.triangle,
      },
    });
  } catch(_){}
});

// 4. Straight dashed thick red C → D.
conn("c4", CT.straight, 160, 250, 360, 250, s => {
  try {
    s.style({
      line: {
        width: 3,
        color: "rgb(192,57,43)",
        lineStyle: PD.dash,
        endArrowheadStyle: AS.triangle,
      },
    });
  } catch(_){}
});

// 5. Elbow B → D vertical with arrow.
conn("c5", CT.elbow, 425, 90, 425, 220, s => {
  try { s.style({ line: { width: 1.5, endArrowheadStyle: AS.triangle } }); } catch(_){}
});
JS

echo "wrote $F"
