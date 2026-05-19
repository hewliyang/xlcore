#!/usr/bin/env bash
# Fixture: DrawingML `<a:effectLst><a:outerShdw>` on themed buttons.
#
# Today the renderer (`packages/xlsx-preview/src/shape.ts`) reads no
# `<a:effectLst>` at all — every shape paints with zero shadow. Excel
# renders the shadow; this fixture is the most visible omission on
# themed cards/buttons (`docs/parity-shapes.md` P1 #2).
#
# **HSX caveat:** SpreadJS — our usual visual baseline — *also*
# silently drops `<a:outerShdw>`, so `.hsx.png` and `.ours.png` look
# nearly identical here. The fixture still has value: (a) it locks in
# `<a:effectLst><a:outerShdw>` extraction once we add it, (b) the
# OOXML is correct so a future Excel-screenshot baseline will diff
# cleanly. When wiring real shadow rendering, expect both pixmaps to
# *diverge from each other* (ours gains shadows, hsx stays flat)
# rather than converge — that's the spec-correct outcome.
#
# Authored via EPPlus because SpreadJS's public style() API doesn't
# expose shape effects.
#
# Panels (one shape each, row of four):
#   s1  roundRect + soft drop  (blur 5pt, dist 3pt, dir 45deg, black)
#   s2  ellipse  + medium       (blur 2.5pt, dist 2.5pt, dir 90deg, black)
#   s3  chevron  + long cast    (blur 8pt, dist 8pt, dir 135deg, charcoal)
#   s4  rect     + tinted       (blur 4pt, dist 2pt, dir 90deg, accent2 color)
#
# Author dependency: requires the gitignored EPPlus project at
# `tests/fixtures/shapes/dotnet-builder/FixtureBuilder/`. See
# `_dotnet-builder-guard.sh` for one-time setup instructions.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
F="${1:-$HERE/outer-shadow.xlsx}"
rm -f "$F"

# shellcheck source=./_dotnet-builder-guard.sh
source "$HERE/_dotnet-builder-guard.sh"
require_dotnet_builder "$HERE"

dotnet run --project "$HERE/dotnet-builder/FixtureBuilder" -- outer-shadow "$F" >/dev/null
python3 "$HERE/dotnet-builder/strip-boms.py" "$F"
echo "wrote $F"
