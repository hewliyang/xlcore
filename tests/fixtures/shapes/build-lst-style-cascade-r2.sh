#!/usr/bin/env bash
# Fixture: lstStyle round-2 features. Exercises the four DrawingML text
# capabilities promoted to first-class extract+paint in P1 #14:
#
#   1. Bullets — character bullets + auto-numbered bullets with `buFont`,
#      `buClr`, `buSzPct`.
#   2. Paragraph indents (`marL` / `indent`) and the hanging-indent
#      pattern (negative `indent`) used by every bullet list ever.
#   3. Line spacing (`<a:lnSpc>` spcPct + spcPts) and paragraph spacing
#      (`<a:spcBef>` + `<a:spcAft>` in pts).
#   4. Run kerning extras: superscript/subscript via `<a:rPr baseline=...>`
#      and the `u="none"` override that must defeat an inherited
#      underline from `lstStyle/defPPr/defRPr`.
#
# SpreadJS doesn't surface any of these from its public API, so the
# build path is the standard one for fixtures that need raw XML:
#
#   1. `hsx create` an empty workbook.
#   2. `hsx eval` adds 4 plain rectangles + caption cells so the panel
#      layout is self-describing.
#   3. `hsx daemon flush` / `stop` to release the in-memory copy.
#   4. Python zip-rewrite splices a hand-authored `<xdr:txBody>` over
#      each shape's existing one — same approach as
#      `build-list-style-inheritance.sh`.
#
# Panels (top → bottom):
#
#   s1 (char bullets) — `lstStyle/defPPr` declares `marL=342900`,
#     `indent=-342900` (hanging), and `<a:buChar char="•"/>` with
#     `<a:buClr><a:srgbClr val="C00000"/></a:buClr>` and
#     `<a:buSzPct val="80000"/>` (80%). Three paragraphs, each one run
#     long; bullets visible as red dots flush at the inset, text at
#     marL offset.
#
#   s2 (autoNum bullets) — `lstStyle/defPPr` declares the same indents,
#     plus `<a:buAutoNum type="arabicPeriod" startAt="3"/>` and
#     `<a:buFont typeface="Georgia"/>`. Three paragraphs → "3." "4."
#     "5." rendered in Georgia.
#
#   s3 (line / paragraph spacing) — two paragraphs separated by
#     `<a:spcBef val=600>` and `<a:spcAft val=300>` (6pt / 3pt) and a
#     `<a:lnSpc><a:spcPct val=180000/>` (180% line height). Visible
#     vertical breathing room vs the default-spaced baseline.
#
#   s4 (baseline + u="none") — one paragraph with mixed runs:
#     "E=mc" plain, "2" with `baseline="30000"` (30% superscript), then
#     " · H" plain, "2" with `baseline="-25000"` (subscript), then "O".
#     `defPPr/defRPr` sets `u="sng"` on the inherited cascade; the
#     last run explicitly sets `u="none"` which must override it
#     (otherwise the trailing "O" gets an unwanted underline).
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
F="${1:-$HERE/lst-style-cascade-r2.xlsx}"
rm -f "$F"
hsx create "$F" >/dev/null

hsx eval "$F" - <<'JS'
const sht = workbook.getSheet(0);
const T = GC.Spread.Sheets.Shapes.AutoShapeType;
for (let c = 0; c < 10; c++) sht.setColumnWidth(c, 90);
for (let r = 0; r < 20; r++) sht.setRowHeight(r, 22);

sht.setValue(0, 0,  "s1 char bullets (• red, 80%, hanging indent)");
sht.setValue(6, 0,  "s2 autoNum arabicPeriod, startAt=3, Georgia");
sht.setValue(13, 0, "s3 line spacing 180% + spcBef 6pt + spcAft 3pt");
sht.setValue(21, 0, "s4 baseline super/sub + u=\"none\" override");

const W = 360, H = 110;
sht.shapes.add("s1", T.rectangle, 40,  30, W, H).text("placeholder");
sht.shapes.add("s2", T.rectangle, 40, 165, W, H).text("placeholder");
sht.shapes.add("s3", T.rectangle, 40, 300, W, 140).text("placeholder");
sht.shapes.add("s4", T.rectangle, 40, 465, W, H).text("placeholder");
JS

hsx daemon flush >/dev/null 2>&1 || true
hsx daemon stop  >/dev/null 2>&1 || true

python3 - "$F" <<'PY'
import io, re, sys, zipfile

PATH = sys.argv[1]

def body(lst_style: str, paragraphs: str) -> str:
    return (
        '<xdr:txBody>'
        '<a:bodyPr vert="horz" wrap="square" anchor="t"/>'
        f'{lst_style}'
        f'{paragraphs}'
        '</xdr:txBody>'
    )

# Common: text 14pt, dark-blue.
RUN_DEFAULTS = (
    '<a:defRPr sz="1400">'
    '<a:solidFill><a:srgbClr val="1F3A5F"/></a:solidFill>'
    '<a:latin typeface="Calibri"/>'
    '</a:defRPr>'
)

# ---- s1: char bullets (red •, 80%, hanging indent) -------------------------
S1_LS = (
    '<a:lstStyle>'
    '<a:defPPr marL="342900" indent="-342900">'
    f'{RUN_DEFAULTS}'
    '<a:buClr><a:srgbClr val="C00000"/></a:buClr>'
    '<a:buSzPct val="80000"/>'
    '<a:buFont typeface="Arial"/>'
    '<a:buChar char="\u2022"/>'
    '</a:defPPr>'
    '</a:lstStyle>'
)
S1_PARAS = (
    '<a:p><a:r><a:t>Apples are red.</a:t></a:r></a:p>'
    '<a:p><a:r><a:t>Bananas are yellow.</a:t></a:r></a:p>'
    '<a:p><a:r><a:t>Cherries are darker red.</a:t></a:r></a:p>'
)
S1 = body(S1_LS, S1_PARAS)

# ---- s2: autoNum arabicPeriod startAt=3, Georgia ---------------------------
S2_LS = (
    '<a:lstStyle>'
    '<a:defPPr marL="342900" indent="-342900">'
    f'{RUN_DEFAULTS}'
    '<a:buClr><a:srgbClr val="2E7D32"/></a:buClr>'
    '<a:buFont typeface="Georgia"/>'
    '<a:buAutoNum type="arabicPeriod" startAt="3"/>'
    '</a:defPPr>'
    '</a:lstStyle>'
)
S2_PARAS = (
    '<a:p><a:r><a:t>First numbered item (starts at three).</a:t></a:r></a:p>'
    '<a:p><a:r><a:t>Second numbered item.</a:t></a:r></a:p>'
    '<a:p><a:r><a:t>Third numbered item.</a:t></a:r></a:p>'
)
S2 = body(S2_LS, S2_PARAS)

# ---- s3: line spacing 180% + spcBef 6pt + spcAft 3pt -----------------------
S3_LS = (
    '<a:lstStyle>'
    '<a:defPPr>'
    f'{RUN_DEFAULTS}'
    '</a:defPPr>'
    '</a:lstStyle>'
)
# spcPts uses 1/100 of a point: 600 = 6pt, 300 = 3pt.
S3_PP = (
    '<a:pPr>'
    '<a:lnSpc><a:spcPct val="180000"/></a:lnSpc>'
    '<a:spcBef><a:spcPts val="600"/></a:spcBef>'
    '<a:spcAft><a:spcPts val="300"/></a:spcAft>'
    '</a:pPr>'
)
S3_PARAS = (
    f'<a:p>{S3_PP}<a:r><a:t>Para 1: lnSpc 180% + 6pt before + 3pt after.</a:t></a:r></a:p>'
    f'<a:p>{S3_PP}<a:r><a:t>Para 2: same spacing rules apply.</a:t></a:r></a:p>'
)
S3 = body(S3_LS, S3_PARAS)

# ---- s4: superscript / subscript + u="none" override -----------------------
# defRPr ships with u="sng" so the cascade puts an underline on every run
# unless a run explicitly says u="none". The final " O" run does just that.
S4_LS = (
    '<a:lstStyle>'
    '<a:defPPr>'
    '<a:defRPr sz="1600" u="sng">'
    '<a:solidFill><a:srgbClr val="1F3A5F"/></a:solidFill>'
    '<a:latin typeface="Calibri"/>'
    '</a:defRPr>'
    '</a:defPPr>'
    '</a:lstStyle>'
)
S4_PARAS = (
    '<a:p>'
      '<a:r><a:rPr u="none"/><a:t>E=mc</a:t></a:r>'
      '<a:r><a:rPr baseline="30000" u="none"/><a:t>2</a:t></a:r>'
      '<a:r><a:rPr u="none"/><a:t>  \u00b7  H</a:t></a:r>'
      '<a:r><a:rPr baseline="-25000" u="none"/><a:t>2</a:t></a:r>'
      '<a:r><a:rPr u="none"/><a:t>O underlined: </a:t></a:r>'
      '<a:r><a:t>here</a:t></a:r>'  # inherits u="sng"
    '</a:p>'
)
S4 = body(S4_LS, S4_PARAS)

REPLACEMENTS = {"s1": S1, "s2": S2, "s3": S3, "s4": S4}

buf = io.BytesIO()
with zipfile.ZipFile(PATH, "r") as zin:
    names = zin.namelist()
    with zipfile.ZipFile(buf, "w", zipfile.ZIP_DEFLATED) as zout:
        for name in names:
            data = zin.read(name)
            if name == "xl/drawings/drawing1.xml":
                xml = data.decode("utf-8")
                for shape_name, new_body in REPLACEMENTS.items():
                    pat = re.compile(
                        r'(name="' + re.escape(shape_name) + r'"[\s\S]*?)'
                        r'<xdr:txBody>[\s\S]*?</xdr:txBody>'
                    )
                    new_xml, n = pat.subn(r'\1' + new_body, xml, count=1)
                    if n != 1:
                        raise SystemExit(f"failed to patch shape {shape_name!r}: matched {n} blocks")
                    xml = new_xml
                data = xml.encode("utf-8")
            zout.writestr(name, data)

with open(PATH, "wb") as fh:
    fh.write(buf.getvalue())

print(f"patched {PATH}")
PY

echo "wrote $F"
