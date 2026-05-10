#!/usr/bin/env python3
"""Build a fixture exercising every OOXML hatch pattern type.

SpreadJS doesn't surface the hatch fill enum on its public style API
(it only writes `solid` patterns when you set `backColor`), so we
patch the OOXML directly. The fixture lays out the 16 non-solid
hatch patterns in a 4x4 grid (B2..E5), each cell labeled with the
pattern name. Foreground = dark blue (#1F3864), background = white.

Catches regressions in (a) extractor's `pattern_type_to_str` mapping
the full `PatternValues` enum, (b) renderer's `paintFill` building
the right 8x8 tile via `PATTERN_TILES_8X8`.
"""
import sys, zipfile, os, tempfile

PATH = sys.argv[1]

PATTERNS = [
    "gray125", "gray0625", "lightGray", "mediumGray",
    "darkGray", "lightHorizontal", "darkHorizontal", "lightVertical",
    "darkVertical", "lightDown", "darkDown", "lightUp",
    "darkUp", "lightGrid", "darkGrid", "lightTrellis",
    "darkTrellis", "solid",
]
# 18 cells laid out 6 cols x 3 rows (B2..G4).
COLS = 6

FG = "FF1F3864"  # dark blue
BG = "FFFFFFFF"  # white

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


def make_fill(pt: str) -> str:
    return (
        f'<fill><patternFill patternType="{pt}">'
        f'<fgColor rgb="{FG}"/><bgColor rgb="{BG}"/>'
        f'</patternFill></fill>'
    )


def make_styles() -> str:
    extra_fills = "".join(make_fill(p) for p in PATTERNS)
    extra_xfs = "".join(
        f'<xf numFmtId="0" fontId="0" fillId="{i+2}" borderId="0" xfId="0" applyFill="1" applyBorder="1" applyAlignment="1"><alignment horizontal="center" vertical="center"/></xf>'
        for i in range(len(PATTERNS))
    )
    return STYLES_XML.format(
        nf=2 + len(PATTERNS),
        nx=1 + len(PATTERNS),
        extra_fills=extra_fills,
        extra_xfs=extra_xfs,
    )


def make_sheet() -> str:
    cells = []
    for i, pt in enumerate(PATTERNS):
        row = 2 + (i // COLS)  # rows 2,3,4
        col = 1 + (i % COLS)   # 1=B, 2=C, ...
        col_letter = chr(ord("A") + col)
        ref = f"{col_letter}{row}"
        cells.append((row, f'<c r="{ref}" s="{i+1}" t="inlineStr"><is><t>{pt}</t></is></c>'))
    by_row: dict[int, list[str]] = {}
    for r, c in cells:
        by_row.setdefault(r, []).append(c)
    rows_xml = "".join(
        f'<row r="{r}" ht="42" customHeight="1">{"".join(by_row[r])}</row>'
        for r in sorted(by_row)
    )
    last_row = 1 + (len(PATTERNS) + COLS - 1) // COLS
    return f"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
           xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <dimension ref="B2:G{last_row}"/>
  <sheetViews><sheetView workbookViewId="0"/></sheetViews>
  <sheetFormatPr defaultRowHeight="15"/>
  <cols>
    <col min="2" max="{1 + COLS}" width="14" customWidth="1"/>
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
