#!/usr/bin/env bash
# Builds a worksheet-level AutoFilter fixture with a saved filter result.
# Excel stores rows hidden by an applied AutoFilter as ordinary
# <row hidden="1"/> entries; the worksheet <autoFilter ref="..."> range
# drives the header dropdown chevrons.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
F="${1:-$HERE/autofilter-hidden-rows.xlsx}"
rm -f "$F"

python3 - "$F" <<'PY'
import sys
from openpyxl import Workbook
from openpyxl.styles import Font, PatternFill
from openpyxl.utils import get_column_letter

path = sys.argv[1]
wb = Workbook()
ws = wb.active
ws.title = "Filtered"

headers = ["Region", "Product", "Amount"]
rows = [
    ("North", "Apples", 120),
    ("South", "Apples", 90),
    ("North", "Pears", 135),
    ("West", "Pears", 70),
    ("East", "Apples", 105),
    ("North", "Grapes", 150),
]
ws.append(headers)
for row in rows:
    ws.append(row)

# Plain worksheet-level AutoFilter (not a ListObject table). Keep only
# Region == North visible, and serialize that saved result by hiding the
# non-matching data rows. This mirrors Excel after applying the filter
# and saving the workbook.
ws.auto_filter.ref = "A1:C7"
ws.auto_filter.add_filter_column(0, ["North"])
for excel_row, data in enumerate(rows, start=2):
    if data[0] != "North":
        ws.row_dimensions[excel_row].hidden = True

for c in range(1, 4):
    cell = ws.cell(1, c)
    cell.font = Font(bold=True, color="FFFFFF")
    cell.fill = PatternFill("solid", fgColor="4472C4")
    ws.column_dimensions[get_column_letter(c)].width = [14, 14, 10][c - 1]

ws.freeze_panes = "A2"
wb.save(path)
PY

echo "wrote $F"
