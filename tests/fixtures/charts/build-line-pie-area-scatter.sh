#!/usr/bin/env bash
# tests/fixtures/charts/build-line-pie-area-scatter.sh
#
# Drops one workbook with four charts laid out side-by-side on Sheet1:
# line / area-stacked / pie / scatter. Same source data range so the
# rendering differences come from chart.type alone, not the numbers.
set -euo pipefail
F=${1:-$(dirname "$0")/line-pie-area-scatter.xlsx}
rm -f "$F"
hsx create "$F"

# Source data: 4 quarters, 3 series.
hsx set "$F" "Sheet1!A1:E1" '[
  [{"value":"Quarter","style":{"fontStyle":{"bold":true}}},
   {"value":"North","style":{"fontStyle":{"bold":true}}},
   {"value":"South","style":{"fontStyle":{"bold":true}}},
   {"value":"East","style":{"fontStyle":{"bold":true}}},
   {"value":"West","style":{"fontStyle":{"bold":true}}}]
]'
hsx set "$F" "Sheet1!A2:E5" '[
  [{"value":"Q1"},{"value":120},{"value":98}, {"value":145},{"value":88}],
  [{"value":"Q2"},{"value":138},{"value":110},{"value":158},{"value":94}],
  [{"value":"Q3"},{"value":151},{"value":121},{"value":172},{"value":102}],
  [{"value":"Q4"},{"value":169},{"value":135},{"value":189},{"value":119}]
]'

# Numeric x/y for the scatter chart (rows 8..13, cols A:B).
hsx set "$F" "Sheet1!A8:B8" '[[{"value":"X","style":{"fontStyle":{"bold":true}}},{"value":"Y","style":{"fontStyle":{"bold":true}}}]]'
hsx set "$F" "Sheet1!A9:B14" '[
  [{"value":1.2}, {"value":3.5}],
  [{"value":2.8}, {"value":4.1}],
  [{"value":3.5}, {"value":2.9}],
  [{"value":4.7}, {"value":5.6}],
  [{"value":5.1}, {"value":6.2}],
  [{"value":6.4}, {"value":5.0}]
]'

# 4 charts: line, stacked area, pie (Q4 by region), scatter.
hsx eval "$F" '
  const sht = workbook.getSheet(0);
  const T = GC.Spread.Sheets.Charts.ChartType;

  const c1 = sht.charts.add("line",   T.line,            350,   0, 380, 240, "A1:E5");
  c1.title({text:"Sales — line"});

  const c2 = sht.charts.add("area",   T.areaStacked,     350, 250, 380, 240, "A1:E5");
  c2.title({text:"Sales — area (stacked)"});

  const c3 = sht.charts.add("pie",    T.pie,             740,   0, 380, 240, "A4:E4");
  c3.title({text:"Q3 share"});

  const c4 = sht.charts.add("scat",   T.xyScatter,       740, 250, 380, 240, "A8:B14");
  c4.title({text:"Scatter"});
'

# Frozen header so the charts always render the same way regardless of
# scroll position when we screenshot.
hsx eval "$F" 'workbook.getSheet(0).frozenRowCount(1);'

echo "Built $F"
ls -la "$F"
