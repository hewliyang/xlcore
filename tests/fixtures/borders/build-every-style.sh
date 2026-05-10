#!/usr/bin/env bash
# Fixture: every OOXML border style on a single workbook.
#
# Covers all 14 ST_BorderStyle values (ECMA-376 §18.18.3) except
# "none": thin / medium / thick / dashed / dotted / double / hair /
# mediumDashed / dashDot / mediumDashDot / dashDotDot /
# mediumDashDotDot / slantDashDot. Each cell has the style applied
# on all four sides + the style name as inline-str text.
#
# Catches regressions in:
#   (a) extractor's `border_style_str` ordered-substring match
#       (slantDashDot is the easy footgun — its lowercase form
#       contains "dashdot", so a less careful order returns
#       "dashDot");
#   (b) renderer's `borderWidth` width table + `drawBorderLine`
#       dash patterns — there are real gaps today on
#       mediumDashDot / mediumDashDotDot / slantDashDot (no special
#       pattern set, painted as solid medium / solid 1px).
#
# We patch the OOXML directly via Python: hsx's public API doesn't
# expose `slantDashDot` cleanly, and we want byte-exact control of
# the styles.xml so JSON snapshots + pixel diffs are deterministic.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
F="${1:-$HERE/every-style.xlsx}"
rm -f "$F"
hsx create "$F" >/dev/null
python3 "$HERE/_patch_every_style.py" "$F"
echo "wrote $F"
