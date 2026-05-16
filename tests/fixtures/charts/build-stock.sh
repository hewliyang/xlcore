#!/usr/bin/env bash
# Build small stock-chart fixtures covering the HLC and OHLC subtypes.
# xlsx-preview renders these via `chartAdvanced.ts::drawStockChart` —
# vertical hi-low marks (`<c:hiLowLines/>` from xlsxwriter's default
# stock layout) plus open/close up-down bars (OHLC only).
#
# Series-count → subtype mapping in the renderer:
#   3 series → HLC  [high, low, close]
#   4 series → OHLC [open, high, low, close]
#
# xlsxwriter authors `<c:marker><c:symbol val="none"/></c:marker>` on
# high/low (so only the hi-low line shows them) and `<c:symbol val=
# "dot"/>` on close — `markerSymbol === "none"` suppression in the
# painter gives us "close-only" markers for free.
set -euo pipefail
OUT_DIR=${1:-$(dirname "$0")}
python3 - "$OUT_DIR" <<'PY'
from pathlib import Path
import sys, xlsxwriter

out = Path(sys.argv[1]); out.mkdir(parents=True, exist_ok=True)

# Shared synthetic OHLC data (5 days, monotonic-ish rally then dip).
OHLC = [
    # Day, Open, High, Low, Close
    ("Mon", 24, 27, 23, 25),
    ("Tue", 25, 28, 24, 27),
    ("Wed", 27, 30, 26, 29),
    ("Thu", 29, 29, 25, 26),
    ("Fri", 26, 31, 26, 30),
]

# ---- HLC (3 series) -----------------------------------------------------
p = out / "chart-stock-hlc.xlsx"
wb = xlsxwriter.Workbook(str(p))
ws = wb.add_worksheet("Sheet1")
ws.write_row(0, 0, ["Date", "High", "Low", "Close"])
for r, (d, _o, h, l, c) in enumerate(OHLC, 1):
    ws.write_row(r, 0, [d, h, l, c])
ch = wb.add_chart({"type": "stock"})
for col, name in [(1, "High"), (2, "Low"), (3, "Close")]:
    ch.add_series({
        "name":       f"=Sheet1!${chr(65+col)}$1",
        "categories": "=Sheet1!$A$2:$A$6",
        "values":     f"=Sheet1!${chr(65+col)}$2:${chr(65+col)}$6",
    })
ch.set_title({"name": "Stock (HLC)"})
ch.set_size({"width": 560, "height": 340})
ws.insert_chart("F2", ch)
wb.close()
print(p)

# ---- OHLC (4 series) ----------------------------------------------------
# xlsxwriter doesn't auto-emit `<c:upDownBars/>` for OHLC, so we author
# four series and hand-patch the chart XML post-write to inject the
# up/down bars element that distinguishes OHLC from HLC visually.
import zipfile, shutil, tempfile, os
p = out / "chart-stock-ohlc.xlsx"
wb = xlsxwriter.Workbook(str(p))
ws = wb.add_worksheet("Sheet1")
ws.write_row(0, 0, ["Date", "Open", "High", "Low", "Close"])
for r, row in enumerate(OHLC, 1):
    ws.write_row(r, 0, list(row))
ch = wb.add_chart({"type": "stock"})
# xlsxwriter only suppresses markers on series 0/1 for OHLC (Open,
# High), leaving Low with a default marker. Excel's actual OHLC
# convention paints markers only on Close, so we override per-series.
for col, name in [(1, "Open"), (2, "High"), (3, "Low"), (4, "Close")]:
    opts = {
        "name":       f"=Sheet1!${chr(65+col)}$1",
        "categories": "=Sheet1!$A$2:$A$6",
        "values":     f"=Sheet1!${chr(65+col)}$2:${chr(65+col)}$6",
    }
    if name in ("Open", "High", "Low"):
        opts["marker"] = {"type": "none"}
    ch.add_series(opts)
ch.set_title({"name": "Stock (OHLC)"})
ch.set_size({"width": 560, "height": 340})
ws.insert_chart("F2", ch)
wb.close()

# Post-process: inject `<c:upDownBars/>` before `<c:axId>` inside
# `<c:stockChart>`. ECMA-376 §21.2.2.207 (sequence: hiLowLines,
# upDownBars, axId, extLst). xlsxwriter already emits `<c:hiLowLines/>`.
with tempfile.TemporaryDirectory() as tmp:
    tmp = Path(tmp)
    with zipfile.ZipFile(p, "r") as z:
        z.extractall(tmp)
    chart_xml = tmp / "xl" / "charts" / "chart1.xml"
    text = chart_xml.read_text()
    if "<c:upDownBars" not in text:
        text = text.replace("<c:axId", "<c:upDownBars><c:gapWidth val=\"150\"/><c:upBars/><c:downBars/></c:upDownBars><c:axId", 1)
        chart_xml.write_text(text)
    # Rewrite the .xlsx in a deterministic order.
    out_tmp = p.with_suffix(".tmp.xlsx")
    with zipfile.ZipFile(out_tmp, "w", zipfile.ZIP_DEFLATED) as z:
        for root, _, files in os.walk(tmp):
            for f in files:
                fp = Path(root) / f
                z.write(fp, fp.relative_to(tmp))
    shutil.move(str(out_tmp), str(p))
print(p)
PY
