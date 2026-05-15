#!/usr/bin/env bash
# Regression: line + area charts whose data exceed a workbook-pinned
# `<c:scaling><c:max>`. Excel clips line strokes / area fills to the
# plot rectangle; xlsx-preview previously painted past the topmost
# gridline because the y-coord came directly from
# `(v - minV) / (maxV - minV)` with no clamp.
set -euo pipefail
OUT_DIR=${1:-$(dirname "$0")}
python3 - "$OUT_DIR" <<'PY'
from pathlib import Path
import sys, xlsxwriter
out = Path(sys.argv[1]); out.mkdir(parents=True, exist_ok=True)
p = out / 'chart-line-area-overflow-clip.xlsx'

wb = xlsxwriter.Workbook(str(p))
ws = wb.add_worksheet('Sheet1')
ws.write_row('A1', ['x', 'line', 'area'])
# B/C/D spike past the pinned max of 100; E returns inside the range.
for r, row in enumerate(
    [['A', 60, 60], ['B', 80, 80], ['C', 130, 130], ['D', 150, 150], ['E', 95, 95]], 1):
    ws.write(r, 0, row[0]); ws.write(r, 1, row[1]); ws.write(r, 2, row[2])

ch1 = wb.add_chart({'type': 'line'})
ch1.add_series({
    'name': 'line',
    'categories': '=Sheet1!$A$2:$A$6',
    'values': '=Sheet1!$B$2:$B$6',
    'line': {'color': '#4472C4', 'width': 2.25},
    'marker': {'type': 'circle', 'size': 6},
})
ch1.set_y_axis({'min': 0, 'max': 100})
ch1.set_title({'name': 'Line over pinned max'})
ch1.set_legend({'none': True})
ch1.set_size({'width': 520, 'height': 300})
ws.insert_chart('E2', ch1)

ch2 = wb.add_chart({'type': 'area'})
ch2.add_series({
    'name': 'area',
    'categories': '=Sheet1!$A$2:$A$6',
    'values': '=Sheet1!$C$2:$C$6',
    'fill': {'color': '#ED7D31'},
})
ch2.set_y_axis({'min': 0, 'max': 100})
ch2.set_title({'name': 'Area over pinned max'})
ch2.set_legend({'none': True})
ch2.set_size({'width': 520, 'height': 300})
ws.insert_chart('E18', ch2)
wb.close()
print(p)
PY
