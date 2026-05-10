#!/usr/bin/env python3
"""Build a fixture that exercises `cellStyleXf` inheritance via
`apply*="0"` flags.

Why hand-patch? Because `hsx` (and most Excel writers) flatten the
inheritance at write-time: when you set a "Title" cell style they
copy fontId/fillId/etc. into the cell xf and emit `applyFont="1"`.
That means the renderer can't tell whether inheritance is being
honored or skipped \u2014 both paths produce the same pixels. To prove
the new code path actually fires, we have to write the *unflattened*
form: cell xfs with `fontId="0"` and `applyFont="0"`, depending on
the parent `cellStyleXfs[xfId]` to supply the real font.

Layout (cells in column A, rows 2..5):

    A2  "Title"        xfId=1  applyFont=0      \u2192 should pick up font 2 (Calibri 18, bold)
    A3  "Heading 1"    xfId=2  applyFont=0,
                                applyBorder=0   \u2192 font 3 (Calibri 14 italic),
                                                  bottom border thick
    A4  "Highlighted"  xfId=3  applyFill=0      \u2192 yellow solid fill
    A5  "Centered"     xfId=4  applyAlignment=0 \u2192 horizontal=center, vertical=center

Cell xfs all carry fontId=0 / fillId=0 / borderId=0 / no alignment so
that without the fix every cell would render as plain Calibri 11 left-
aligned with no fill or border. With the fix they inherit from the
named-style parents.
"""
import sys, zipfile, os, tempfile

PATH = sys.argv[1]

STYLES_XML = """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <fonts count="4">
    <font><sz val="11"/><color rgb="FF000000"/><name val="Calibri"/></font>
    <font><sz val="11"/><color rgb="FF000000"/><name val="Calibri"/></font>
    <font><sz val="18"/><b/><color rgb="FF1F4E79"/><name val="Calibri"/></font>
    <font><sz val="14"/><i/><color rgb="FF2E75B6"/><name val="Calibri"/></font>
  </fonts>
  <fills count="3">
    <fill><patternFill patternType="none"/></fill>
    <fill><patternFill patternType="gray125"/></fill>
    <fill><patternFill patternType="solid"><fgColor rgb="FFFFE699"/><bgColor indexed="64"/></patternFill></fill>
  </fills>
  <borders count="2">
    <border><left/><right/><top/><bottom/><diagonal/></border>
    <border>
      <left/><right/><top/>
      <bottom style="thick"><color rgb="FF1F4E79"/></bottom>
      <diagonal/>
    </border>
  </borders>
  <cellStyleXfs count="5">
    <!-- 0: Normal -->
    <xf numFmtId="0" fontId="0" fillId="0" borderId="0"/>
    <!-- 1: Title \u2192 font 2 -->
    <xf numFmtId="0" fontId="2" fillId="0" borderId="0" applyFont="1"/>
    <!-- 2: Heading 1 \u2192 font 3 + bottom border -->
    <xf numFmtId="0" fontId="3" fillId="0" borderId="1" applyFont="1" applyBorder="1"/>
    <!-- 3: Highlighted \u2192 yellow fill -->
    <xf numFmtId="0" fontId="0" fillId="2" borderId="0" applyFill="1"/>
    <!-- 4: Centered \u2192 alignment center/center -->
    <xf numFmtId="0" fontId="0" fillId="0" borderId="0" applyAlignment="1">
      <alignment horizontal="center" vertical="center"/>
    </xf>
  </cellStyleXfs>
  <cellXfs count="5">
    <!-- 0: default Normal -->
    <xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/>
    <!-- 1: Title cell \u2014 inherits font from xfId=1 -->
    <xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="1" applyFont="0"/>
    <!-- 2: Heading 1 cell \u2014 inherits font + border -->
    <xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="2" applyFont="0" applyBorder="0"/>
    <!-- 3: Highlighted \u2014 inherits fill -->
    <xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="3" applyFill="0"/>
    <!-- 4: Centered \u2014 inherits alignment -->
    <xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="4" applyAlignment="0"/>
  </cellXfs>
  <cellStyles count="5">
    <cellStyle name="Normal" xfId="0" builtinId="0"/>
    <cellStyle name="Title" xfId="1" builtinId="15"/>
    <cellStyle name="Heading 1" xfId="2" builtinId="16"/>
    <cellStyle name="Highlighted" xfId="3"/>
    <cellStyle name="Centered" xfId="4"/>
  </cellStyles>
</styleSheet>"""

CELLS = [
    ("A2", 1, "Title"),
    ("A3", 2, "Heading 1"),
    ("A4", 3, "Highlighted"),
    ("A5", 4, "Centered"),
]


def make_sheet() -> str:
    rows = []
    for ref, xfi, label in CELLS:
        rownum = ref[1:]
        rows.append(
            f'<row r="{rownum}" ht="30" customHeight="1">'
            f'<c r="{ref}" s="{xfi}" t="inlineStr"><is><t>{label}</t></is></c>'
            f'</row>'
        )
    return f"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
           xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <dimension ref="A2:A5"/>
  <sheetViews><sheetView workbookViewId="0"/></sheetViews>
  <sheetFormatPr defaultRowHeight="15"/>
  <cols><col min="1" max="1" width="22" customWidth="1"/></cols>
  <sheetData>
    {''.join(rows)}
  </sheetData>
</worksheet>"""


def rewrite(path: str) -> None:
    with zipfile.ZipFile(path, "r") as zin:
        names = zin.namelist()
        blobs = {n: zin.read(n) for n in names}

    blobs["xl/styles.xml"] = STYLES_XML.encode("utf-8")
    sheet_path = next(
        (n for n in names if n.startswith("xl/worksheets/sheet") and n.endswith(".xml")),
        None,
    )
    if sheet_path is None:
        raise RuntimeError("no sheet1.xml found in xlsx")
    blobs[sheet_path] = make_sheet().encode("utf-8")

    fd, tmp = tempfile.mkstemp(suffix=".xlsx")
    os.close(fd)
    with zipfile.ZipFile(tmp, "w", zipfile.ZIP_DEFLATED) as zout:
        for n in names:
            zout.writestr(n, blobs[n])
    os.replace(tmp, path)


if __name__ == "__main__":
    rewrite(PATH)
