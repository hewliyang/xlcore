#!/usr/bin/env bash
# Build small fixtures exercising the legacy 3D chart variants
# (`bar3DChart`, `line3DChart`, `area3DChart`, `pie3DChart`) plus
# `ofPieChart`. xlsx-preview drops the 3D-only perspective/depth
# flourishes and renders these via the 2D painters — see
# `docs/parity-charts.md` priority order item #1.
set -euo pipefail
OUT_DIR=${1:-$(dirname "$0")}
python3 - "$OUT_DIR" <<'PY'
from pathlib import Path
import sys
from openpyxl import Workbook
from openpyxl.chart import (
    BarChart3D, LineChart3D, AreaChart3D, PieChart3D, ProjectedPieChart,
    Reference,
)

out = Path(sys.argv[1]); out.mkdir(parents=True, exist_ok=True)

def fill(name, ChartCls, **kw):
    wb = Workbook(); ws = wb.active; ws.title = "Sheet1"
    ws.append(["Quarter", "Services", "Manpower"])
    for row in [["Q1", 35, 20], ["Q2", 45, 24], ["Q3", 52, 29], ["Q4", 60, 33]]:
        ws.append(row)
    ch = ChartCls()
    for k, v in kw.items(): setattr(ch, k, v)
    ch.title = name
    data = Reference(ws, min_col=2, min_row=1, max_col=3, max_row=5)
    cats = Reference(ws, min_col=1, min_row=2, max_row=5)
    ch.add_data(data, titles_from_data=True)
    ch.set_categories(cats)
    ch.height = 8; ch.width = 14
    ws.add_chart(ch, "F2")
    p = out / f"chart-3d-{name.lower().replace(' ', '-')}.xlsx"
    wb.save(p); print(p)

fill("bar3D", BarChart3D, type="col", grouping="clustered")
fill("line3D", LineChart3D)
fill("area3D", AreaChart3D)

# Pie3D needs single-series data: trim down.
def pie_book(ChartCls, name):
    wb = Workbook(); ws = wb.active; ws.title = "Sheet1"
    ws.append(["Quarter", "Services"])
    for row in [["Q1", 35], ["Q2", 45], ["Q3", 52], ["Q4", 60]]:
        ws.append(row)
    ch = ChartCls()
    ch.title = name
    data = Reference(ws, min_col=2, min_row=1, max_row=5)
    cats = Reference(ws, min_col=1, min_row=2, max_row=5)
    ch.add_data(data, titles_from_data=True)
    ch.set_categories(cats)
    ch.height = 8; ch.width = 14
    ws.add_chart(ch, "F2")
    p = out / f"chart-3d-{name.lower().replace(' ', '-')}.xlsx"
    wb.save(p); print(p)

pie_book(PieChart3D, "pie3D")
pie_book(ProjectedPieChart, "ofPie")

# openpyxl emits `<c:splitType val="auto"/>` on the default
# ProjectedPieChart. ECMA-376 §21.2.3.40 lists `auto` but the pinned
# ooxmlsdk 0.6.1 `SplitValues` enum doesn't, so reading the fixture
# bombs with `invalid enum value while parsing SplitValues: "auto"`.
# Drop the offending element until ooxmlsdk catches up. (Renderer
# doesn't read splitType yet either.)
import re, zipfile, shutil, tempfile, os
p = out / "chart-3d-ofpie.xlsx"
with tempfile.TemporaryDirectory() as d:
    with zipfile.ZipFile(p) as zin:
        zin.extractall(d)
    cp = Path(d, "xl", "charts", "chart1.xml")
    if cp.exists():
        x = cp.read_text(encoding="utf-8")
        x = re.sub(r"<c:splitType[^/]*/>", "", x)
        x = re.sub(r"<splitType[^/]*/>", "", x)
        cp.write_text(x, encoding="utf-8")
    tmp = p.with_suffix(".tmp.xlsx")
    with zipfile.ZipFile(tmp, "w", zipfile.ZIP_DEFLATED) as zout:
        for root, _, files in os.walk(d):
            for fn in files:
                full = Path(root, fn)
                zout.write(full, full.relative_to(d).as_posix())
    shutil.move(tmp, p)
PY
