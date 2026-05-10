#!/usr/bin/env python3
"""Rewrite an empty xlsx (created by `hsx create`) into a fixture with
five cells in row 2 (B2..F2), each carrying a different diagonal-border
configuration. SpreadJS silently drops `borderDiagonalUp` /
`borderDiagonalDown` on xlsx export, so we patch the OOXML directly.

Layout (column widths 80px, row height 50px):

      B            C            D            E            F
   2  [ \\ thin ]  [ / thin ]  [ X thin ]  [ X thick ]  [ X red dashed ]

Each cell's xf references one of five new <border> entries in
xl/styles.xml. <border> carries `diagonalUp` / `diagonalDown` attrs +
a single `<diagonal>` child holding style + color (both diagonals share
the style; that's how OOXML serializes it).
"""
import sys, zipfile, re, os, tempfile

PATH = sys.argv[1]

# (label, diagonalUp, diagonalDown, style, rgb)
BORDERS = [
    ("\\",          False, True,  "thin",   "FF000000"),
    ("/",           True,  False, "thin",   "FF000000"),
    ("X",           True,  True,  "thin",   "FF000000"),
    ("X thick",     True,  True,  "thick",  "FF000000"),
    ("X red dash",  True,  True,  "dashed", "FFC00000"),
]

STYLES_XML = """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <fonts count="1"><font><sz val="11"/><color rgb="FF000000"/><name val="Calibri"/></font></fonts>
  <fills count="2"><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="gray125"/></fill></fills>
  <borders count="{nb}">
    <border><left/><right/><top/><bottom/><diagonal/></border>
    {extra_borders}
  </borders>
  <cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs>
  <cellXfs count="{nx}">
    <xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/>
    {extra_xfs}
  </cellXfs>
  <cellStyles count="1"><cellStyle name="Normal" xfId="0" builtinId="0"/></cellStyles>
</styleSheet>"""


def make_border(up: bool, down: bool, style: str, rgb: str) -> str:
    attrs = []
    if up:   attrs.append('diagonalUp="1"')
    if down: attrs.append('diagonalDown="1"')
    attrs_s = (" " + " ".join(attrs)) if attrs else ""
    return (f'<border{attrs_s}>'
            f'<left/><right/><top/><bottom/>'
            f'<diagonal style="{style}"><color rgb="{rgb}"/></diagonal>'
            f'</border>')


def make_styles() -> str:
    extra_borders = "".join(make_border(up, down, st, rgb) for (_, up, down, st, rgb) in BORDERS)
    extra_xfs = "".join(
        f'<xf numFmtId="0" fontId="0" fillId="0" borderId="{i+1}" xfId="0" applyBorder="1"/>'
        for i in range(len(BORDERS))
    )
    return STYLES_XML.format(nb=1 + len(BORDERS), nx=1 + len(BORDERS), extra_borders=extra_borders, extra_xfs=extra_xfs)


def make_sheet() -> str:
    # 5 cells in row 2: B2..F2. xf indices 1..5.
    cells = []
    for i, (label, *_rest) in enumerate(BORDERS):
        col_letter = chr(ord("B") + i)
        ref = f"{col_letter}2"
        # inlineStr for simplicity — no shared-strings table needed.
        cells.append(
            f'<c r="{ref}" s="{i+1}" t="inlineStr"><is><t>{label.replace("&", "&amp;")}</t></is></c>'
        )
    cells_xml = "".join(cells)
    return f"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
           xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <dimension ref="B2:F2"/>
  <sheetViews><sheetView workbookViewId="0"/></sheetViews>
  <sheetFormatPr defaultRowHeight="15"/>
  <cols>
    <col min="2" max="6" width="11.5" customWidth="1"/>
  </cols>
  <sheetData>
    <row r="2" ht="37.5" customHeight="1">{cells_xml}</row>
  </sheetData>
</worksheet>"""


def rewrite(path: str) -> None:
    with zipfile.ZipFile(path, "r") as zin:
        names = zin.namelist()
        blobs = {n: zin.read(n) for n in names}

    blobs["xl/styles.xml"] = make_styles().encode("utf-8")
    # find the worksheet path: typically xl/worksheets/sheet1.xml
    sheet_path = next((n for n in names if n.startswith("xl/worksheets/sheet") and n.endswith(".xml")), None)
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
