#!/usr/bin/env bash
# tests/fixtures/charts/build-bubble.sh
#
# Single bubble chart on Sheet1. Source data: 6 points with X/Y/Size
# columns. Exercises:
#   - `<c:bubbleChart>` plotArea group dispatch
#   - per-series `<c:xVal>` / `<c:yVal>` / `<c:bubbleSize>` cached values
#   - default `<c:sizeRepresents val="area"/>` semantics
#   - default `<c:bubbleScale val="100"/>` (Excel writes it explicitly)
set -euo pipefail
F=${1:-$(dirname "$0")/bubble.xlsx}
rm -f "$F"
hsx create "$F"

hsx set "$F" "Sheet1!A1:C1" '[
  [{"value":"X","style":{"fontStyle":{"bold":true}}},
   {"value":"Y","style":{"fontStyle":{"bold":true}}},
   {"value":"Size","style":{"fontStyle":{"bold":true}}}]
]'
hsx set "$F" "Sheet1!A2:C7" '[
  [{"value":1.2},{"value":3.5},{"value":12}],
  [{"value":2.8},{"value":4.1},{"value":40}],
  [{"value":3.5},{"value":2.9},{"value":4}],
  [{"value":4.7},{"value":5.6},{"value":85}],
  [{"value":5.1},{"value":6.2},{"value":24}],
  [{"value":6.4},{"value":5.0},{"value":60}]
]'

# One bubble chart. ChartType.bubble = scatter w/ size dimension.
hsx eval "$F" '
  const sht = workbook.getSheet(0);
  const T = GC.Spread.Sheets.Charts.ChartType;
  const c = sht.charts.add("bubble", T.bubble, 250, 0, 480, 320, "A1:C7");
  c.title({text:"Bubble"});
'

echo "Built $F"
ls -la "$F"
