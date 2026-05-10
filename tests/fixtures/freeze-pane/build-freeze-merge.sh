#!/usr/bin/env bash
# Fixture: a bordered merge that crosses both a vertical and horizontal
# freeze split. Locks in the renderer fix where merges spanning a freeze
# boundary used to drop fill/border on the panes that didn't contain the
# top-left cell.
#
# Layout (freeze at row 3, col 3):
#
#       A     B     C     D     E
#   1   .     .     .     .     .
#   2   .     .     .     .     .          <-- freeze boundary (horizontal)
#   3 [ MERGED A3:E3, yellow fill, thick black box border, "FY24 banner"  ]
#   4   .     .     .     .     .
#   5 [ MERGED B5:E5, sky-blue fill, medium border, "subheader"           ]
#   6 [ MERGED B6:D7  - 3-col x 2-row,  green fill, thin border,  "tile"  ]
#       ^                  ^
#       |                  freeze boundary (vertical) is between B and C
#       freeze boundary (vertical) is between C and D? actually leftCol=3
#
# Notes on splits: SpreadJS frozenColumnCount(2) means cols 0..1 (A,B) are
# pinned and the split lies between B and C. That's the case the bug hit:
# the A:E and B:E merges have their top-left in the pinned (left) pane
# while their right-hand cells are in the scrolling pane.

set -euo pipefail
F=${1:-$(dirname "$0")/freeze-merge.xlsx}
rm -f "$F"
hsx create "$F"

# Some content so the freeze panes are visually meaningful.
hsx set "$F" "Sheet1!A1:E1" '[
  [{"value":"col A"},{"value":"col B"},{"value":"col C"},{"value":"col D"},{"value":"col E"}]
]'
hsx set "$F" "Sheet1!A2:E2" '[
  [{"value":1},{"value":2},{"value":3},{"value":4},{"value":5}]
]'
hsx set "$F" "Sheet1!A4:E4" '[
  [{"value":"x"},{"value":"y"},{"value":"z"},{"value":"w"},{"value":"v"}]
]'

# Wide column widths so the merge artefacts are obvious if they reappear.
hsx eval "$F" '
  const s = workbook.getSheet(0);
  for (let c = 0; c < 5; c++) s.setColumnWidth(c, 100);
  for (let r = 0; r < 8; r++) s.setRowHeight(r, 24);
'

# Merge 1: A3:E3 — full-width banner spanning the vertical freeze split.
# Yellow fill + thick all-around border on the top-left cell. This is the
# regression case that motivated the fix.
hsx eval "$F" '
  const s = workbook.getSheet(0);
  s.addSpan(2, 0, 1, 5);
  s.setValue(2, 0, "FY24 banner — A3:E3 (crosses col freeze)");
  const cell = s.getCell(2, 0);
  cell.backColor("#FEF3C7").hAlign(1).fontStyle({bold:true});
  const thick = new GC.Spread.Sheets.LineBorder("#000000", GC.Spread.Sheets.LineStyle.thick);
  s.getRange(2, 0, 1, 5).setBorder(thick, {all:true});
'

# Merge 2: B5:E5 — also crosses the vertical split (B is pinned, C–E scroll).
hsx eval "$F" '
  const s = workbook.getSheet(0);
  s.addSpan(4, 1, 1, 4);
  s.setValue(4, 1, "subheader B5:E5");
  s.getCell(4, 1).backColor("#BFDBFE").hAlign(1);
  const med = new GC.Spread.Sheets.LineBorder("#1E3A8A", GC.Spread.Sheets.LineStyle.medium);
  s.getRange(4, 1, 1, 4).setBorder(med, {all:true});
'

# Merge 3: B6:D7 — 3x2 block, also crosses vertical split.
hsx eval "$F" '
  const s = workbook.getSheet(0);
  s.addSpan(5, 1, 2, 3);
  s.setValue(5, 1, "tile B6:D7");
  s.getCell(5, 1).backColor("#BBF7D0").hAlign(1).vAlign(1);
  const thin = new GC.Spread.Sheets.LineBorder("#065F46", GC.Spread.Sheets.LineStyle.thin);
  s.getRange(5, 1, 2, 3).setBorder(thin, {all:true});
'

# Freeze: 2 rows + 2 cols pinned. This puts:
#   - the vertical split between B (pinned) and C (scroll)
#   - the horizontal split between row 2 (pinned) and row 3 (scroll)
# Merge A3:E3 sits below the horizontal split → BL+BR panes.
# Merge B5:E5 sits below + crosses the vertical split → BL+BR panes.
hsx eval "$F" '
  const s = workbook.getSheet(0);
  s.frozenRowCount(2);
  s.frozenColumnCount(2);
'

echo "Built $F"
ls -la "$F"
