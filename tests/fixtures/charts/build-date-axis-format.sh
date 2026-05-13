#!/usr/bin/env bash
# tests/fixtures/charts/build-date-axis-format.sh
#
# Line chart with numeric date serials as category labels. The chart XML
# carries cached category numbers; the extractor must still surface the
# category number format (from cache or source-cell style) so the renderer
# paints dates instead of raw serials.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
F="${1:-$HERE/date-axis-format.xlsx}"
rm -f "$F"
hsx create "$F" >/dev/null

hsx set "$F" "Sheet1!A1:B1" '[[
  {"value":"Date","style":{"fontStyle":{"bold":true}}},
  {"value":"Revenue","style":{"fontStyle":{"bold":true}}}
]]'
hsx set "$F" "Sheet1!A2:B5" '[
  [{"value":45658,"style":{"formatter":"mmm d"}}, {"value":120}],
  [{"value":45689,"style":{"formatter":"mmm d"}}, {"value":138}],
  [{"value":45717,"style":{"formatter":"mmm d"}}, {"value":151}],
  [{"value":45748,"style":{"formatter":"mmm d"}}, {"value":169}]
]'

hsx eval "$F" '
  const sht = workbook.getSheet(0);
  const T = GC.Spread.Sheets.Charts.ChartType;
  const c = sht.charts.add("date-line", T.line, 0, 110, 560, 320, "A1:B5");
  c.title({text:"Revenue by date"});
'

hsx daemon flush >/dev/null 2>&1 || true

echo "Built $F"
ls -la "$F"
