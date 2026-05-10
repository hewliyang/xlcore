#!/usr/bin/env python3
"""Build a workbook that exercises CF rule priority + cross-kind
`stopIfTrue` masking. SpreadJS doesn't expose `stopIfTrue` on its
public xlsx-emit path so we patch the OOXML directly.

Layout (one sheet, 4 columns x 10 data rows):

       A         B         C         D
 1     no-stop   stop-cs   stop-db   stop-icon
 2..11 1..10     1..10     1..10     1..10

Rule structure (priority lower = higher precedence):

  Column A — control. cellIs(>7) yellow, priority 2, stopIfTrue=false.
            colorScale (green→red) priority 1.
            Both rules paint together: cells 8..10 should overlay yellow
            on top of the colorScale red. (We render colorScale OVER
            dxf today, so without stopIfTrue the yellow loses — that's
            existing — but cells 1..7 still show colorScale.) This is
            the "no stopIfTrue" baseline.

  Column B — `stop-cs`: cellIs(>7) yellow, priority 1, stopIfTrue=true.
            colorScale (green→red) priority 2 over the same range.
            Expected with the fix: cells 8..10 paint yellow only
            (colorScale skipped); cells 1..7 paint colorScale.

  Column C — `stop-db`: cellIs(>7) yellow, priority 1, stopIfTrue=true.
            dataBar priority 2 over same range.
            Expected: cells 8..10 yellow w/ no bar; 1..7 standard bar.

  Column D — `stop-icon`: cellIs(>7) yellow, priority 1, stopIfTrue=true.
            iconSet (3Arrows) priority 2 over same range.
            Expected: cells 8..10 yellow w/ no icon; 1..7 normal icon.

Without our fix, the colorScale / dataBar / iconSet pass ignores the
higher-priority cellIs's stopIfTrue and paints over rows 8..10 in cols
B..D anyway.
"""
import sys, zipfile, os, tempfile

PATH = sys.argv[1]

STYLES_XML = """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <fonts count="2">
    <font><sz val="11"/><color rgb="FF000000"/><name val="Calibri"/></font>
    <font><b/><sz val="11"/><color rgb="FF000000"/><name val="Calibri"/></font>
  </fonts>
  <fills count="2"><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="gray125"/></fill></fills>
  <borders count="1"><border><left/><right/><top/><bottom/><diagonal/></border></borders>
  <cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs>
  <cellXfs count="2">
    <xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/>
    <xf numFmtId="0" fontId="1" fillId="0" borderId="0" xfId="0" applyFont="1"/>
  </cellXfs>
  <cellStyles count="1"><cellStyle name="Normal" xfId="0" builtinId="0"/></cellStyles>
  <dxfs count="1">
    <dxf>
      <font><b/><color rgb="FF000000"/></font>
      <fill><patternFill><bgColor rgb="FFFFD966"/></patternFill></fill>
    </dxf>
  </dxfs>
</styleSheet>"""


def make_sheet() -> str:
    # Header row.
    headers = ["no-stop", "stop-cs", "stop-db", "stop-icon"]
    rows = []
    h_cells = []
    for i, h in enumerate(headers):
        col = chr(ord("A") + i)
        h_cells.append(f'<c r="{col}1" s="1" t="inlineStr"><is><t>{h}</t></is></c>')
    rows.append(f'<row r="1">{"".join(h_cells)}</row>')

    for r in range(2, 12):
        v = r - 1  # 1..10
        cells = []
        for i in range(4):
            col = chr(ord("A") + i)
            cells.append(f'<c r="{col}{r}"><v>{v}</v></c>')
        rows.append(f'<row r="{r}">{"".join(cells)}</row>')

    # CF blocks. Priorities are global across blocks; we keep them
    # interleaved per column so cellIs always wins precedence.
    cf_blocks = []

    # Column A — control: colorScale (priority 1) + cellIs (priority 2,
    # stopIfTrue=false). Both apply.
    cf_blocks.append(
        '<conditionalFormatting sqref="A2:A11">'
        '<cfRule type="colorScale" priority="2">'
        '<colorScale>'
        '<cfvo type="min"/><cfvo type="percentile" val="50"/><cfvo type="max"/>'
        '<color rgb="FF63BE7B"/><color rgb="FFFFEB84"/><color rgb="FFF8696B"/>'
        '</colorScale>'
        '</cfRule>'
        '<cfRule type="cellIs" dxfId="0" priority="1" operator="greaterThan">'
        '<formula>7</formula></cfRule>'
        '</conditionalFormatting>'
    )

    # Column B — stop-cs: cellIs (priority 3, stopIfTrue=true) wins
    # masking the colorScale (priority 4).
    cf_blocks.append(
        '<conditionalFormatting sqref="B2:B11">'
        '<cfRule type="cellIs" dxfId="0" priority="3" operator="greaterThan" stopIfTrue="1">'
        '<formula>7</formula></cfRule>'
        '<cfRule type="colorScale" priority="4">'
        '<colorScale>'
        '<cfvo type="min"/><cfvo type="percentile" val="50"/><cfvo type="max"/>'
        '<color rgb="FF63BE7B"/><color rgb="FFFFEB84"/><color rgb="FFF8696B"/>'
        '</colorScale>'
        '</cfRule>'
        '</conditionalFormatting>'
    )

    # Column C — stop-db: cellIs (priority 5, stopIfTrue=true) masks
    # dataBar (priority 6).
    cf_blocks.append(
        '<conditionalFormatting sqref="C2:C11">'
        '<cfRule type="cellIs" dxfId="0" priority="5" operator="greaterThan" stopIfTrue="1">'
        '<formula>7</formula></cfRule>'
        '<cfRule type="dataBar" priority="6">'
        '<dataBar><cfvo type="min"/><cfvo type="max"/>'
        '<color rgb="FF638EC6"/></dataBar>'
        '</cfRule>'
        '</conditionalFormatting>'
    )

    # Column D — stop-icon: cellIs (priority 7, stopIfTrue=true) masks
    # iconSet (priority 8). 3Arrows preset.
    cf_blocks.append(
        '<conditionalFormatting sqref="D2:D11">'
        '<cfRule type="cellIs" dxfId="0" priority="7" operator="greaterThan" stopIfTrue="1">'
        '<formula>7</formula></cfRule>'
        '<cfRule type="iconSet" priority="8">'
        '<iconSet iconSet="3Arrows">'
        '<cfvo type="percent" val="0"/>'
        '<cfvo type="percent" val="33"/>'
        '<cfvo type="percent" val="67"/>'
        '</iconSet>'
        '</cfRule>'
        '</conditionalFormatting>'
    )

    return f"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
           xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <dimension ref="A1:D11"/>
  <sheetViews><sheetView workbookViewId="0"/></sheetViews>
  <sheetFormatPr defaultRowHeight="15"/>
  <cols>
    <col min="1" max="4" width="14" customWidth="1"/>
  </cols>
  <sheetData>
    {''.join(rows)}
  </sheetData>
  {''.join(cf_blocks)}
</worksheet>"""


def rewrite(path: str) -> None:
    with zipfile.ZipFile(path, "r") as zin:
        names = zin.namelist()
        blobs = {n: zin.read(n) for n in names}

    blobs["xl/styles.xml"] = STYLES_XML.encode("utf-8")
    sheet_path = next((n for n in names if n.startswith("xl/worksheets/sheet") and n.endswith(".xml")), None)
    if sheet_path is None:
        raise RuntimeError("no sheet1.xml found")
    blobs[sheet_path] = make_sheet().encode("utf-8")

    fd, tmp = tempfile.mkstemp(suffix=".xlsx")
    os.close(fd)
    with zipfile.ZipFile(tmp, "w", zipfile.ZIP_DEFLATED) as zout:
        for n in names:
            zout.writestr(n, blobs[n])
    os.replace(tmp, path)


if __name__ == "__main__":
    rewrite(PATH)
