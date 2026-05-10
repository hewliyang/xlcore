#!/usr/bin/env python3
"""Rewrite an empty xlsx (created by `hsx create`) into a fixture
that exercises **every** OOXML BorderStyleValues entry — the 14
non-`none` styles defined in ECMA-376 §18.18.3:

    thin, medium, dashed, dotted, thick, double, hair,
    mediumDashed, dashDot, mediumDashDot, dashDotDot,
    mediumDashDotDot, slantDashDot

We patch the OOXML directly because:
  - hsx normalizes some of these on round-trip (e.g. slantDashDot
    survives, but generating it from the public API requires an
    enum the JS surface doesn't expose).
  - We want every style on disk byte-exact so a layout.json snapshot
    + a pixel diff are both deterministic.

Layout (2 rows × 7 cells; each cell carries the style on all four
sides; cell text is the style name):

         B            C            D            E            F            G            H
   2  [ thin ]   [ medium ]   [ thick ]   [ dashed ]  [ dotted ]   [ double ]    [ hair ]
   3  [ mDashed ][ dashDot ]  [ mDD ]    [ dDD ]     [ mDDD ]     [ slantDD ]    [   .   ]

Where mDashed = mediumDashed, mDD = mediumDashDot, dDD = dashDotDot,
mDDD = mediumDashDotDot, slantDD = slantDashDot.
"""
import os
import sys
import tempfile
import zipfile

PATH = sys.argv[1]

# OOXML ST_BorderStyle (ECMA-376 §18.18.3) — every value except "none".
STYLES = [
    "thin",
    "medium",
    "thick",
    "dashed",
    "dotted",
    "double",
    "hair",
    "mediumDashed",
    "dashDot",
    "mediumDashDot",
    "dashDotDot",
    "mediumDashDotDot",
    "slantDashDot",
]
# 14 cells fit a 2×7 grid; pad row 2 with a sentinel-empty cell so
# the layout is rectangular even though we only need 13 styles
# above (we leave the trailing slot blank rather than duplicating).
LABELS = STYLES + [""]

STYLES_XML_TMPL = """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
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


def make_border(style: str) -> str:
    sides = "".join(
        f'<{side} style="{style}"><color rgb="FF000000"/></{side}>'
        for side in ("left", "right", "top", "bottom")
    )
    return f"<border>{sides}<diagonal/></border>"


def make_styles() -> str:
    extra_borders = "".join(make_border(st) for st in STYLES)
    extra_xfs = "".join(
        f'<xf numFmtId="0" fontId="0" fillId="0" borderId="{i+1}" xfId="0" applyBorder="1"/>'
        for i in range(len(STYLES))
    )
    return STYLES_XML_TMPL.format(
        nb=1 + len(STYLES),
        nx=1 + len(STYLES),
        extra_borders=extra_borders,
        extra_xfs=extra_xfs,
    )


def make_sheet() -> str:
    rows = []
    # 2 rows × 7 cells. Style index i corresponds to xf i+1.
    for ri, row_idx in enumerate((2, 3)):
        cells = []
        for ci in range(7):
            slot = ri * 7 + ci
            if slot >= len(STYLES):
                break
            label = STYLES[slot]
            col_letter = chr(ord("B") + ci)
            ref = f"{col_letter}{row_idx}"
            cells.append(
                f'<c r="{ref}" s="{slot+1}" t="inlineStr">'
                f'<is><t>{label}</t></is></c>'
            )
        rows.append(
            f'<row r="{row_idx}" ht="40" customHeight="1">{"".join(cells)}</row>'
        )
    rows_xml = "".join(rows)
    return f"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
           xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <dimension ref="B2:H3"/>
  <sheetViews><sheetView workbookViewId="0"/></sheetViews>
  <sheetFormatPr defaultRowHeight="15"/>
  <cols>
    <col min="2" max="8" width="16" customWidth="1"/>
  </cols>
  <sheetData>{rows_xml}</sheetData>
</worksheet>"""


def rewrite(path: str) -> None:
    with zipfile.ZipFile(path, "r") as zin:
        names = zin.namelist()
        blobs = {n: zin.read(n) for n in names}

    blobs["xl/styles.xml"] = make_styles().encode("utf-8")
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
