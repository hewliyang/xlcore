#!/usr/bin/env bash
# Fixture: x14 sparklines (line / column / win-loss + group-axis + extrema markers).
#
# We patch the OOXML directly because hsx's public JS API does not
# expose the x14 sparkline schema. Excel writes sparklines under
# `<extLst>/<ext uri="{05C60535-1F16-4fd2-B633-F4F36F0B64E0}">/<x14:sparklineGroups>`.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
F="${1:-$HERE/sparklines.xlsx}"
rm -f "$F"
hsx create "$F" >/dev/null
python3 "$HERE/_patch_sparklines.py" "$F"
echo "wrote $F"
