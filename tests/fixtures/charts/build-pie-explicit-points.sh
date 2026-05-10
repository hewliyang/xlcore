#!/usr/bin/env bash
# tests/fixtures/charts/build-pie-explicit-points.sh
#
# A single-sheet workbook with one pie chart whose slices each carry an
# explicit `<c:dPt>` fill. This is what Excel writes when the user
# right-clicks a slice and picks "Format Data Point... → Fill". hsx
# (SpreadJS) doesn't expose a writer for per-slice colors, so we let
# hsx lay down a vanilla pie and then post-patch the chart XML to
# inject the `<c:dPt>` blocks via Python zip-rewrite (same pattern as
# tests/fixtures/cf/build-data-bar.sh and tests/fixtures/themes/_patch_theme.py).
#
# Targets: extractor surfaces `ChartSeries.pointColors[i]` and the
# renderer paints each slice with that color instead of cycling the
# default 6-color palette.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
F="${1:-$HERE/pie-explicit-points.xlsx}"
rm -f "$F"
hsx create "$F" >/dev/null

# Source data: 5 categories, one Q3 row.
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
  const c = sht.charts.add("pie", T.pie, 0, 80, 480, 320, "A1:F2");
  c.title({text:"Q3 share — explicit slice colors"});
'

hsx daemon flush >/dev/null 2>&1 || true

# Inject five <c:dPt> blocks into the pie series so each slice gets a
# distinct hand-picked color (deep red, teal, gold, indigo, mint). The
# pie series sits inside the only <c:pieChart> block; we splice the
# dPts in right after <c:tx>...</c:tx> (always present, single match).
python3 - "$F" <<'PY'
import sys, zipfile, shutil, re
path = sys.argv[1]
COLORS = ["B22222", "008080", "DAA520", "4B0082", "20B2AA"]  # 5 slices
def make_dpts():
    parts = []
    for i, hex_ in enumerate(COLORS):
        parts.append(
            f'<c:dPt><c:idx val="{i}"/><c:bubble3D val="0"/>'
            f'<c:spPr><a:solidFill><a:srgbClr val="{hex_}"/></a:solidFill>'
            f'</c:spPr></c:dPt>'
        )
    return "".join(parts)

# The chart we want is the first (and only) one. Find it by content.
with zipfile.ZipFile(path, 'r') as z:
    chart_names = [n for n in z.namelist() if re.match(r"xl/charts/chart\d+\.xml$", n)]
    assert len(chart_names) == 1, chart_names
    chart_name = chart_names[0]
    xml = z.read(chart_name).decode()

# Locate the pie series: <c:pieChart>...<c:ser>...<c:tx>...</c:tx>(<c:dPts here>)...</c:ser>
pie_match = re.search(r"<c:pieChart\b[^>]*>.*?</c:pieChart>", xml, flags=re.S)
assert pie_match, "no <c:pieChart>"
pie_block = pie_match.group(0)
# Per OOXML schema, <c:dPt> belongs between <c:explosion> and <c:cat>;
# SpreadJS emits <c:explosion> at the very end of the series instead, so
# the safest splice point is right before <c:cat> (where dPt is allowed
# in xs:sequence and ooxmlsdk parses it deterministically).
patched_pie, n = re.subn(
    r"(<c:ser\b[^>]*>.*?)(<c:cat\b)",
    lambda m: m.group(1) + make_dpts() + m.group(2),
    pie_block, count=1, flags=re.S,
)
assert n == 1, "did not splice <c:dPt> blocks"
patched = xml.replace(pie_block, patched_pie)

tmp = path + ".new"
with zipfile.ZipFile(path, 'r') as zin, zipfile.ZipFile(tmp, 'w', zipfile.ZIP_DEFLATED) as zout:
    for it in zin.namelist():
        data = patched.encode() if it == chart_name else zin.read(it)
        zout.writestr(it, data)
shutil.move(tmp, path)
PY

echo "Built $F"
ls -la "$F"
