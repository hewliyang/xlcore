#!/usr/bin/env python3
"""Rewrite an empty xlsx into a fixture exercising OOXML font-family
detection: `<scheme val="major|minor"/>` (theme-font references) and
`<family val="N"/>` (numeric family hint used as a CSS fallback class).

The fixture defines a custom theme1.xml whose major-font is "Georgia"
(a serif) and minor-font is "Verdana" (a sans-serif). Each test cell
uses a fontId whose `<scheme>` references one of those slots; the
denormalized `<name>` is intentionally set to `"WRONG"` so a renderer
that ignores `<scheme>` will paint the wrong typeface.

A separate row exercises the `<family>` numeric hint with an obviously
non-existent typeface ("NotInstalledFontXYZ") and family values 1..5;
the browser falls through to each family's CSS generic.

Layout (row 2 = header label, row 3 = sample):

    B          C            D           E            F           G
  major-     minor-     family=1    family=3    family=4    family=5
  scheme    scheme     (Roman)     (Modern)    (Script)    (Decorative)
"""
import os
import sys
import tempfile
import zipfile

PATH = sys.argv[1]


# Custom theme: major=Georgia (serif), minor=Verdana (sans). Spreadsheet
# index slots reuse the Office 2007+ defaults — we only care about the
# font scheme here, not colors.
THEME_XML = """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="FontDetectFixture">
  <a:themeElements>
    <a:clrScheme name="Office">
      <a:dk1><a:sysClr val="windowText" lastClr="000000"/></a:dk1>
      <a:lt1><a:sysClr val="window" lastClr="FFFFFF"/></a:lt1>
      <a:dk2><a:srgbClr val="44546A"/></a:dk2>
      <a:lt2><a:srgbClr val="E7E6E6"/></a:lt2>
      <a:accent1><a:srgbClr val="4472C4"/></a:accent1>
      <a:accent2><a:srgbClr val="ED7D31"/></a:accent2>
      <a:accent3><a:srgbClr val="A5A5A5"/></a:accent3>
      <a:accent4><a:srgbClr val="FFC000"/></a:accent4>
      <a:accent5><a:srgbClr val="5B9BD5"/></a:accent5>
      <a:accent6><a:srgbClr val="70AD47"/></a:accent6>
      <a:hlink><a:srgbClr val="0563C1"/></a:hlink>
      <a:folHlink><a:srgbClr val="954F72"/></a:folHlink>
    </a:clrScheme>
    <a:fontScheme name="FontDetectFixture">
      <a:majorFont>
        <a:latin typeface="Georgia"/>
        <a:ea typeface=""/>
        <a:cs typeface=""/>
      </a:majorFont>
      <a:minorFont>
        <a:latin typeface="Verdana"/>
        <a:ea typeface=""/>
        <a:cs typeface=""/>
      </a:minorFont>
    </a:fontScheme>
    <a:fmtScheme name="Office">
      <a:fillStyleLst>
        <a:solidFill><a:schemeClr val="phClr"/></a:solidFill>
        <a:solidFill><a:schemeClr val="phClr"/></a:solidFill>
        <a:solidFill><a:schemeClr val="phClr"/></a:solidFill>
      </a:fillStyleLst>
      <a:lnStyleLst>
        <a:ln w="6350" cap="flat" cmpd="sng" algn="ctr"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln>
        <a:ln w="12700" cap="flat" cmpd="sng" algn="ctr"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln>
        <a:ln w="19050" cap="flat" cmpd="sng" algn="ctr"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln>
      </a:lnStyleLst>
      <a:effectStyleLst>
        <a:effectStyle><a:effectLst/></a:effectStyle>
        <a:effectStyle><a:effectLst/></a:effectStyle>
        <a:effectStyle><a:effectLst/></a:effectStyle>
      </a:effectStyleLst>
      <a:bgFillStyleLst>
        <a:solidFill><a:schemeClr val="phClr"/></a:solidFill>
        <a:solidFill><a:schemeClr val="phClr"/></a:solidFill>
        <a:solidFill><a:schemeClr val="phClr"/></a:solidFill>
      </a:bgFillStyleLst>
    </a:fmtScheme>
  </a:themeElements>
</a:theme>"""


# Fonts:
#   0: default (Calibri 11)
#   1: scheme=major + name=WRONG  -> should resolve to Georgia
#   2: scheme=minor + name=WRONG  -> should resolve to Verdana
#   3: name=NotInstalledFontXYZ + family=1 (Roman/serif)
#   4: name=NotInstalledFontXYZ + family=3 (Modern/monospace)
#   5: name=NotInstalledFontXYZ + family=4 (Script/cursive)
#   6: name=NotInstalledFontXYZ + family=5 (Decorative/fantasy)
STYLES_XML = """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <fonts count="7">
    <font><sz val="14"/><color rgb="FF000000"/><name val="Calibri"/></font>
    <font><sz val="20"/><color rgb="FF000000"/><name val="WRONG"/><scheme val="major"/></font>
    <font><sz val="20"/><color rgb="FF000000"/><name val="WRONG"/><scheme val="minor"/></font>
    <font><sz val="20"/><color rgb="FF000000"/><name val="NotInstalledFontXYZ"/><family val="1"/></font>
    <font><sz val="20"/><color rgb="FF000000"/><name val="NotInstalledFontXYZ"/><family val="3"/></font>
    <font><sz val="20"/><color rgb="FF000000"/><name val="NotInstalledFontXYZ"/><family val="4"/></font>
    <font><sz val="20"/><color rgb="FF000000"/><name val="NotInstalledFontXYZ"/><family val="5"/></font>
  </fonts>
  <fills count="2">
    <fill><patternFill patternType="none"/></fill>
    <fill><patternFill patternType="gray125"/></fill>
  </fills>
  <borders count="1"><border><left/><right/><top/><bottom/><diagonal/></border></borders>
  <cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs>
  <cellXfs count="7">
    <xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/>
    <xf numFmtId="0" fontId="1" fillId="0" borderId="0" xfId="0" applyFont="1"/>
    <xf numFmtId="0" fontId="2" fillId="0" borderId="0" xfId="0" applyFont="1"/>
    <xf numFmtId="0" fontId="3" fillId="0" borderId="0" xfId="0" applyFont="1"/>
    <xf numFmtId="0" fontId="4" fillId="0" borderId="0" xfId="0" applyFont="1"/>
    <xf numFmtId="0" fontId="5" fillId="0" borderId="0" xfId="0" applyFont="1"/>
    <xf numFmtId="0" fontId="6" fillId="0" borderId="0" xfId="0" applyFont="1"/>
  </cellXfs>
  <cellStyles count="1"><cellStyle name="Normal" xfId="0" builtinId="0"/></cellStyles>
</styleSheet>"""


def cell_inline(ref: str, text: str, s: int = 0) -> str:
    return f'<c r="{ref}" s="{s}" t="inlineStr"><is><t xml:space="preserve">{text}</t></is></c>'


def make_sheet() -> str:
    headers = [
        ("B2", "scheme=major"),
        ("C2", "scheme=minor"),
        ("D2", "family=1 Roman"),
        ("E2", "family=3 Modern"),
        ("F2", "family=4 Script"),
        ("G2", "family=5 Decorative"),
    ]
    samples = [
        ("B3", "AaBb 123", 1),
        ("C3", "AaBb 123", 2),
        ("D3", "AaBb 123", 3),
        ("E3", "AaBb 123", 4),
        ("F3", "AaBb 123", 5),
        ("G3", "AaBb 123", 6),
    ]
    row2 = "".join(cell_inline(r, t) for r, t in headers)
    row3 = "".join(cell_inline(r, t, s=s) for r, t, s in samples)
    rows_xml = (
        f'<row r="2" ht="22" customHeight="1">{row2}</row>'
        f'<row r="3" ht="40" customHeight="1">{row3}</row>'
    )
    return f"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
           xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <dimension ref="B2:G3"/>
  <sheetViews><sheetView workbookViewId="0"/></sheetViews>
  <sheetFormatPr defaultRowHeight="15"/>
  <cols><col min="2" max="7" width="22" customWidth="1"/></cols>
  <sheetData>{rows_xml}</sheetData>
</worksheet>"""


def rewrite(path: str) -> None:
    with zipfile.ZipFile(path, "r") as zin:
        names = zin.namelist()
        blobs = {n: zin.read(n) for n in names}

    blobs["xl/styles.xml"] = STYLES_XML.encode("utf-8")
    # Overwrite theme1.xml. hsx writes a theme part by default; we just
    # replace its body. (If it didn't exist we'd also have to patch
    # [Content_Types] / workbook.xml.rels — but hsx always emits one.)
    theme_path = next(
        (n for n in names if n.startswith("xl/theme/") and n.endswith(".xml")),
        None,
    )
    if theme_path is None:
        raise RuntimeError("no theme1.xml found in xlsx")
    blobs[theme_path] = THEME_XML.encode("utf-8")

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
