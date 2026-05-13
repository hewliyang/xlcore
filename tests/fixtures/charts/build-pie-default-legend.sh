#!/usr/bin/env bash
# tests/fixtures/charts/build-pie-default-legend.sh
#
# A vanilla pie chart with no explicit <c:dPt> slice colors. Excel / hsx
# paint slices from the default Office palette and the legend swatches should
# use the same per-slice colors. This catches regressions where a slice-keyed
# legend falls back to the single series color for every entry.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
F="${1:-$HERE/pie-default-legend.xlsx}"
rm -f "$F"
hsx create "$F" >/dev/null

hsx set "$F" "Sheet1!A1:F1" '[
  [{"value":"Quarter","style":{"fontStyle":{"bold":true}}},
   {"value":"North","style":{"fontStyle":{"bold":true}}},
   {"value":"South","style":{"fontStyle":{"bold":true}}},
   {"value":"East","style":{"fontStyle":{"bold":true}}},
   {"value":"West","style":{"fontStyle":{"bold":true}}},
   {"value":"Central","style":{"fontStyle":{"bold":true}}}]
]'
hsx set "$F" "Sheet1!A2:F2" '[
  [{"value":"Q3"},{"value":151},{"value":121},{"value":172},{"value":102},{"value":88}]
]'

hsx eval "$F" '
  const sht = workbook.getSheet(0);
  const T = GC.Spread.Sheets.Charts.ChartType;
  const c = sht.charts.add("pie", T.pie, 0, 80, 520, 320, "A1:F2");
  c.title({text:"Q3 share — default slice palette"});
'

hsx daemon flush >/dev/null 2>&1 || true

echo "Built $F"
ls -la "$F"
