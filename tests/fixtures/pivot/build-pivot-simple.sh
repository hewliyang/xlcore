#!/usr/bin/env bash
# Builds a simple pivot table: a small sales table on Sheet1!A1:D13 plus a
# pivot table on a second sheet that aggregates by Region (rows) × Product
# (cols) summing Amount. Exercises the cheap-path renderer (materialized
# cell values + pivot chrome).
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
F="${1:-$HERE/pivot-simple.xlsx}"
rm -f "$F"
hsx create "$F" >/dev/null

hsx eval "$F" - <<'JS'
// --- source data on Sheet1 ---
const headers = ["Region", "Product", "Quarter", "Amount"];
const rows = [
  ["North", "Widget",  "Q1",  500],
  ["North", "Widget",  "Q2",  600],
  ["North", "Gadget",  "Q1",  300],
  ["North", "Gadget",  "Q2",  450],
  ["South", "Widget",  "Q1",  700],
  ["South", "Widget",  "Q2",  800],
  ["South", "Gadget",  "Q1",  250],
  ["South", "Gadget",  "Q2",  400],
  ["East",  "Widget",  "Q1",  550],
  ["East",  "Gadget",  "Q1",  220],
  ["West",  "Widget",  "Q2",  900],
  ["West",  "Gadget",  "Q2",  380],
];
for (let c = 0; c < headers.length; c++) sheet.getCell(0, c).value(headers[c]);
for (let r = 0; r < rows.length; r++) {
  for (let c = 0; c < rows[r].length; c++) {
    sheet.getCell(1 + r, c).value(rows[r][c]);
  }
}
sheet.setColumnWidth(0, 80);
sheet.setColumnWidth(1, 80);
sheet.setColumnWidth(2, 70);
sheet.setColumnWidth(3, 80);

// --- pivot on a new sheet ---
workbook.addSheet(1);
const pivotSheet = workbook.getSheet(1);
pivotSheet.name("Pivot");

const pt = pivotSheet.pivotTables.add(
  "RegionProductPivot",
  "Sheet1!A1:D13",
  1, 1,                                 // anchor row, col
  GC.Spread.Pivot.PivotTableLayoutType.outline,
  GC.Spread.Pivot.PivotTableThemes.medium2
);
pt.add("Region",  "Region",  GC.Spread.Pivot.PivotTableFieldType.rowField);
pt.add("Product", "Product", GC.Spread.Pivot.PivotTableFieldType.columnField);
pt.add("Amount",  "Sum of Amount", GC.Spread.Pivot.PivotTableFieldType.valueField);

pivotSheet.setColumnWidth(1, 110);
pivotSheet.setColumnWidth(2, 90);
pivotSheet.setColumnWidth(3, 90);
pivotSheet.setColumnWidth(4, 100);

workbook.setActiveSheetIndex(1);
JS

echo "wrote $F"
