#!/usr/bin/env bash
# tests/fixtures/charts/build-data-labels-per-point.sh
#
# A single-sheet workbook exercising per-data-point <c:dLbl> overrides
# inside <c:dLbls>. ECMA-376 §21.2.2.49 lets each <c:dLbl> override
# delete/text/position/numFmt/show* on a single data point — common
# in pies that label only the largest slice, or column charts that
# highlight an outlier.
#
# Two charts side-by-side:
#   - Pie with showCategory at the chart level; per-slice overrides:
#     idx=0 literal text "★ TOP", idx=1 deleted, idx=3 numFmt override.
#   - Column with chart-level showValue+outEnd; per-bar overrides:
#     idx=2 inEnd+inverted text style, idx=4 deleted, idx=0 literal "Q1!".
#
# hsx (SpreadJS) doesn't expose per-point dLbl on its API, so we let
# hsx lay down vanilla labels and post-patch the chart XML with Python.
# Targets the new schema field `DataLabels.pointOverrides`.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
F="${1:-$HERE/data-labels-per-point.xlsx}"
rm -f "$F"
hsx create "$F" >/dev/null

# Source data: 5 quarters across one series.
hsx set "$F" "Sheet1!A1:F1" '[
  [{"value":"","style":{"fontStyle":{"bold":true}}},
   {"value":"Q1","style":{"fontStyle":{"bold":true}}},
   {"value":"Q2","style":{"fontStyle":{"bold":true}}},
   {"value":"Q3","style":{"fontStyle":{"bold":true}}},
   {"value":"Q4","style":{"fontStyle":{"bold":true}}},
   {"value":"Q5","style":{"fontStyle":{"bold":true}}}]
]'
hsx set "$F" "Sheet1!A2:F2" '[
  [{"value":"Revenue"},{"value":151},{"value":121},{"value":92},{"value":102},{"value":88}]
]'

hsx eval "$F" '
  const sht = workbook.getSheet(0);
  const T = GC.Spread.Sheets.Charts.ChartType;
  // Pie on the left with chart-level showCategory (default text=cat name).
  const pie = sht.charts.add("pie", T.pie, 0, 80, 360, 320, "A1:F2");
  pie.title({text:"Pie — per-slice <c:dLbl>"});
  pie.dataLabels({showCategoryName: true});
  // Column on the right with chart-level showValue.
  const col = sht.charts.add("col", T.columnClustered, 380, 80, 380, 320, "A1:F2");
  col.title({text:"Column — per-bar <c:dLbl>"});
  col.dataLabels({showValue: true, position: GC.Spread.Sheets.Charts.DataLabelPosition.outEnd});
'

hsx daemon flush >/dev/null 2>&1 || true

# Splice <c:dLbl> blocks into both charts.
python3 - "$F" <<'PY'
import sys, zipfile, shutil, re
path = sys.argv[1]

def make_dlbl(idx, **kw):
    """Build a <c:dLbl idx=N>...</c:dLbl> with the requested overrides."""
    parts = [f'<c:idx val="{idx}"/>']
    if kw.get("delete"):
        parts.append('<c:delete val="1"/>')
        return f'<c:dLbl>{"".join(parts)}</c:dLbl>'
    if "text" in kw:
        # <c:tx><c:rich>...<a:p><a:r><a:t>literal</a:t></a:r></a:p>...</c:rich></c:tx>
        t = kw["text"]
        parts.append(
            '<c:tx><c:rich>'
            '<a:bodyPr/><a:lstStyle/>'
            f'<a:p><a:r><a:rPr lang="en-US"/><a:t>{t}</a:t></a:r></a:p>'
            '</c:rich></c:tx>'
        )
    if "numFmt" in kw:
        parts.append(f'<c:numFmt formatCode="{kw["numFmt"]}" sourceLinked="0"/>')
    if "position" in kw:
        parts.append(f'<c:dLblPos val="{kw["position"]}"/>')
    for fname, attr in [
        ("showLegendKey", "showLegendKey"),
        ("showVal", "showVal"),
        ("showCatName", "showCatName"),
        ("showSerName", "showSerName"),
        ("showPercent", "showPercent"),
        ("showBubbleSize", "showBubbleSize"),
    ]:
        if attr in kw:
            parts.append(f'<c:{fname} val="{1 if kw[attr] else 0}"/>')
    return f'<c:dLbl>{"".join(parts)}</c:dLbl>'

def patch_pie(xml):
    # The pie's <c:dLbls> sits inside <c:pieChart><c:ser>. We inject
    # the <c:dLbl> children at the FRONT of <c:dLbls>, before the
    # parent show* sequence (per CT_DLbls schema order).
    pie_m = re.search(r"<c:pieChart\b[^>]*>.*?</c:pieChart>", xml, flags=re.S)
    if not pie_m:
        return xml, 0
    pie_block = pie_m.group(0)
    # Pick the *outer* <c:dLbls> on the series (inside <c:ser>), not the
    # chart-group one. Match by location: it's the second occurrence
    # because hsx writes both. Actually hsx may only write one; just
    # match the first <c:dLbls>...</c:dLbls> in the pie block.
    dlbls_m = re.search(r"<c:dLbls\b[^>]*>", pie_block)
    if not dlbls_m:
        return xml, 0
    overrides = "".join([
        make_dlbl(0, text="★ TOP", showCatName=True),
        make_dlbl(1, delete=True),
        make_dlbl(3, showVal=True, numFmt="$#,##0"),
    ])
    patched_pie = pie_block.replace(
        dlbls_m.group(0),
        dlbls_m.group(0) + overrides,
        1,
    )
    return xml.replace(pie_block, patched_pie, 1), 1

def patch_col(xml):
    bar_m = re.search(r"<c:barChart\b[^>]*>.*?</c:barChart>", xml, flags=re.S)
    if not bar_m:
        return xml, 0
    bar_block = bar_m.group(0)
    dlbls_m = re.search(r"<c:dLbls\b[^>]*>", bar_block)
    if not dlbls_m:
        return xml, 0
    overrides = "".join([
        make_dlbl(0, text="Q1!", showVal=False),
        make_dlbl(2, position="inEnd"),
        make_dlbl(4, delete=True),
    ])
    patched_bar = bar_block.replace(
        dlbls_m.group(0),
        dlbls_m.group(0) + overrides,
        1,
    )
    return xml.replace(bar_block, patched_bar, 1), 1

with zipfile.ZipFile(path, 'r') as z:
    chart_names = sorted(n for n in z.namelist() if re.match(r"xl/charts/chart\d+\.xml$", n))
    contents = {n: z.read(n).decode() for n in chart_names}

# Per ECMA-376 §21.2.2.49 the children of <c:dLbls> after the <c:dLbl>
# array must appear in this order (matches DataLabelsChoiceSequence in
# ooxmlsdk's chart schema). hsx emits them in an arbitrary order and
# ooxmlsdk's parser drops every show* sibling that appears in the
# wrong slot — same root issue as the leaderLines-after-extLst quirk
# called out in data-labels.xlsx's build script.
DLBLS_TAIL_ORDER = [
    "numFmt",
    "spPr",
    "txPr",
    "dLblPos",
    "showLegendKey",
    "showVal",
    "showCatName",
    "showSerName",
    "showPercent",
    "showBubbleSize",
    "separator",
    "showLeaderLines",
    "leaderLines",
]

def fix_dlbls_order(xml):
    def fix_one(m):
        block = m.group(0)
        # Capture the <c:dLbl>... per-point overrides exactly as written
        # (they sit at the front of <c:dLbls> per the schema).
        head_dlbls = re.findall(r"<c:dLbl\b(?:[^>/]*/>|.*?</c:dLbl>)", block, flags=re.S)
        # Body = block stripped of <c:dLbls> wrapper + <c:dLbl> heads.
        body = re.sub(r"</?c:dLbls\b[^>]*>", "", block, flags=re.S)
        for d in head_dlbls:
            body = body.replace(d, "", 1)
        # Carve out <c:extLst> separately — it sits AFTER the choice
        # sequence per CT_DLbls.
        ext_lst = ""
        ext_m = re.search(r"<c:extLst\b.*?</c:extLst>", body, flags=re.S)
        if ext_m:
            ext_lst = ext_m.group(0)
            body = body.replace(ext_lst, "", 1)
        # Bucket the remaining siblings by qname-local-part.
        buckets = {k: [] for k in DLBLS_TAIL_ORDER}
        # Match either self-closing <c:Foo .../> or paired <c:Foo>...</c:Foo>.
        # Self-closing must come first so the paired regex's `</c:Foo>`
        # isn't required to be present.
        for tag in DLBLS_TAIL_ORDER:
            pattern = (
                rf"<c:{tag}\b[^/>]*/>|<c:{tag}\b[^>]*>.*?</c:{tag}>"
            )
            for em in re.finditer(pattern, body, flags=re.S):
                buckets[tag].append(em.group(0))
            body = re.sub(pattern, "", body, flags=re.S)
        # Anything left in `body` (e.g. ChartShapeProperties extras
        # already lumped into spPr above) gets prepended so we don't
        # lose it. Strip whitespace to keep the XML tidy.
        leftover = body.strip()
        ordered = "".join("".join(buckets[t]) for t in DLBLS_TAIL_ORDER)
        return (
            "<c:dLbls>"
            + "".join(head_dlbls)
            + leftover
            + ordered
            + ext_lst
            + "</c:dLbls>"
        )
    return re.sub(r"<c:dLbls\b[^>]*>.*?</c:dLbls>", fix_one, xml, flags=re.S)

patched = {}
for n, xml in contents.items():
    new_xml, np = patch_pie(xml)
    if np == 0:
        new_xml, nc = patch_col(xml)
        if nc == 0:
            patched[n] = xml
        else:
            patched[n] = new_xml
    else:
        patched[n] = new_xml
    # Fix hsx's invalid <c:showLeaderLines> position so ooxmlsdk's
    # strict-sequence parser can read the parent show* run.
    patched[n] = fix_dlbls_order(patched[n])

# Both patches should have applied across the chart parts combined.
total = sum(1 for n, xml in patched.items() if xml != contents[n])
assert total >= 2, f"only patched {total} charts"

tmp = path + ".new"
with zipfile.ZipFile(path, 'r') as zin, zipfile.ZipFile(tmp, 'w', zipfile.ZIP_DEFLATED) as zout:
    for it in zin.namelist():
        data = patched[it].encode() if it in patched else zin.read(it)
        zout.writestr(it, data)
shutil.move(tmp, path)
PY

echo "Built $F"
ls -la "$F"
