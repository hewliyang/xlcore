#!/usr/bin/env bash
# Fixture: `<a:blipFill>` inside `<xdr:sp>/<xdr:spPr>` (shape *fill*
# sourced from an embedded image — distinct from `<xdr:pic>`), plus
# `asvg:svgBlip` extension (modern Office's vector sidecar).
# Built via EPPlus because SpreadJS's public API can't author shape
# blip fills, and `asvg:svgBlip` is a post-save XML splice anyway.
#
# Today the renderer (`packages/xlsx-preview/src/shape.ts`) only
# handles `<a:noFill>` / `<a:solidFill>` / `<a:gradFill>` on shapes.
# `<a:blipFill>` on `<xdr:sp>` falls through to the accent1 paint
# (it only renders the embedded image when the shape is `<xdr:pic>`).
# This fixture lights up `docs/parity-shapes.md` P1 #10.
#
# Panels:
#   b1 — rect       PNG blip,    full stretch
#   b2 — ellipse    PNG blip,    image must be clipped to the silhouette
#   b3 — roundRect  PNG blip,    `<a:srcRect>` crops outer 25% on every side
#   b4 — chevron    PNG blip + 1.5pt black outline
#   b5 — rect       PNG raster + `asvg:svgBlip` sidecar → renderer must
#                   prefer the vector (crisper at scale).
#
# What to eyeball: `hsx.png` paints five images (b5 = SVG blue/yellow);
# `ours.png` (pre-fix) paints five flat-blue rectangles. Post-fix both
# converge except b5 where SpreadJS may drop the SVG sidecar and use the
# raster fallback — that's the same kind of intentional divergence as
# `outer-shadow.xlsx`.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
F="${1:-$HERE/blip-fills.xlsx}"
rm -f "$F"

# shellcheck source=./_dotnet-builder-guard.sh
source "$HERE/_dotnet-builder-guard.sh"
require_dotnet_builder "$HERE"

SAMPLE_IMG="$HERE/_blip-sample.png"
if [[ ! -f "$SAMPLE_IMG" ]]; then
    echo "missing sample image: $SAMPLE_IMG" >&2
    exit 2
fi

dotnet run --project "$HERE/dotnet-builder/FixtureBuilder" -- blip-fills "$F" "$SAMPLE_IMG" >/dev/null
python3 "$HERE/dotnet-builder/strip-boms.py" "$F"
echo "wrote $F"
