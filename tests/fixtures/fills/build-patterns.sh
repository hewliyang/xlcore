#!/usr/bin/env bash
# Fixture: every OOXML pattern fill type (gray125, gray0625, lightGray,
# mediumGray, darkGray, light/dark Horizontal/Vertical/Down/Up,
# light/dark Grid, light/dark Trellis, plus solid).
#
# SpreadJS doesn't expose hatch fills on its public style API (it
# only writes solid `patternType="solid"` from `backColor`), so we
# patch the OOXML directly the same way the diagonal-borders fixture
# does. See `_patch_patterns.py` for the layout + style table.
#
# Catches regressions in (a) extractor's `pattern_type_to_str` mapping
# the full `PatternValues` enum, (b) renderer's `paintFill` building
# the right 8x8 tile via `PATTERN_TILES_8X8`.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
F="${1:-$HERE/patterns.xlsx}"
rm -f "$F"
hsx create "$F" >/dev/null
python3 "$HERE/_patch_patterns.py" "$F"
echo "wrote $F"
