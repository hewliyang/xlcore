#!/usr/bin/env python3
"""Rewrite an empty xlsx into a fixture exercising OOXML
`<vertAlign val="superscript|subscript"/>` (ECMA-376 §18.4.14).

Two flavors are covered:

  1. Cell-font vertAlign — `<font><vertAlign val="superscript"/></font>`
     in `styles.xml`. The whole cell renders at sub/super size+position.
  2. Rich-text run vertAlign — `<rPr><vertAlign val="..."/></rPr>` inside
     an inline `<is><r>...</r></is>` block, mixed with baseline runs.

We patch the OOXML directly because hsx's public JS surface doesn't
expose vertAlign on either path.

Layout (row 2 = label, row 3 = sample):

       B            C            D            E              F
  Cell-Super    Cell-Sub      H_2O         x^2          E = mc^2
   X²-ish      X-sub-ish     [H][2sub][O] [x][2sup]   [E = mc][2sup]
"""
import os
import sys
import tempfile
import zipfile

PATH = sys.argv[1]

STYLES_XML = """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <fonts count="3">
    <!-- 0: plain baseline -->
    <font><sz val="14"/><color rgb="FF000000"/><name val="Calibri"/></font>
    <!-- 1: cell-font superscript -->
    <font><sz val="14"/><color rgb="FF000000"/><name val="Calibri"/><vertAlign val="superscript"/></font>
    <!-- 2: cell-font subscript -->
    <font><sz val="14"/><color rgb="FF000000"/><name val="Calibri"/><vertAlign val="subscript"/></font>
  </fonts>
  <fills count="2">
    <fill><patternFill patternType="none"/></fill>
    <fill><patternFill patternType="gray125"/></fill>
  </fills>
  <borders count="1"><border><left/><right/><top/><bottom/><diagonal/></border></borders>
  <cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs>
  <cellXfs count="3">
    <xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/>
    <xf numFmtId="0" fontId="1" fillId="0" borderId="0" xfId="0" applyFont="1"/>
    <xf numFmtId="0" fontId="2" fillId="0" borderId="0" xfId="0" applyFont="1"/>
  </cellXfs>
  <cellStyles count="1"><cellStyle name="Normal" xfId="0" builtinId="0"/></cellStyles>
</styleSheet>"""


def run(text: str, *, sup: bool = False, sub: bool = False) -> str:
    """One `<r>` rich-text run, optionally with vertAlign."""
    va = ""
    if sup:
        va = '<vertAlign val="superscript"/>'
    elif sub:
        va = '<vertAlign val="subscript"/>'
    rpr = f"<rPr><sz val=\"14\"/><color rgb=\"FF000000\"/><rFont val=\"Calibri\"/>{va}</rPr>"
    # `xml:space="preserve"` so any leading/trailing spaces in `text` survive.
    return f'<r>{rpr}<t xml:space="preserve">{text}</t></r>'


def cell_inline_rich(ref: str, runs: str, s: int = 0) -> str:
    return f'<c r="{ref}" s="{s}" t="inlineStr"><is>{runs}</is></c>'


def cell_inline_plain(ref: str, text: str, s: int = 0) -> str:
    return f'<c r="{ref}" s="{s}" t="inlineStr"><is><t>{text}</t></is></c>'


def make_sheet() -> str:
    # Row 2: labels (plain text, baseline style).
    headers = [
        ("B2", "Cell-Super"),
        ("C2", "Cell-Sub"),
        ("D2", "H2O (rich)"),
        ("E2", "x^2 (rich)"),
        ("F2", "E=mc^2 (rich)"),
    ]
    row2 = "".join(cell_inline_plain(r, t) for r, t in headers)

    # Row 3: samples.
    # B3: whole-cell superscript via xf=1.
    # C3: whole-cell subscript via xf=2.
    # D3: H, 2(sub), O.
    # E3: x, 2(sup).
    # F3: "E = mc", "2"(sup).
    row3_cells = [
        cell_inline_plain("B3", "tiny up", s=1),
        cell_inline_plain("C3", "tiny down", s=2),
        cell_inline_rich(
            "D3",
            run("H") + run("2", sub=True) + run("O"),
        ),
        cell_inline_rich(
            "E3",
            run("x") + run("2", sup=True),
        ),
        cell_inline_rich(
            "F3",
            run("E = mc") + run("2", sup=True),
        ),
    ]
    row3 = "".join(row3_cells)

    rows_xml = (
        f'<row r="2" ht="22" customHeight="1">{row2}</row>'
        f'<row r="3" ht="32" customHeight="1">{row3}</row>'
    )
    return f"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
           xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <dimension ref="B2:F3"/>
  <sheetViews><sheetView workbookViewId="0"/></sheetViews>
  <sheetFormatPr defaultRowHeight="15"/>
  <cols><col min="2" max="6" width="22" customWidth="1"/></cols>
  <sheetData>{rows_xml}</sheetData>
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
