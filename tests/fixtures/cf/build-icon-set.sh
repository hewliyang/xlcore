#!/usr/bin/env bash
# Builds a workbook exercising the `iconSet` CF kind across the variants
# our renderer should handle in v0:
#
#   row 1: 3 traffic-lights (unrimmed)  + auto thresholds (default 33/67%)
#   row 2: 3 arrows (colored)           + auto thresholds
#   row 3: 3 symbols (circled)          + numeric thresholds at v>=33, v>=67
#   row 4: 4 ratings                    + percent thresholds 25/50/75
#   row 5: 5 arrows (colored)           + percent thresholds 20/40/60/80
#   row 6: 5 quarters                   + auto thresholds
#   row 7: 3 traffic-lights, reverse=true   (high values get red)
#   row 8: 3 traffic-lights, showIconOnly  (hides value)
#
# Pixel-diff target against `hsx`. Open known divergences (legacy x14
# extensions, fancy presets) live in TRIAGE.md.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
F="${1:-$HERE/icon-set.xlsx}"
rm -f "$F"
hsx create "$F" >/dev/null

hsx eval "$F" - <<'JS'
const Sheets = GC.Spread.Sheets;
const Cf = Sheets.ConditionalFormatting;
const IST = Cf.IconSetType;
const IVT = Cf.IconValueType;
const Range = Sheets.Range;

const headers = ["label", "v1", "v2", "v3", "v4", "v5"];
for (let c = 0; c < headers.length; c++) {
  sheet.getCell(0, c).value(headers[c]);
  sheet.getCell(0, c).font("bold 11pt Calibri");
}

// Each row: [label, values[], iconSetType, criteria-builder, opts].
// Criteria builder receives the rule and may set thresholds; null = leave defaults.
const rows = [
  ["3 lights",   [10, 30, 50, 70, 90, 100],   IST.threeTrafficLightsUnrimmed, null, {}],
  ["3 arrows",   [10, 30, 50, 70, 90, 100],   IST.threeArrowsColored,         null, {}],
  // hsx writes user-set iconCriteria as empty `<cfvo/>` (the data
  // actually lives in the x14 extension, which it skips). Stick with
  // SpreadJS's defaults — they match Excel: 3-set 33/67%, 4-set
  // 25/50/75%, 5-set 20/40/60/80%.
  ["3 symbols",  [10, 30, 50, 70, 90, 100],   IST.threeSymbolsCircled,    null, {}],
  ["4 ratings",  [10, 30, 50, 70, 90, 100],   IST.fourRatings,            null, {}],
  ["5 arrows",   [10, 30, 50, 70, 90, 100],   IST.fiveArrowsColored,      null, {}],
  ["5 quarters", [10, 30, 50, 70, 90, 100],   IST.fiveQuarters,               null, {}],
  ["reverse",    [10, 30, 50, 70, 90, 100],   IST.threeTrafficLightsUnrimmed, null, { reverse: true }],
  ["iconOnly",   [10, 30, 50, 70, 90, 100],   IST.threeTrafficLightsUnrimmed, null, { iconOnly: true }],
];

for (let i = 0; i < rows.length; i++) {
  const [label, values, iconSetType, build, opts] = rows[i];
  const r = i + 1;
  sheet.getCell(r, 0).value(label);
  for (let c = 0; c < values.length; c++) {
    sheet.getCell(r, c + 1).value(values[c]);
  }
  const rule = new Cf.IconSetRule();
  rule.iconSetType(iconSetType);
  if (build) build(rule);
  if (opts.reverse)  rule.reverseIconOrder(true);
  if (opts.iconOnly) rule.showIconOnly(true);
  rule.ranges([new Range(r, 1, 1, values.length)]);
  sheet.conditionalFormats.addRule(rule);
}

sheet.setColumnWidth(0, 100);
for (let c = 1; c <= 6; c++) sheet.setColumnWidth(c, 70);

JS

# hsx screenshot of the fixture, narrowed to the table.
hsx screenshot "$F" "Sheet1!A1:G9" -o "$HERE/icon-set.hsx.png" >/dev/null

echo "wrote $F + icon-set.hsx.png"
