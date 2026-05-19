#!/usr/bin/env bash
# Fixture: DrawingML `<a:lstStyle>` paragraph / run inheritance.
#
# OOXML cascades run + paragraph defaults through (lowest → highest):
#
#   1. `<a:lstStyle><a:defPPr>`                  — body-level defaults
#   2. `<a:lstStyle><a:lvlNpPr>` (matching `pPr@lvl`, default 0 → lvl1pPr)
#   3. paragraph's own `<a:pPr>` + `<a:pPr><a:defRPr>`
#   4. run's own `<a:rPr>`
#
# Without that cascade, a `<xdr:txBody>` whose runs carry only `<a:rPr/>`
# (or no rPr at all) reads as size=default, no color, no font — even
# when the workbook's template specified all three on the lstStyle.
# This is "a real fidelity gap on themed templates" per
# `docs/parity-shapes.md` P0 #5.
#
# The fixture cannot be authored straight through `hsx eval` because
# SpreadJS writes per-paragraph `<a:pPr><a:defRPr>` defaults and direct
# run `<a:rPr>` instead of a body-level `<a:lstStyle>`. So we:
#
#   1. Have hsx create four identical rectangles (s1..s4), each with the
#      label "Inherit me" so the rendered glyphs are visible.
#   2. Post-patch `xl/drawings/drawing1.xml` with a Python zip-rewrite
#      that, for each shape, strips the SpreadJS-emitted `<a:pPr>` +
#      run `<a:rPr>` and injects a chosen `<a:lstStyle>` block (plus,
#      for s4, also re-inserts a partial paragraph `<a:pPr><a:defRPr>`
#      and a partial run `<a:rPr>` so we can exercise precedence).
#
# Panels (left-to-right, top-to-bottom):
#
#   s1 (control)  — no lstStyle, no pPr, no rPr. Text should fall back
#                   to a basic system default (small black) so we can
#                   *see* that the other three panels behave differently.
#   s2 (defPPr)   — lstStyle/defPPr/defRPr: 24pt bold yellow Georgia,
#                   centered. With inheritance broken, s2 renders like s1.
#   s3 (lvl1pPr)  — lstStyle/lvl1pPr/defRPr: 20pt italic white Courier,
#                   right-aligned. Tests the lvl1pPr arm (which paragraph
#                   level=0 must resolve to, *not* defPPr).
#   s4 (precedence) — lstStyle/defPPr says 18pt magenta Verdana;
#                     pPr/defRPr overrides color → cyan;
#                     rPr overrides size → 28pt.
#                     Expected result: 28pt cyan Verdana bold (bold
#                     comes from defPPr/defRPr; only color was
#                     re-specified at the pPr level; only size at the
#                     rPr level).
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
F="${1:-$HERE/list-style-inheritance.xlsx}"
rm -f "$F"
hsx create "$F" >/dev/null

hsx eval "$F" - <<'JS'
const sht = workbook.getSheet(0);
const T = GC.Spread.Sheets.Shapes.AutoShapeType;
for (let c = 0; c < 10; c++) sht.setColumnWidth(c, 90);
for (let r = 0; r < 16; r++) sht.setRowHeight(r, 22);

// Labels in column A so the panel layout is self-describing.
sht.setValue(0, 0, "s1 control (no lstStyle / pPr / rPr)");
sht.setValue(5, 0, "s2 lstStyle/defPPr → 24pt bold yellow Georgia ctr");
sht.setValue(10, 0, "s3 lstStyle/lvl1pPr → 20pt italic white Courier r");
sht.setValue(15, 0, "s4 precedence: rPr(sz28) > pPr/defRPr(cyan) > defPPr(Verdana,b,magenta,18)");

const W = 280, H = 80;
sht.shapes.add("s1", T.rectangle,  40,  30, W, H).text("Inherit me");
sht.shapes.add("s2", T.rectangle,  40, 140, W, H).text("Inherit me");
sht.shapes.add("s3", T.rectangle,  40, 250, W, H).text("Inherit me");
sht.shapes.add("s4", T.rectangle,  40, 360, W, H).text("Inherit me");
JS

# Flush the daemon's in-memory copy to disk and evict the file BEFORE
# patching — otherwise the daemon will happily clobber our XML rewrite
# on its next idle flush. (`hsx daemon stop` is the cheapest way to
# guarantee a clean slate; downstream `hsx screenshot` will spin the
# daemon back up on demand.)
hsx daemon flush >/dev/null 2>&1 || true
hsx daemon stop  >/dev/null 2>&1 || true

python3 - "$F" <<'PY'
"""Rewrite drawing1.xml so each shape's txBody exercises a different
arm of the lstStyle cascade. We use stdlib only — `lxml` keeps the
xmlns:a / xmlns:xdr prefixes that the extractor expects, and the
SpreadJS-emitted file is small enough that a single regex pass per
shape is the right cost.
"""
import re, sys, zipfile, io, shutil

PATH = sys.argv[1]

# Each entry: (shape_name, replacement <xdr:txBody>...</xdr:txBody>).
# We rebuild the entire txBody so we don't have to fight SpreadJS's
# choice of attributes / whitespace.
def body(lst_style: str, paragraph: str) -> str:
    return (
        '<xdr:txBody>'
        '<a:bodyPr vert="horz" wrap="square" anchor="t"/>'
        f'{lst_style}'
        f'{paragraph}'
        '</xdr:txBody>'
    )

# s1: no lstStyle, paragraph has no pPr, run has no rPr → renderer
# should fall back to system default (small black). Visible "Inherit me"
# in a tiny default font is the *expected* baseline.
S1 = body("", '<a:p><a:r><a:t>Inherit me</a:t></a:r></a:p>')

# s2: lstStyle/defPPr → 24pt bold yellow Georgia center.
S2 = body(
    '<a:lstStyle><a:defPPr algn="ctr">'
    '<a:defRPr sz="2400" b="1">'
    '<a:solidFill><a:srgbClr val="FFD700"/></a:solidFill>'
    '<a:latin typeface="Georgia"/>'
    '</a:defRPr></a:defPPr></a:lstStyle>',
    '<a:p><a:r><a:t>Inherit me</a:t></a:r></a:p>',
)

# s3: lstStyle/lvl1pPr → 20pt italic white Courier right.
# Paragraph level defaults to 0, which by spec maps to lvl1pPr.
S3 = body(
    '<a:lstStyle><a:lvl1pPr algn="r">'
    '<a:defRPr sz="2000" i="1">'
    '<a:solidFill><a:srgbClr val="FFFFFF"/></a:solidFill>'
    '<a:latin typeface="Courier New"/>'
    '</a:defRPr></a:lvl1pPr></a:lstStyle>',
    '<a:p><a:r><a:t>Inherit me</a:t></a:r></a:p>',
)

# s4: precedence. defPPr is the lowest; pPr/defRPr overrides color only;
# rPr overrides size only. Bold + font should survive from defPPr.
S4 = body(
    '<a:lstStyle><a:defPPr>'
    '<a:defRPr sz="1800" b="1">'
    '<a:solidFill><a:srgbClr val="FF00FF"/></a:solidFill>'  # magenta
    '<a:latin typeface="Verdana"/>'
    '</a:defRPr></a:defPPr></a:lstStyle>',
    '<a:p>'
      '<a:pPr>'
        '<a:defRPr>'
          '<a:solidFill><a:srgbClr val="00FFFF"/></a:solidFill>'  # cyan
        '</a:defRPr>'
      '</a:pPr>'
      '<a:r><a:rPr sz="2800"/><a:t>Inherit me</a:t></a:r>'
    '</a:p>',
)

REPLACEMENTS = {"s1": S1, "s2": S2, "s3": S3, "s4": S4}

# Read drawing1.xml from the zip.
buf = io.BytesIO()
with zipfile.ZipFile(PATH, "r") as zin:
    names = zin.namelist()
    with zipfile.ZipFile(buf, "w", zipfile.ZIP_DEFLATED) as zout:
        for name in names:
            data = zin.read(name)
            if name == "xl/drawings/drawing1.xml":
                xml = data.decode("utf-8")
                for shape_name, new_body in REPLACEMENTS.items():
                    # Find the <xdr:sp> whose cNvPr name="<shape_name>"
                    # and replace its existing <xdr:txBody>…</xdr:txBody>
                    # with the new one. The cNvPr → txBody distance is
                    # well under the worksheet drawing size, so a
                    # bounded non-greedy match is safe.
                    pat = re.compile(
                        r'(name="' + re.escape(shape_name) + r'"[\s\S]*?)'
                        r'<xdr:txBody>[\s\S]*?</xdr:txBody>'
                    )
                    new_xml, n = pat.subn(r'\1' + new_body, xml, count=1)
                    if n != 1:
                        raise SystemExit(
                            f"failed to patch shape {shape_name!r}: matched {n} blocks"
                        )
                    xml = new_xml
                data = xml.encode("utf-8")
            zout.writestr(name, data)

with open(PATH, "wb") as fh:
    fh.write(buf.getvalue())

print(f"patched {PATH}")
PY

echo "wrote $F"
