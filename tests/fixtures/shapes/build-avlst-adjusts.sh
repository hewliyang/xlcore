#!/usr/bin/env bash
# Fixture: `<a:avLst>` adjust values on `roundRect` / cardinal arrows /
# `leftRightArrow`.
#
# OOXML's `prstGeom` presets expose 1-2 "adjust" handles per shape that
# parameterise the path: `roundRect`'s corner radius, the cardinal
# arrows' tail thickness + head length, etc. The extractor already
# pulls `adj1` / `adj2` off `<a:gd>` (see `shapes.rs::preset_adj1` /
# `preset_adj2`), but until now the painter hardcoded defaults for
# every preset except the brace family. This is parity-shapes.md
# P1 #6 / shortcut #3.
#
# Panels (4 columns × 4 rows). Each row exercises one shape preset
# with three adjust extremes plus a baseline (no avLst, painter
# default kicks in):
#
#   Row 1 — roundRect          adj=default · adj=0 (square)   · adj=30000 · adj=50000 (pill)
#   Row 2 — rightArrow         default · adj1=20000 adj2=30000 (thin/short)
#                              · default-only · adj1=80000 adj2=80000 (fat/long)
#   Row 3 — upArrow            same sweep as rightArrow
#   Row 4 — leftRightArrow     same sweep
#
# Authored via `hsx eval` (SpreadJS doesn't expose adjust-handle
# setters on its shape API) + a Python zip-rewrite that injects an
# `<a:avLst>` into each shape's `<a:prstGeom>`. Same recipe as
# `build-list-style-inheritance.sh`.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
F="${1:-$HERE/avlst-adjusts.xlsx}"
rm -f "$F"
hsx create "$F" >/dev/null

hsx eval "$F" - <<'JS'
const sht = workbook.getSheet(0);
const T = GC.Spread.Sheets.Shapes.AutoShapeType;

for (let c = 0; c < 14; c++) sht.setColumnWidth(c, 90);
for (let r = 0; r < 24; r++) sht.setRowHeight(r, 22);

// Each row: 4 panels of one shape kind; columns are adjust sweeps.
// We label only by shape kind in column A and rely on the visual
// pattern across the row for the adjust sweep.
sht.setValue(0, 0,  "roundRect");
sht.setValue(6, 0,  "rightArrow");
sht.setValue(12, 0, "upArrow");
sht.setValue(18, 0, "leftRightArrow");

const W = 120, H = 80, GUT_X = 24;
const X0 = 40, COLS = 4;
const rowY = [30, 170, 310, 450];

function row(rowIdx, type, names) {
  const y = rowY[rowIdx];
  for (let i = 0; i < COLS; i++) {
    const x = X0 + i * (W + GUT_X);
    sht.shapes.add(names[i], type, x, y, W, H);
  }
}

row(0, T.roundedRectangle, ["rr0", "rr1", "rr2", "rr3"]);
row(1, T.rightArrow,       ["ra0", "ra1", "ra2", "ra3"]);
row(2, T.upArrow,          ["ua0", "ua1", "ua2", "ua3"]);
row(3, T.leftRightArrow,   ["lr0", "lr1", "lr2", "lr3"]);
JS

# Flush + stop the daemon before patching — same footgun as
# build-list-style-inheritance.sh.
hsx daemon flush >/dev/null 2>&1 || true
hsx daemon stop  >/dev/null 2>&1 || true

python3 - "$F" <<'PY'
"""Inject an <a:avLst> with explicit <a:gd> formulas into each shape's
<a:prstGeom> in xl/drawings/drawing1.xml. We target by cNvPr name and
rewrite the prstGeom element wholesale (SpreadJS may emit it either
self-closed `<a:prstGeom prst="..."/>` or with an empty
`<a:avLst/>` child; we normalise both).
"""
import re, sys, zipfile, io

PATH = sys.argv[1]

# (shape_name, preset, [(gd_name, val), ...])
# Column 0 = baseline (no avLst → painter defaults). Columns 1..3 = sweeps.
PATCH = {
    # roundRect: adj = corner radius * 100000 / min(w,h).
    "rr0": ("roundRect", []),                            # default (~16.667%)
    "rr1": ("roundRect", [("adj", 0)]),                  # 0 → sharp corners (== rect)
    "rr2": ("roundRect", [("adj", 30000)]),              # rounder
    "rr3": ("roundRect", [("adj", 50000)]),              # pill (radius = min/2)

    # rightArrow: adj1 = tail-height %, adj2 = head-length %.
    "ra0": ("rightArrow", []),                           # default 50/50
    "ra1": ("rightArrow", [("adj1", 20000), ("adj2", 30000)]),  # thin tail, short head
    "ra2": ("rightArrow", [("adj1", 50000), ("adj2", 70000)]),  # default tail, long head
    "ra3": ("rightArrow", [("adj1", 80000), ("adj2", 80000)]),  # fat tail, long head

    # upArrow: same semantics, axes swapped.
    "ua0": ("upArrow", []),
    "ua1": ("upArrow", [("adj1", 20000), ("adj2", 30000)]),
    "ua2": ("upArrow", [("adj1", 50000), ("adj2", 70000)]),
    "ua3": ("upArrow", [("adj1", 80000), ("adj2", 80000)]),

    # leftRightArrow: adj1 = tail height %, adj2 = per-side head length %.
    "lr0": ("leftRightArrow", []),
    "lr1": ("leftRightArrow", [("adj1", 20000), ("adj2", 30000)]),
    "lr2": ("leftRightArrow", [("adj1", 50000), ("adj2", 70000)]),
    "lr3": ("leftRightArrow", [("adj1", 80000), ("adj2", 80000)]),
}

def avlst_xml(gds):
    if not gds:
        return "<a:avLst/>"
    inner = "".join(
        f'<a:gd name="{name}" fmla="val {val}"/>' for (name, val) in gds
    )
    return f"<a:avLst>{inner}</a:avLst>"

def new_prstgeom(preset, gds):
    return f'<a:prstGeom prst="{preset}">{avlst_xml(gds)}</a:prstGeom>'

buf = io.BytesIO()
with zipfile.ZipFile(PATH, "r") as zin:
    with zipfile.ZipFile(buf, "w", zipfile.ZIP_DEFLATED) as zout:
        for name in zin.namelist():
            data = zin.read(name)
            if name == "xl/drawings/drawing1.xml":
                xml = data.decode("utf-8")
                for shape_name, (preset, gds) in PATCH.items():
                    # Match the <xdr:sp> whose cNvPr name="<shape_name>",
                    # then non-greedily its prstGeom (either self-closed
                    # or with children). Replace just the prstGeom.
                    pat = re.compile(
                        r'(name="' + re.escape(shape_name) + r'"[\s\S]*?)'
                        r'<a:prstGeom\s[^>]*?(?:/>|>[\s\S]*?</a:prstGeom>)'
                    )
                    repl = r'\1' + new_prstgeom(preset, gds)
                    xml2, n = pat.subn(repl, xml, count=1)
                    if n != 1:
                        raise SystemExit(
                            f"failed to patch {shape_name!r}: matched {n} blocks"
                        )
                    xml = xml2
                data = xml.encode("utf-8")
            zout.writestr(name, data)

with open(PATH, "wb") as fh:
    fh.write(buf.getvalue())

print(f"patched {PATH}")
PY

echo "wrote $F"
