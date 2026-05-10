#!/usr/bin/env python3
"""Rewrite an empty xlsx into a fixture exercising every OOXML
underline variant (ECMA-376 §18.18.91 ST_UnderlineValues):

    single, double, singleAccounting, doubleAccounting

We patch the OOXML directly because hsx's public JS surface only
exposes a single boolean `underline()` setter; there's no way to
emit `<u val="doubleAccounting"/>` through the public API.

Layout:

         B          C          D            E              F
   2  [no u]    [single]   [double]   [singleAcct]   [doubleAcct]
   3  Lorem    Lorem      Lorem       $1,234.50      $1,234.50

Row 2 carries the variant name (so a layout.json snapshot is
self-documenting). Row 3 carries representative content: plain text
for single/double, currency-shaped text for the accounting variants
(matches Excel's typical use).
"""
import os
import sys
import tempfile
import zipfile

PATH = sys.argv[1]

# (name, val-attr-or-None) — None means no `val` attr (defaults to single).
VARIANTS = [
    ("none", None),  # no underline at all (control)
    ("single", "single"),
    ("double", "double"),
    ("singleAccounting", "singleAccounting"),
    ("doubleAccounting", "doubleAccounting"),
]

STYLES_XML_TMPL = """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <fonts count="{nf}">
    <font><sz val="14"/><color rgb="FF000000"/><name val="Calibri"/></font>
    {extra_fonts}
  </fonts>
  <fills count="2"><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="gray125"/></fill></fills>
  <borders count="1"><border><left/><right/><top/><bottom/><diagonal/></border></borders>
  <cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs>
  <cellXfs count="{nx}">
    <xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/>
    {extra_xfs}
  </cellXfs>
  <cellStyles count="1"><cellStyle name="Normal" xfId="0" builtinId="0"/></cellStyles>
</styleSheet>"""


def make_font(val: str | None) -> str:
    # Font 0 is plain, fontId 1+ each carry one underline variant.
    # `None` means: no underline element at all (the "none" control).
    u = ""
    if val is not None:
        if val == "single":
            # Element with no `val` attr — defaults to single.
            u = "<u/>"
        else:
            u = f'<u val="{val}"/>'
    return f'<font><sz val="14"/><color rgb="FF000000"/><name val="Calibri"/>{u}</font>'


def make_styles() -> str:
    # font 0 = plain (used by "no underline" cells)
    # fonts 1..N correspond to VARIANTS[1..N] (skipping the "none" control,
    # which reuses font 0).
    underlined = [v for v in VARIANTS if v[1] is not None]
    extra_fonts = "".join(make_font(val) for _, val in underlined)
    nf = 1 + len(underlined)
    # cellXfs: xf 0 = default; xf i (i>=1) uses fontId=i.
    extra_xfs = "".join(
        f'<xf numFmtId="0" fontId="{i+1}" fillId="0" borderId="0" xfId="0" applyFont="1"/>'
        for i in range(len(underlined))
    )
    nx = 1 + len(underlined)
    return STYLES_XML_TMPL.format(
        nf=nf, nx=nx, extra_fonts=extra_fonts, extra_xfs=extra_xfs
    )


def make_sheet() -> str:
    # Headers in row 2 (style names). Body in row 3.
    row2_cells = []
    row3_cells = []
    # xf index for each variant cell: 0 for "none" (plain), then 1..N for underlined.
    underlined_idx = 0
    for ci, (name, val) in enumerate(VARIANTS):
        col = chr(ord("B") + ci)
        if val is None:
            xf = 0
            sample = "Lorem"
        else:
            underlined_idx += 1
            xf = underlined_idx
            sample = (
                "$1,234.50"
                if "Accounting" in name
                else "Lorem"
            )
        row2_cells.append(
            f'<c r="{col}2" t="inlineStr"><is><t>{name}</t></is></c>'
        )
        row3_cells.append(
            f'<c r="{col}3" s="{xf}" t="inlineStr"><is><t>{sample}</t></is></c>'
        )
    rows_xml = (
        f'<row r="2" ht="22" customHeight="1">{"".join(row2_cells)}</row>'
        f'<row r="3" ht="32" customHeight="1">{"".join(row3_cells)}</row>'
    )
    return f"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
           xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <dimension ref="B2:F3"/>
  <sheetViews><sheetView workbookViewId="0"/></sheetViews>
  <sheetFormatPr defaultRowHeight="15"/>
  <cols><col min="2" max="6" width="20" customWidth="1"/></cols>
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
