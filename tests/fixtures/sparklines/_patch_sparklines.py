#!/usr/bin/env python3
"""Patch an empty xlsx into a sparkline-feature fixture.

Layout (Sheet1):

         A             B   C   D   E   F   G    H            I
   2  | Plain line  | 10 | 25 | 18 | 32 | 28 | 40 | <line>      |
   3  | Markers     | 12 | 19 | 24 |  8 | 30 | 22 | <line+all>  |
   4  | Mixed       | 15 |-10 |  8 |-12 | 22 | -3 | <line+axis> |
   5  | Column      | 15 |-10 |  8 |-12 | 22 | -3 | <column>    |
   6  | Group-y A   |  5 | 10 |  7 | 11 |  9 | 12 | <line.grp>  |
   7  | Group-y B   | 80 | 95 | 90 |105 |100 |120 | <line.grp>  |
   8  | Win/Loss    |  1 | -1 |  1 |  1 | -1 |  1 | <stacked>   |

Five sparkline groups, all anchored in column H:

  group 1 (line, default)            -> H2
  group 2 (line, markers + extrema)  -> H3
  group 3 (line, axis + neg color)   -> H4
  group 4 (column, neg color)        -> H5
  group 5 (line, minAxisType=group + maxAxisType=group, shared y-scale across rows 6 & 7)
                                     -> H6, H7
  group 6 (stacked / win-loss)       -> H8

We patch the OOXML directly because hsx's public JS API does not expose
the x14 sparkline schema; sparkline groups live under
`<extLst>/<ext uri="{05C60535-1F16-4fd2-B633-F4F36F0B64E0}">/<x14:sparklineGroups>`.
"""
import sys, zipfile, os, tempfile

PATH = sys.argv[1]

# (label, [v1..v6])
ROWS = [
    ("Plain line", [10, 25, 18, 32, 28, 40]),
    ("Markers",    [12, 19, 24,  8, 30, 22]),
    ("Mixed",      [15,-10,  8,-12, 22, -3]),
    ("Column",     [15,-10,  8,-12, 22, -3]),
    ("Group-y A",  [ 5, 10,  7, 11,  9, 12]),
    ("Group-y B",  [80, 95, 90,105,100,120]),
    ("Win/Loss",   [ 1, -1,  1,  1, -1,  1]),
]
ROW_INDEX_OFFSET = 2  # first row is row 2

def col_letter(n: int) -> str:
    s = ""
    while n > 0:
        n, r = divmod(n - 1, 26)
        s = chr(65 + r) + s
    return s

def cells_for_row(label: str, vals, row: int) -> str:
    out = [f'<c r="A{row}" t="inlineStr"><is><t>{label}</t></is></c>']
    for i, v in enumerate(vals):
        col = col_letter(2 + i)
        out.append(f'<c r="{col}{row}"><v>{v}</v></c>')
    return "".join(out)

def make_sheet() -> str:
    rows_xml = []
    for i, (label, vals) in enumerate(ROWS):
        r = ROW_INDEX_OFFSET + i
        # Wider rows so sparklines have headroom.
        rows_xml.append(f'<row r="{r}" ht="22" customHeight="1">{cells_for_row(label, vals, r)}</row>')

    # Five sparkline groups, all under x14:sparklineGroups.
    # Sparkline data ranges are B<r>:G<r>; anchors are H<r>.
    def grp(open_attrs: str, color_children: str, sparks: list[tuple[int, int]], extra_inner="") -> str:
        # `sparks` = list of (anchor_row, data_row) tuples.
        sl_xml = "".join(
            f'<x14:sparkline><xne:f>Sheet1!B{dr}:G{dr}</xne:f><xne:sqref>H{ar}</xne:sqref></x14:sparkline>'
            for ar, dr in sparks
        )
        return (
            f'<x14:sparklineGroup {open_attrs}>'
            f'{color_children}{extra_inner}'
            f'<x14:sparklines>{sl_xml}</x14:sparklines>'
            f'</x14:sparklineGroup>'
        )

    # Default colors: Excel's typical out-of-the-box sparkline blue, with
    # red negatives where relevant.
    series_blue = '<x14:colorSeries rgb="FF376092"/>'
    neg_red     = '<x14:colorNegative rgb="FFFF0000"/>'
    axis_black  = '<x14:colorAxis rgb="FF000000"/>'
    markers_red = '<x14:colorMarkers rgb="FFD00000"/>'
    high_green  = '<x14:colorHigh rgb="FF00B050"/>'
    low_red     = '<x14:colorLow rgb="FFFF0000"/>'
    first_grn   = '<x14:colorFirst rgb="FF92D050"/>'
    last_grn    = '<x14:colorLast rgb="FF92D050"/>'

    g1 = grp(
        'displayEmptyCellsAs="gap"',
        series_blue + neg_red + axis_black + markers_red + first_grn + last_grn + high_green + low_red,
        [(2, 2)],
    )
    g2 = grp(
        'displayEmptyCellsAs="gap" markers="1" high="1" low="1" first="1" last="1"',
        series_blue + neg_red + axis_black + markers_red + first_grn + last_grn + high_green + low_red,
        [(3, 3)],
    )
    g3 = grp(
        'displayEmptyCellsAs="gap" negative="1" displayXAxis="1"',
        series_blue + neg_red + axis_black + markers_red + first_grn + last_grn + high_green + low_red,
        [(4, 4)],
    )
    g4 = grp(
        'displayEmptyCellsAs="gap" type="column" negative="1" high="1" low="1"',
        series_blue + neg_red + axis_black + markers_red + first_grn + last_grn + high_green + low_red,
        [(5, 5)],
    )
    g5 = grp(
        'displayEmptyCellsAs="gap" minAxisType="group" maxAxisType="group"',
        series_blue + neg_red + axis_black + markers_red + first_grn + last_grn + high_green + low_red,
        [(6, 6), (7, 7)],
    )
    g6 = grp(
        'displayEmptyCellsAs="gap" type="stacked"',
        series_blue + neg_red + axis_black + markers_red + first_grn + last_grn + high_green + low_red,
        [(8, 8)],
    )

    sparkline_groups = g1 + g2 + g3 + g4 + g5 + g6

    ext_lst = (
        '<extLst>'
        '<ext uri="{05C60535-1F16-4fd2-B633-F4F36F0B64E0}" '
        'xmlns:x14="http://schemas.microsoft.com/office/spreadsheetml/2009/9/main">'
        f'<x14:sparklineGroups xmlns:xne="http://schemas.microsoft.com/office/excel/2006/main">'
        f'{sparkline_groups}'
        '</x14:sparklineGroups>'
        '</ext>'
        '</extLst>'
    )

    return f"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
           xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
           xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006"
           xmlns:x14="http://schemas.microsoft.com/office/spreadsheetml/2009/9/main"
           mc:Ignorable="x14">
  <dimension ref="A2:H8"/>
  <sheetViews><sheetView workbookViewId="0"/></sheetViews>
  <sheetFormatPr defaultRowHeight="15"/>
  <cols>
    <col min="1" max="1" width="14" customWidth="1"/>
    <col min="2" max="7" width="6"  customWidth="1"/>
    <col min="8" max="8" width="22" customWidth="1"/>
  </cols>
  <sheetData>
    {''.join(rows_xml)}
  </sheetData>
  {ext_lst}
</worksheet>"""

def rewrite(path: str) -> None:
    with zipfile.ZipFile(path, "r") as zin:
        names = zin.namelist()
        blobs = {n: zin.read(n) for n in names}
    sheet_path = next((n for n in names if n.startswith("xl/worksheets/sheet") and n.endswith(".xml")), None)
    if sheet_path is None:
        raise RuntimeError("no sheet1.xml in xlsx")
    blobs[sheet_path] = make_sheet().encode("utf-8")
    fd, tmp = tempfile.mkstemp(suffix=".xlsx")
    os.close(fd)
    with zipfile.ZipFile(tmp, "w", zipfile.ZIP_DEFLATED) as zout:
        for n in names:
            zout.writestr(n, blobs[n])
    os.replace(tmp, path)

if __name__ == "__main__":
    rewrite(PATH)
