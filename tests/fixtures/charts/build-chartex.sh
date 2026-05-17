#!/usr/bin/env bash
# Build chartEx (`cx:` namespace, ECMA-376 part 4 §13) fixtures via
# SpreadJS (`hsx eval`). xlsxwriter / openpyxl cannot author chartEx —
# the body uses opaque `_xlchart.vN.X` aliases that resolve through
# `workbook.xml`'s hidden `<definedName>` entries, plus paired
# `chartStyle` / `colorStyle` parts. SpreadJS round-trips all of this
# natively when you `addChart(..., ChartType.{funnel,treemap,sunburst})`
# and save as xlsx — verified by `unzip -l` showing
# `xl/charts/chartEx1.xml` and a `cx:` body with the expected
# `layoutId="..."` value.
#
# Generated fixtures (priority order item #6 in `docs/parity-charts.md`):
#   chart-funnel-chartex.xlsx       (ChartType.funnel    = 54, layoutId="funnel")
#   chart-treemap-chartex.xlsx      (ChartType.treemap   = 58, layoutId="treemap")
#   chart-sunburst-chartex.xlsx     (ChartType.sunburst  = 57, layoutId="sunburst")
#
# The existing waterfall chartEx fixture (`chart-waterfall-chartex.xlsx`)
# was authored in Excel desktop and is left alone — keeping at least
# one Excel-desktop-authored fixture is useful as a sanity check that
# our extractor handles both Excel's and SpreadJS's chartEx flavors.
#
# Why only three layouts via SpreadJS
# -----------------------------------
# SpreadJS *can* author paretoLine, boxWhisker, clusteredColumn (histogram),
# and regionMap chartEx parts, but the round-trip is unreliable:
#
#  - paretoLine: SpreadJS omits `<cx:axis>` blocks entirely. Excel-authored
#    pareto charts have two axes (`valueAxis` for bars, secondary for the
#    cumulative line). hsx fails to re-import its own export with
#    `Cannot read properties of undefined (reading 'val')` — and even if
#    we hand-patched the axes in, the per-bar percentage scaling that the
#    cumulative line needs isn't authored.
#  - boxWhisker: SpreadJS writes a chartEx body with `layoutId="boxWhisker"`
#    but with one series per category column. When re-rendered (in
#    SpreadJS itself), it degenerates to a clustered-column chart of the
#    raw observations rather than quartile boxes. No useful ground truth.
#  - clusteredColumn (histogram, =60): SpreadJS does *not* auto-bin —
#    each observation becomes its own bar, defeating the point of the
#    histogram layout.
#  - regionMap: requires Bing Maps geographic resolution; not worth
#    synthesizing in a minimal fixture.
#
# These four belong to a future "Excel-desktop-authored chartEx" batch
# (same path as `chart-waterfall-chartex.xlsx`). Funnel / treemap /
# sunburst already exercise the chartEx extractor + the
# `cx:numDim type=size` (treemap/sunburst) and `cx:numDim type=val`
# (funnel) data-binding code paths, which is the bulk of the new
# painter-shared surface area.
set -euo pipefail

OUT_DIR=${1:-$(dirname "$0")}
mkdir -p "$OUT_DIR"

# Make sure no stale daemon-cached copy of the same path is around.
hsx daemon stop >/dev/null 2>&1 || true

# Build one chartEx fixture. Args: <out-path> <eval-script>
build_one() {
  local out="$1"; local script="$2"
  rm -f "$out"
  hsx --no-daemon create "$out" >/dev/null
  hsx --no-daemon eval "$out" "$script" >/dev/null
  echo "$out"
}

# ---- funnel (single descending series) ---------------------------------
build_one "$OUT_DIR/chart-funnel-chartex.xlsx" '
const s = workbook.getActiveSheet();
s.setValue(0,0,"Stage"); s.setValue(0,1,"Count");
[["Visit",1000],["Signup",450],["Trial",220],["Paid",90],["Renewed",40]]
  .forEach(([n,v],i)=>{ s.setValue(i+1,0,n); s.setValue(i+1,1,v); });
s.charts.add("Chart1", GC.Spread.Sheets.Charts.ChartType.funnel,
  200, 10, 520, 340, "A1:B6");
"ok"
'

# ---- treemap (one-level hierarchy + value) -----------------------------
build_one "$OUT_DIR/chart-treemap-chartex.xlsx" '
const s = workbook.getActiveSheet();
s.setValue(0,0,"Region"); s.setValue(0,1,"Country"); s.setValue(0,2,"GDP");
const rows = [
  ["Americas","USA",27],["Americas","Brazil",2.1],["Americas","Canada",2.1],
  ["EMEA","Germany",4.5],["EMEA","UK",3.3],["EMEA","France",3.1],
  ["APAC","China",17.7],["APAC","Japan",4.2],["APAC","India",3.7],
];
rows.forEach((r,i)=>{ r.forEach((v,c)=>s.setValue(i+1,c,v)); });
s.charts.add("Chart1", GC.Spread.Sheets.Charts.ChartType.treemap,
  240, 10, 540, 380, "A1:C10");
"ok"
'

# ---- sunburst (two-level hierarchy + value) ----------------------------
build_one "$OUT_DIR/chart-sunburst-chartex.xlsx" '
const s = workbook.getActiveSheet();
s.setValue(0,0,"Quarter"); s.setValue(0,1,"Month"); s.setValue(0,2,"Sales");
const rows = [
  ["Q1","Jan",12],["Q1","Feb",15],["Q1","Mar",18],
  ["Q2","Apr",22],["Q2","May",25],["Q2","Jun",30],
  ["Q3","Jul",28],["Q3","Aug",26],["Q3","Sep",24],
  ["Q4","Oct",20],["Q4","Nov",27],["Q4","Dec",35],
];
rows.forEach((r,i)=>{ r.forEach((v,c)=>s.setValue(i+1,c,v)); });
s.charts.add("Chart1", GC.Spread.Sheets.Charts.ChartType.sunburst,
  240, 10, 540, 380, "A1:C13");
"ok"
'

# Make sure the daemon is down so re-running the script in CI doesn't
# pick up an interim cached copy.
hsx daemon stop >/dev/null 2>&1 || true
echo "done: $OUT_DIR"
