#!/usr/bin/env bash
# Builds a workbook whose styles reference theme color slots (`theme="N"`)
# instead of hardcoded RGB. Used to verify that xlcore parses
# `xl/theme/theme1.xml` and that the renderer resolves theme refs against
# the parsed palette rather than the hardcoded Office defaults.
#
# Spreadsheet `theme="N"` mapping (note the lt/dk pair swap vs XML order):
#   0:lt1 (white)   1:dk1 (black)   2:lt2  3:dk2
#   4..9: accent1..accent6           10:hlink   11:folHlink
#
# Strategy: hsx creates an empty workbook (gives us a valid xlsx skeleton
# including theme1.xml + the relationships). Then `_patch_theme.py`
# rewrites theme1.xml with a Cyber palette, and rewrites styles.xml to
# add 12 fills referencing `theme="0"` through `theme="11"`, plus 12
# cells in row 1 each tagged with one of those fills. We hand-write the
# styles instead of going through hsx because hsx's JSON-then-flush
# write model races with subsequent reads.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
F="${1:-$HERE/custom-theme-accent.xlsx}"
rm -f "$F"
hsx create "$F" >/dev/null
python3 "$HERE/_patch_theme.py" "$F"
echo "wrote $F"
