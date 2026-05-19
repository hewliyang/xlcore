#!/usr/bin/env bash
# Fixture: `<a:xfrm rot="..."/>` on `<xdr:grpSpPr>` — group rotation as
# a rigid body. Exercises `docs/parity-shapes.md` P1 #4 (group rotation
# half). Authored via EPPlus because SpreadJS's shape JS API doesn't
# expose group-rotation setters and `ExcelGroupShape.Rotation` isn't
# exposed by EPPlus either, so the builder edits the group's `<a:xfrm>`
# XML directly to add `rot` (plus the off/ext/chOff/chExt EPPlus emits
# as zero by default).
#
# Panels (column of three groups, each containing rect + rightArrow +
# ellipse stacked horizontally):
#   g1 — baseline (rot=0)
#   g2 — rot 30°
#   g3 — rot 90°
#
# Eyeball: `hsx.png` shows each group's three children rotated as a
# rigid body around the group's bbox center. `ours.png` baseline (pre-
# fix) shows children un-rotated. Post-fix `ours.png` matches `hsx.png`.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
F="${1:-$HERE/groups-rotated.xlsx}"
rm -f "$F"

# shellcheck source=./_dotnet-builder-guard.sh
source "$HERE/_dotnet-builder-guard.sh"
require_dotnet_builder "$HERE"

dotnet run --project "$HERE/dotnet-builder/FixtureBuilder" -- groups-rotated "$F" >/dev/null
python3 "$HERE/dotnet-builder/strip-boms.py" "$F"
echo "wrote $F"
