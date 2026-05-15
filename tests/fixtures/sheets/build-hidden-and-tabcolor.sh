#!/usr/bin/env bash
# Fixture: workbook with mixed sheet visibility + tab colors.
#
# Exercises two PARITY items at once:
#   * Hidden sheets — extractor surfaces `state`, renderer omits hidden
#     tabs and never shows veryHidden.
#   * Tab chrome — extractor surfaces `<sheetPr><tabColor/>`, renderer
#     paints a tinted stripe per tab.
#
# Sheets (left-to-right):
#   1. "Visible Red"     — visible, tabColor srgb FF0000 (no theme).
#   2. "Visible Theme"   — visible, tabColor theme=4 (accent1) tint=-0.25.
#   3. "Plain"           — visible, no tabColor (control).
#   4. "Hidden Sheet"    — state=hidden, tabColor srgb 00AA00.
#   5. "VeryHidden"      — state=veryHidden, no tabColor.
# The workbook's activeTab points at sheet 2 ("Visible Theme") so we
# also exercise activeSheetIndex pass-through.

set -euo pipefail
F=${1:-$(dirname "$0")/hidden-and-tabcolor.xlsx}
DIR=$(cd "$(dirname "$0")" && pwd)
rm -f "$F"

python3 - "$F" <<'PY'
import sys
from openpyxl import Workbook
from openpyxl.styles import Color

path = sys.argv[1]
wb = Workbook()

# Sheet 1 — visible, srgb red tab.
s1 = wb.active
s1.title = "Visible Red"
s1["A1"] = "Visible Red sheet"
s1["A2"] = "tab: srgb FF0000"
s1.sheet_properties.tabColor = Color(rgb="FFFF0000")

# Sheet 2 — visible, theme accent1 tinted darker.
s2 = wb.create_sheet("Visible Theme")
s2["A1"] = "Visible Theme sheet"
s2["A2"] = "tab: theme=4 tint=-0.25"
tc = Color()
tc.type = "theme"
tc.theme = 4
tc.tint = -0.25
s2.sheet_properties.tabColor = tc

# Sheet 3 — plain, no tab color.
s3 = wb.create_sheet("Plain")
s3["A1"] = "Plain visible sheet (no tab color)"

# Sheet 4 — hidden, green tab.
s4 = wb.create_sheet("Hidden Sheet")
s4["A1"] = "If you can read this, hidden filtering is broken."
s4.sheet_properties.tabColor = Color(rgb="FF00AA00")
s4.sheet_state = "hidden"

# Sheet 5 — veryHidden.
s5 = wb.create_sheet("VeryHidden")
s5["A1"] = "VeryHidden — should never appear in the tab strip."
s5.sheet_state = "veryHidden"

# activeTab = 1 ("Visible Theme").
wb.active = 1

wb.save(path)
print(f"wrote {path}")
PY

echo "Built $F"
ls -la "$F"
