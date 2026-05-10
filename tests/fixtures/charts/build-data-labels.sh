#!/usr/bin/env bash
# tests/fixtures/charts/build-data-labels.sh
#
# Six charts side-by-side, each exercising a different `<c:dLbls>` shape:
#
#   1. column   — showValue, position=outEnd
#   2. bar      — showValue, position=ctr (in-bar centered)
#   3. line     — showValue, position=t  (above marker)
#   4. area     — showCategory + showValue
#   5. pie      — showPercent + showCategory, position=outEnd
#   6. scatter  — showValue at each xy point
#
# Both per-series and per-chart pathways are covered — Excel writes
# the dLbls block at the chart-group level when a single set of flags
# applies to every series, and per-series otherwise.
set -euo pipefail
F=${1:-$(dirname "$0")/data-labels.xlsx}
rm -f "$F"
hsx create "$F"

# Shared source data: 4 quarters x 3 regions.
hsx set "$F" "Sheet1!A1:D1" '[
  [{"value":"Quarter","style":{"fontStyle":{"bold":true}}},
   {"value":"North","style":{"fontStyle":{"bold":true}}},
   {"value":"South","style":{"fontStyle":{"bold":true}}},
   {"value":"East","style":{"fontStyle":{"bold":true}}}]
]'
hsx set "$F" "Sheet1!A2:D5" '[
  [{"value":"Q1"},{"value":120},{"value":98}, {"value":145}],
  [{"value":"Q2"},{"value":138},{"value":110},{"value":158}],
  [{"value":"Q3"},{"value":151},{"value":121},{"value":172}],
  [{"value":"Q4"},{"value":169},{"value":135},{"value":189}]
]'
# Scatter source.
hsx set "$F" "Sheet1!F1:G1" '[[{"value":"X","style":{"fontStyle":{"bold":true}}},{"value":"Y","style":{"fontStyle":{"bold":true}}}]]'
hsx set "$F" "Sheet1!F2:G7" '[
  [{"value":1.2}, {"value":3.5}],
  [{"value":2.8}, {"value":4.1}],
  [{"value":3.5}, {"value":2.9}],
  [{"value":4.7}, {"value":5.6}],
  [{"value":5.1}, {"value":6.2}],
  [{"value":6.4}, {"value":5.0}]
]'

hsx eval "$F" '
  const sht = workbook.getSheet(0);
  const T = GC.Spread.Sheets.Charts.ChartType;
  const Pos = GC.Spread.Sheets.Charts.DataLabelPosition;
  // SpreadJS enum: bestFit=0, below=1, center=2, insideBase=3,
  // insideEnd=4, left=5, outsideEnd=6, right=7, above=8.
  // OOXML mapping: above->t, below->b, center->ctr, insideBase->inBase,
  // insideEnd->inEnd, outsideEnd->outEnd, left->l, right->r, bestFit->bestFit.

  const apply = (chart, opts) => {
    const sers = chart.series().get();
    for (let i = 0; i < sers.length; i++) chart.series().set(i, { dataLabels: opts });
  };

  // 1. column: outsideEnd value labels
  let c = sht.charts.add("c1", T.columnClustered, 350, 0, 360, 240, "A1:D5");
  c.title({text:"Column \u2014 outsideEnd"});
  apply(c, { showValue: true, position: Pos.outsideEnd });

  // 2. bar: in-bar centered
  c = sht.charts.add("c2", T.barClustered, 720, 0, 360, 240, "A1:D5");
  c.title({text:"Bar \u2014 center"});
  apply(c, { showValue: true, position: Pos.center });

  // 3. line: above marker
  c = sht.charts.add("c3", T.line, 1090, 0, 360, 240, "A1:D5");
  c.title({text:"Line \u2014 above"});
  apply(c, { showValue: true, position: Pos.above });

  // 4. area: category + value, default position
  c = sht.charts.add("c4", T.areaStacked, 350, 250, 360, 240, "A1:D5");
  c.title({text:"Area \u2014 cat+val"});
  apply(c, { showCategoryName: true, showValue: true });

  // 5. pie: showPercent + showCategory, outside
  c = sht.charts.add("c5", T.pie, 720, 250, 360, 240, "A4:D4");
  c.title({text:"Pie \u2014 % + cat"});
  apply(c, { showPercentage: true, showCategoryName: true, position: Pos.outsideEnd });

  // 6. scatter: showValue at each point
  c = sht.charts.add("c6", T.xyScatter, 1090, 250, 360, 240, "F1:G7");
  c.title({text:"Scatter \u2014 value"});
  apply(c, { showValue: true });
'

hsx eval "$F" 'workbook.getSheet(0).frozenRowCount(1);'

# Force-flush hsx's daemon write buffer before we read/edit the file.
hsx daemon flush || true

# Workaround for an ooxmlsdk parse quirk: when SpreadJS emits a pie
# chart with `dLbls/showLeaderLines=true`, it writes the optional
# `<c:leaderLines>` block AFTER `<c:extLst>`. ooxmlsdk's `#[sdk(
# sequence)]` parser sees this trailing element and "restarts" the
# DataLabelsChoice::Sequence, which overwrites the previously-parsed
# show_value/show_category/etc. fields with None. Strip the trailing
# block so the pie chart's labels actually round-trip through extract.
# (Excel rebuilds leader lines automatically when a label is dragged,
# so this isn't lossy from a user-visible standpoint.)
python3 - <<PY
import zipfile, re, shutil, os
f = "$F"
tmp = f + ".tmp.zip"
with zipfile.ZipFile(f) as zin, zipfile.ZipFile(tmp, "w", zipfile.ZIP_DEFLATED) as zout:
  for item in zin.namelist():
    data = zin.read(item)
    if item == "xl/charts/chart5.xml":
      s = data.decode("utf-8")
      s2 = re.sub(r"<c:leaderLines>.*?</c:leaderLines>", "", s, count=1)
      data = s2.encode("utf-8")
    zout.writestr(item, data)
shutil.move(tmp, f)
PY

echo "Built $F"
ls -la "$F"
