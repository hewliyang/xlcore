#!/usr/bin/env python3
"""Build a fixture exercising OOXML `<gradientFill>` variants.

SpreadJS's public style API doesn't expose gradient fills (it only
writes solid `patternType="solid"` for `backColor`), so we patch the
OOXML directly. The fixture lays out 6 cells (B2..D3) covering:

  B2 linear, degree=0   (left -> right)        2-stop blue/red
  C2 linear, degree=90  (top  -> bottom)       2-stop blue/red
  D2 linear, degree=45  (TL   -> BR diagonal)  2-stop blue/red
  B3 linear, degree=0   3-stop blue/yellow/red (position 0, 0.5, 1)
  C3 linear, degree=270 (bottom -> top)        2-stop blue/red
  D3 path     (radial, inner rect 30%-30%-30%-30%) 2-stop blue/red

Catches regressions in (a) the schema GradientStop/gradientType/
gradientDegree/gradientLeft|Right|Top|Bottom round-trip, (b) the
renderer's multi-stop linear axis projection + path radial.
"""
import sys, zipfile, os, tempfile

PATH = sys.argv[1]

# (label, gradientFill XML inner)
BLUE = "FF1F77B4"
RED = "FFD62728"
YEL = "FFFFC000"

def stops_2(c1=BLUE, c2=RED):
    return (
        f'<stop position="0"><color rgb="{c1}"/></stop>'
        f'<stop position="1"><color rgb="{c2}"/></stop>'
    )

def stops_3():
    return (
        f'<stop position="0"><color rgb="{BLUE}"/></stop>'
        f'<stop position="0.5"><color rgb="{YEL}"/></stop>'
        f'<stop position="1"><color rgb="{RED}"/></stop>'
    )

GRADS = [
    ("linear 0\u00b0",   f'<gradientFill degree="0">{stops_2()}</gradientFill>'),
    ("linear 90\u00b0",  f'<gradientFill degree="90">{stops_2()}</gradientFill>'),
    ("linear 45\u00b0",  f'<gradientFill degree="45">{stops_2()}</gradientFill>'),
    ("3-stop B-Y-R",     f'<gradientFill degree="0">{stops_3()}</gradientFill>'),
    ("linear 270\u00b0", f'<gradientFill degree="270">{stops_2()}</gradientFill>'),
    ("path radial",      f'<gradientFill type="path" left="0.3" right="0.3" top="0.3" bottom="0.3">{stops_2()}</gradientFill>'),
]

COLS = 3  # 3 wide x 2 tall

STYLES_XML = """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <fonts count="1"><font><sz val="9"/><color rgb="FF000000"/><name val="Calibri"/></font></fonts>
  <fills count="{nf}">
    <fill><patternFill patternType="none"/></fill>
    <fill><patternFill patternType="gray125"/></fill>
    {extra_fills}
  </fills>
  <borders count="1"><border><left style="thin"><color rgb="FF808080"/></left><right style="thin"><color rgb="FF808080"/></right><top style="thin"><color rgb="FF808080"/></top><bottom style="thin"><color rgb="FF808080"/></bottom><diagonal/></border></borders>
  <cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs>
  <cellXfs count="{nx}">
    <xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/>
    {extra_xfs}
  </cellXfs>
  <cellStyles count="1"><cellStyle name="Normal" xfId="0" builtinId="0"/></cellStyles>
</styleSheet>"""


def make_styles() -> str:
    extra_fills = "".join(f'<fill>{xml}</fill>' for _label, xml in GRADS)
    extra_xfs = "".join(
        f'<xf numFmtId="0" fontId="0" fillId="{i+2}" borderId="0" xfId="0" applyFill="1" applyBorder="1" applyAlignment="1"><alignment horizontal="center" vertical="center"/></xf>'
        for i in range(len(GRADS))
    )
    return STYLES_XML.format(
        nf=2 + len(GRADS),
        nx=1 + len(GRADS),
        extra_fills=extra_fills,
        extra_xfs=extra_xfs,
    )


def make_sheet() -> str:
    cells = []
    for i, (label, _xml) in enumerate(GRADS):
        row = 2 + (i // COLS)  # rows 2, 3
        col = 1 + (i % COLS)
        col_letter = chr(ord("A") + col)
        ref = f"{col_letter}{row}"
        cells.append((row, f'<c r="{ref}" s="{i+1}" t="inlineStr"><is><t>{label}</t></is></c>'))
    by_row: dict[int, list[str]] = {}
    for r, c in cells:
        by_row.setdefault(r, []).append(c)
    rows_xml = "".join(
        f'<row r="{r}" ht="60" customHeight="1">{"".join(by_row[r])}</row>'
        for r in sorted(by_row)
    )
    return f"""<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>
<worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"
           xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\">
  <dimension ref=\"B2:D3\"/>
  <sheetViews><sheetView workbookViewId=\"0\"/></sheetViews>
  <sheetFormatPr defaultRowHeight=\"15\"/>
  <cols>
    <col min=\"2\" max=\"{1 + COLS}\" width=\"22\" customWidth=\"1\"/>
  </cols>
  <sheetData>{rows_xml}</sheetData>
</worksheet>"""


def rewrite(path: str) -> None:
    with zipfile.ZipFile(path, "r") as zin:
        names = zin.namelist()
        blobs = {n: zin.read(n) for n in names}
    blobs["xl/styles.xml"] = make_styles().encode("utf-8")
    sheet_path = next((n for n in names if n.startswith("xl/worksheets/sheet") and n.endswith(".xml")), None)
    if sheet_path is None:
        raise RuntimeError("no worksheet found")
    blobs[sheet_path] = make_sheet().encode("utf-8")
    fd, tmp = tempfile.mkstemp(suffix=".xlsx")
    os.close(fd)
    with zipfile.ZipFile(tmp, "w", zipfile.ZIP_DEFLATED) as zout:
        for n in names:
            zout.writestr(n, blobs[n])
    os.replace(tmp, path)


if __name__ == "__main__":
    rewrite(PATH)
