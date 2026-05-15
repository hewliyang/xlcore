#!/usr/bin/env bash
# Regression: stacked column chart with `<c:scaling><c:max val="100"/>` on
# the value axis whose category sums (106, 118, 127, 136, 141) exceed
# the pinned max. Native Excel clips bar fills to the plot rectangle —
# any segment above `100` is invisible. xlsx-preview previously painted
# bar fills past the topmost gridline.
#
# Mirrors the symptom from the AGS Metrics Model "Charts" sheet,
# middle stacked-column chart at I32:Q46 (RFS+KC+KH+Ashtree per year).
set -euo pipefail
OUT_DIR=${1:-$(dirname "$0")}
python3 - "$OUT_DIR" <<'PY'
from pathlib import Path
import sys, xlsxwriter
out = Path(sys.argv[1]); out.mkdir(parents=True, exist_ok=True)
p = out / 'chart-stacked-overflow-clip.xlsx'

wb = xlsxwriter.Workbook(str(p))
ws = wb.add_worksheet('Sheet1')
ws.write_row('A1', ['Year', 'RFS', 'KC+KH', 'Ashtree'])
rows = [
    ['2019', 53, 47, 6],
    ['2020', 56, 55, 7],
    ['2021', 58, 61, 8],
    ['2022', 60, 67, 9],
    ['2023', 62, 70, 9],
]
for r, row in enumerate(rows, 1):
    ws.write_row(r, 0, row)

ch = wb.add_chart({'type': 'column', 'subtype': 'stacked'})
for col, name, color in [
    (1, 'RFS', '#5B9BD5'),
    (2, 'KC+KH', '#ED7D31'),
    (3, 'Ashtree', '#A5A5A5'),
]:
    ch.add_series({
        'name': name,
        'categories': '=Sheet1!$A$2:$A$6',
        'values': f'=Sheet1!${chr(65+col)}$2:${chr(65+col)}$6',
        'fill': {'color': color},
    })
# Pin y-axis max below the stacked totals so overflow is forced.
ch.set_y_axis({'min': 0, 'max': 100})
ch.set_title({'name': 'Stacked overflow vs pinned y-axis max'})
ch.set_legend({'position': 'bottom'})
ch.set_size({'width': 560, 'height': 360})
ws.insert_chart('F2', ch)
wb.close()
print(p)
PY
