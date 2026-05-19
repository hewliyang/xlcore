#!/usr/bin/env bash
# Fixture: themed shapes whose **paint comes only from `<xdr:style>`**
# (no direct `<a:solidFill>` / `<a:ln>` inside `<xdr:spPr>`).
#
# Office writes shape XML this way fairly often — when the user picks
# a quick-style preset without overriding the fill/line directly, the
# `<xdr:spPr>` ends up with just `<a:xfrm>` + `<a:prstGeom>` and all
# the color comes from `<a:fillRef>` / `<a:lnRef>` against the
# theme's `<a:fmtScheme>`. Renderers that only look at direct paint
# show ghost shapes (no fill, no outline). Locks in the style-ref
# resolver in `crates/xlcore-export/src/shapes.rs::resolve_style_refs`.
#
# Construction strategy: build the same geometry as `basic-autoshapes`
# (so we still exercise preset dispatch), then post-process the
# `xl/drawings/drawing*.xml` XML to:
#   1. strip every `<a:solidFill>...</a:solidFill>` and `<a:ln ...>...</a:ln>`
#      directly under `<xdr:spPr>`,
#   2. rewrite each shape's `<a:fillRef idx="1">` to cycle through accent1..6
#      so we visibly differentiate the shapes (otherwise every shape would
#      paint accent1 and we'd have no way to tell the resolver is honoring
#      the schemeClr override on the ref element itself),
#   3. cycle `<a:lnRef idx="1|2|3">` so the three default theme line widths
#      (subtle/moderate/intense → 6350/12700/19050 EMU) are all exercised.
#
# What to eyeball: `ours.png` should show the same shapes as
# `basic-autoshapes.ours.png` but with rainbow theming and three
# visibly distinct outline weights. Ghosting any single shape means
# the resolver isn't kicking in for that path.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
F="${1:-$HERE/style-refs-themed.xlsx}"
SRC="$HERE/basic-autoshapes.xlsx"
if [ ! -f "$SRC" ]; then
  echo "missing source fixture $SRC — run build-basic-autoshapes.sh first" >&2
  exit 1
fi
rm -f "$F"
# We intentionally do NOT rebuild basic-autoshapes here — currently
# `hsx eval` doesn't persist shape mutations to disk (regression in
# the hsx tool), so this fixture is derived from the committed
# basic-autoshapes.xlsx via pure XML rewrite.
cp "$SRC" "$F"

python3 - "$F" <<'PY'
import re, sys, shutil, zipfile, tempfile, os

src = sys.argv[1]
tmp_dir = tempfile.mkdtemp()
out = os.path.join(tmp_dir, "patched.xlsx")

with zipfile.ZipFile(src, "r") as zin, zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as zout:
    for item in zin.infolist():
        data = zin.read(item.filename)
        if item.filename.startswith("xl/drawings/drawing") and item.filename.endswith(".xml"):
            xml = data.decode("utf-8")

            # 1. Strip direct <a:solidFill>...</a:solidFill> blocks (anywhere
            #    in the drawing — they only ever live under xdr:spPr or
            #    inside <a:ln>, both of which we want gone).
            xml = re.sub(r"<a:solidFill>.*?</a:solidFill>", "", xml, flags=re.S)
            # 2. Strip direct <a:ln ...>...</a:ln> blocks (the outline under spPr).
            # IMPORTANT: anchor with `(\s|>)` after `ln` so this does NOT
            # eat `<a:lnRef ...>...</a:lnRef>` blocks inside `<xdr:style>`.
            xml = re.sub(r"<a:ln(\s[^>]*)?>.*?</a:ln>", "", xml, flags=re.S)
            # ...and self-closing <a:ln .../> just in case.
            xml = re.sub(r"<a:ln(\s[^>]*)?/>", "", xml)

            # 3. Rewrite each shape's lnRef/fillRef to cycle through theme
            #    accents + line widths. We walk shape-by-shape because
            #    re.sub-with-counter is awkward when the matches share state.
            accents = ["accent1", "accent2", "accent3", "accent4", "accent5", "accent6"]
            ln_widths = ["1", "2", "3"]
            counter = [0]
            def rewrite(m):
                i = counter[0]
                acc = accents[i % len(accents)]
                lw = ln_widths[i % len(ln_widths)]
                counter[0] = i + 1
                return (
                    f'<xdr:style>'
                    f'<a:lnRef idx="{lw}"><a:schemeClr val="{acc}"/></a:lnRef>'
                    f'<a:fillRef idx="1"><a:schemeClr val="{acc}"/></a:fillRef>'
                    f'<a:effectRef idx="0"><a:schemeClr val="{acc}"/></a:effectRef>'
                    f'<a:fontRef idx="minor"><a:schemeClr val="dk1"/></a:fontRef>'
                    f'</xdr:style>'
                )
            xml = re.sub(r"<xdr:style>.*?</xdr:style>", rewrite, xml, flags=re.S)

            data = xml.encode("utf-8")
        zout.writestr(item, data)

shutil.move(out, src)
shutil.rmtree(tmp_dir)
print(f"patched {src}")
PY

echo "wrote $F"
