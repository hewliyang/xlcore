#!/usr/bin/env bash
# Build small radar-chart fixtures covering the three radarStyle values
# (`standard`, `marker`, `filled`). xlsx-preview renders each via the
# polar painter in `chartAdvanced.ts::drawRadarChart`. See
# `docs/parity-charts.md` priority order item #2.
#
# Note: we use xlsxwriter (not openpyxl) because openpyxl emits a
# `<marker><symbol val="none"/></marker>` on every radar series — even
# for `radarStyle="marker"` — which our renderer correctly honors (per
# ECMA-376 series-marker-override-wins semantics), giving the
# `marker` and `standard` fixtures indistinguishable output. xlsxwriter
# omits the per-series marker block, letting `radarStyle` drive whether
# markers paint.
set -euo pipefail
OUT_DIR=${1:-$(dirname "$0")}
python3 - "$OUT_DIR" <<'PY'
from pathlib import Path
import sys, xlsxwriter

out = Path(sys.argv[1]); out.mkdir(parents=True, exist_ok=True)

# xlsxwriter subtypes map to ECMA-376 radarStyle values:
#   radar               -> radarStyle="standard"   (line only)
#   radar with_markers  -> radarStyle="marker"     (Excel UI default)
#   radar filled        -> radarStyle="filled"     (filled polygon)
SUBTYPES = [
    ("standard", None),
    ("marker",   "with_markers"),
    ("filled",   "filled"),
]

ROWS = [
    ["Axis",        "Product A", "Product B"],
    ["Speed",        78, 62],
    ["Reliability",  88, 74],
    ["Comfort",      65, 81],
    ["Price",        55, 90],
    ["Style",        82, 70],
    ["Safety",       91, 80],
]

for style_name, subtype in SUBTYPES:
    p = out / f"chart-radar-{style_name}.xlsx"
    wb = xlsxwriter.Workbook(str(p))
    ws = wb.add_worksheet("Sheet1")
    for r, row in enumerate(ROWS):
        ws.write_row(r, 0, row)
    ch_opts = {"type": "radar"}
    if subtype is not None:
        ch_opts["subtype"] = subtype
    ch = wb.add_chart(ch_opts)
    for col, name in [(1, "Product A"), (2, "Product B")]:
        ch.add_series({
            "name":       f"=Sheet1!${chr(65+col)}$1",
            "categories": "=Sheet1!$A$2:$A$7",
            "values":     f"=Sheet1!${chr(65+col)}$2:${chr(65+col)}$7",
        })
    ch.set_title({"name": f"Radar ({style_name})"})
    ch.set_size({"width": 560, "height": 360})
    ws.insert_chart("F2", ch)
    wb.close()
    print(p)
PY
