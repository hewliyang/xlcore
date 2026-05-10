#!/usr/bin/env bash
# Builds a workbook exercising every `cellIs` operator the renderer
# supports today: greaterThan, greaterThanOrEqual, lessThan,
# lessThanOrEqual, equal, notEqual, between, notBetween.
#
# Layout: each operator gets its own labelled column block. Column A
# names the operator; columns B..G hold the test values 1, 10, 50, 100,
# 200, "foo". The CF dxf paints matched cells red w/ bold white text so
# any miss stands out in a screenshot diff against `hsx`.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
F="${1:-$HERE/cell-is.xlsx}"
rm -f "$F"
hsx create "$F" >/dev/null

hsx eval "$F" - <<'JS'
const Sheets = GC.Spread.Sheets;
const Ops = Sheets.ConditionalFormatting.ComparisonOperators;
const Range = Sheets.Range;

// 6-wide value strip across columns B..G, repeated per row.
const values = [1, 10, 50, 100, 200, "foo"];
const headers = ["operator", "1", "10", "50", "100", "200", '"foo"'];

// Header row.
for (let c = 0; c < headers.length; c++) {
  sheet.getCell(0, c).value(headers[c]);
  sheet.getCell(0, c).font("bold 11pt Calibri");
}

// Each row: label + value strip + cellIs rule.
const cases = [
  { label: "greaterThan 50",            op: Ops.greaterThan,            args: [50] },
  { label: "greaterThanOrEqual 100",    op: Ops.greaterThanOrEqualsTo,  args: [100] },
  { label: "lessThan 50",               op: Ops.lessThan,               args: [50] },
  { label: "lessThanOrEqual 10",        op: Ops.lessThanOrEqualsTo,     args: [10] },
  { label: "equal 10",                  op: Ops.equalsTo,               args: [10] },
  { label: 'equal "foo"',               op: Ops.equalsTo,               args: ['foo'] },
  { label: "notEqual 50",               op: Ops.notEqualsTo,            args: [50] },
  { label: "between 10 100",            op: Ops.between,                args: [10, 100] },
  { label: "notBetween 10 100",         op: Ops.notBetween,             args: [10, 100] },
];

const dxfRed = { backColor: "#ff4444", foreColor: "#ffffff", fontWeight: "bold" };

for (let i = 0; i < cases.length; i++) {
  const r = i + 1;
  sheet.getCell(r, 0).value(cases[i].label);
  for (let c = 0; c < values.length; c++) {
    sheet.getCell(r, c + 1).value(values[c]);
  }
  // Range covers the value cells in this row only.
  const rng = new Range(r, 1, 1, values.length);
  sheet.conditionalFormats.addCellValueRule(
    cases[i].op, cases[i].args[0], cases[i].args[1] ?? null, dxfRed, [rng]
  );
}

// Widen column A so the labels read; standard width on the rest.
sheet.setColumnWidth(0, 200);
for (let c = 1; c <= 6; c++) sheet.setColumnWidth(c, 60);
JS

# hsx daemon caches writes; flush before reading on disk (or zipping).
hsx daemon flush >/dev/null 2>&1 || true
echo "wrote $F"
