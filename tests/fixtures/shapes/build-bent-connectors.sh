#!/usr/bin/env bash
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
F="${1:-$HERE/bent-connectors.xlsx}"
rm -f "$F"
hsx create "$F" >/dev/null

hsx eval "$F" - <<'JS'
const sht = workbook.getSheet(0);
const T  = GC.Spread.Sheets.Shapes.AutoShapeType;
const CT = GC.Spread.Sheets.Shapes.ConnectorType;
const AS = GC.Spread.Sheets.Shapes.ArrowheadStyle;

for (let c = 0; c < 16; c++) sht.setColumnWidth(c, 80);
for (let r = 0; r < 30; r++) sht.setRowHeight(r, 22);

const BOX_W = 110, BOX_H = 48;

const rows = [
  { y: 20,  label: "b2" },
  { y: 150, label: "b3" },
  { y: 280, label: "b4" },
  { y: 410, label: "b5" },
];
for (const r of rows) {
  const ax = 30,  ay = r.y;
  const bx = 440, by = r.y + 70;
  sht.shapes.add(`${r.label}_A`, T.rectangle, ax, ay, BOX_W, BOX_H).text(`${r.label} from`);
  sht.shapes.add(`${r.label}_B`, T.rectangle, bx, by, BOX_W, BOX_H).text(`${r.label} to`);
  const c = sht.shapes.addConnector(
    `${r.label}_conn`,
    CT.elbow,
    ax + BOX_W, ay + BOX_H / 2,
    bx,          by + BOX_H / 2,
  );
  try {
    c.style({ line: { width: 1.5, endArrowheadStyle: AS.triangle } });
  } catch (_) {}
}

const labels = [
  { row: 1,  text: "bentConnector2 \u2014 L-bend (no adj)" },
  { row: 7,  text: "bentConnector3 \u2014 adj1=50%" },
  { row: 13, text: "bentConnector4 \u2014 adj1=50%, adj2=50%" },
  { row: 19, text: "bentConnector5 \u2014 adj1=33%, adj2=50%, adj3=67%" },
];
for (const l of labels) sht.setValue(l.row, 7, l.text);
JS

hsx daemon flush >/dev/null 2>&1 || true
python3 "$HERE/_patch_bent_connectors.py" "$F"

echo "wrote $F"
