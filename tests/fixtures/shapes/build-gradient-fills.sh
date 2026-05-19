#!/usr/bin/env bash
# Fixture: DrawingML `<a:gradFill>` on shapes — built via EPPlus
# because SpreadJS's public style() API silently drops gradient fills
# to solidFill on save.
#
# Today the renderer (`packages/xlsx-preview/src/shape.ts`) only
# handles `<a:noFill>` and `<a:solidFill>`. `<a:gradFill>` (linear /
# radial / path) falls through to the accent1 paint. SpreadJS + Excel
# render gradients faithfully → this fixture lights up the single
# biggest visible gap on themed shapes (`docs/parity-shapes.md` P1 #1).
#
# Panels (one rect each):
#   g1  linear horizontal      lin ang=0      accent1-ish → accent2-ish
#   g2  linear 45deg           lin ang=45     accent1-ish → accent2-ish
#   g3  linear vertical        lin ang=90     accent1-ish → accent2-ish
#   g4  radial                 path=circle    accent1-ish → accent2-ish
#   g5  3-stop linear          lin ang=0      accent1 → accent3 → accent2
#
# What to eyeball: `hsx.png` paints five smooth gradients;
# `ours.png` paints five flat blue rectangles. Once the renderer
# learns `gradFill`, both screenshots converge.
#
# Author dependency: requires the gitignored EPPlus project at
# `tests/fixtures/shapes/dotnet-builder/FixtureBuilder/`. See
# `_dotnet-builder-guard.sh` for one-time setup instructions. The
# committed .xlsx + .hsx.png + .ours.png are the source of truth for
# CI; this script is provenance / re-author tooling only.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
F="${1:-$HERE/gradient-fills.xlsx}"
rm -f "$F"

# shellcheck source=./_dotnet-builder-guard.sh
source "$HERE/_dotnet-builder-guard.sh"
require_dotnet_builder "$HERE"

# EPPlus authors the gradient XML directly; no XML patching required.
# Strip the UTF-8 BOM EPPlus prepends to every part — our DrawingML
# parser rejects it on `xl/drawings/drawing*.xml`.
dotnet run --project "$HERE/dotnet-builder/FixtureBuilder" -- gradient-fills "$F" >/dev/null
python3 "$HERE/dotnet-builder/strip-boms.py" "$F"
echo "wrote $F"
