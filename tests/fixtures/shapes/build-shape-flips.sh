#!/usr/bin/env bash
# Fixture: non-connector `flipH` / `flipV` on the shape's `<a:xfrm>`.
#
# Today the renderer (`packages/xlsx-preview/src/shape.ts`) honors
# `flipH` / `flipV` for connectors only — non-connector shape flips
# are silently ignored. Excel + SpreadJS honor them everywhere → this
# fixture exercises the gap (`docs/parity-shapes.md` shortcut #7,
# P1 queue #4).
#
# Authored via EPPlus because SpreadJS's shape JS API doesn't expose
# `flipHorizontal` / `flipVertical` setters; `Shape.style()` won't
# emit them, and they round-trip as no-ops.
#
# Panels (row of five):
#   a1  rightArrow  baseline                  (points right)
#   a2  rightArrow  flipH                      (should point left)
#   a3  chevron     baseline                   (points right)
#   a4  chevron     flipH                      (should point left)
#   a5  pentagon    flipV                      (apex flipped to bottom)
#
# Eyeball: `hsx.png` shows the labels matching the visual direction
# of each shape; `ours.png` shows every shape in its un-flipped
# orientation.
#
# Author dependency: requires the gitignored EPPlus project at
# `tests/fixtures/shapes/dotnet-builder/FixtureBuilder/`. See
# `_dotnet-builder-guard.sh` for one-time setup instructions.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
F="${1:-$HERE/shape-flips.xlsx}"
rm -f "$F"

# shellcheck source=./_dotnet-builder-guard.sh
source "$HERE/_dotnet-builder-guard.sh"
require_dotnet_builder "$HERE"

dotnet run --project "$HERE/dotnet-builder/FixtureBuilder" -- shape-flips "$F" >/dev/null
python3 "$HERE/dotnet-builder/strip-boms.py" "$F"
echo "wrote $F"
