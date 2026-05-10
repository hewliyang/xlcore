#!/usr/bin/env bash
# Builds a workbook with a single TableStyleMedium2 ListObject — header
# + 5 data rows + totals row + autoFilter — to exercise the renderer's
# table chrome (header band, banded rows, filter arrows, totals row).
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
F="${1:-$HERE/table-medium.xlsx}"
rm -f "$F"
hsx create "$F" >/dev/null

hsx eval "$F" - <<'JS'
const headers = ["Region", "Q1", "Q2", "Q3"];
const rows = [
  ["North",   1200, 1500, 1800],
  ["South",    900, 1100, 1300],
  ["East",    1400, 1600, 1750],
  ["West",     800,  950, 1050],
  ["Central", 1100, 1250, 1400],
];

for (let c = 0; c < headers.length; c++) sheet.getCell(0, c).value(headers[c]);
for (let r = 0; r < rows.length; r++) {
  for (let c = 0; c < rows[r].length; c++) {
    sheet.getCell(1 + r, c).value(rows[r][c]);
  }
}
sheet.getCell(6, 0).value("Total");
for (let c = 1; c < 4; c++) {
  sheet.getCell(6, c).formula(`SUBTOTAL(109,${String.fromCharCode(65 + c)}2:${String.fromCharCode(65 + c)}6)`);
}

const Tables = GC.Spread.Sheets.Tables;
// add(name, row, col, rowCount, colCount, theme) — 7 rows = header + 5 data + totals
const t = sheet.tables.add("RegionSales", 0, 0, 7, 4, Tables.TableThemes.medium2);
t.showHeader(true);
t.showFooter(true);
t.bandRows(true);
t.filterButtonVisible(true);

sheet.setColumnWidth(0, 90);
for (let c = 1; c < 4; c++) sheet.setColumnWidth(c, 70);
JS

echo "wrote $F"
