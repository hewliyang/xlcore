#!/usr/bin/env python3
"""Rewrite an empty xlsx (created by `hsx create`) into a fixture with:
  1. a Cyber color theme installed at xl/theme/theme1.xml
  2. xl/styles.xml carrying 12 solid fills, one per theme slot, each
     stored as `<fgColor theme="N"/>` (the path that exercises the
     theme-color resolution we're testing)
  3. xl/worksheets/sheet1.xml row 1 with 12 cells (A1..L1) each tagged
     with the matching xf -> the matching theme slot

Standalone: only depends on stdlib (zipfile, re, os, tempfile).

We rewrite styles.xml from scratch (the hsx-default skeleton has only
the minimum fonts/fills/borders) so the cellXfs ids we assign in
sheet1.xml are predictable.
"""
import sys, zipfile, re, os, tempfile

PATH = sys.argv[1]

# Cyber palette in spreadsheet (theme=N) order:
#   0:lt1, 1:dk1, 2:lt2, 3:dk2, 4..9:accent1..6, 10:hlink, 11:folHlink
SLOT_TO_HEX = [
    "F5FFF5", "0A0A0A", "E0F7FA", "1B3A4B",
    "00BFA5", "FF4081", "7C4DFF", "FFD740", "18FFFF", "76FF03",
    "0091EA", "AA00FF",
]
SLOT_LABELS = [
    "lt1", "dk1", "lt2", "dk2",
    "acc1", "acc2", "acc3", "acc4", "acc5", "acc6",
    "hlink", "folHlink",
]
# OOXML element order in <a:clrScheme> is dk1/lt1/dk2/lt2/accent1..6/hlink/folHlink:
# the first two pairs are swapped vs. the spreadsheet `theme="N"` indexing.
XML_ORDER = ["dk1", "lt1", "dk2", "lt2",
             "accent1", "accent2", "accent3", "accent4", "accent5", "accent6",
             "hlink", "folHlink"]
SPREADSHEET_TO_XML = [1, 0, 3, 2, 4, 5, 6, 7, 8, 9, 10, 11]


def make_clr_scheme() -> str:
    parts = ['<a:clrScheme name="Cyber">']
    for elem, slot in zip(XML_ORDER, SPREADSHEET_TO_XML):
        parts.append(f'<a:{elem}><a:srgbClr val="{SLOT_TO_HEX[slot]}"/></a:{elem}>')
    parts.append('</a:clrScheme>')
    return "".join(parts)


def patch_theme_xml(theme_xml: str) -> str:
    return re.sub(
        r'<a:clrScheme[^>]*>.*?</a:clrScheme>',
        make_clr_scheme(),
        theme_xml,
        count=1,
        flags=re.DOTALL,
    )


def make_styles_xml() -> str:
    """A minimal styleSheet with 2 fonts (default + bold-white-for-dark
    cells) and 14 fills (the 2 mandatory built-ins + 12 theme-ref ones).
    cellXfs[1..12] correspond to theme slot 0..11. cellXfs[0] is the
    default. Fonts: 0=default, 1=white-bold (for use on dark slots).
    """
    fills = ['<fill><patternFill patternType="none"/></fill>',
             '<fill><patternFill patternType="gray125"/></fill>']
    for slot in range(12):
        fills.append(
            f'<fill><patternFill patternType="solid">'
            f'<fgColor theme="{slot}"/><bgColor indexed="64"/>'
            f'</patternFill></fill>'
        )
    # Slots whose background is dark enough that black text would vanish.
    DARK_SLOTS = {1, 3, 4, 5, 6, 10, 11}
    cell_xfs = ['<xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/>']
    for slot in range(12):
        font_id = 1 if slot in DARK_SLOTS else 0
        # fillId is offset by 2 (the two builtin fills above).
        cell_xfs.append(
            f'<xf numFmtId="0" fontId="{font_id}" fillId="{slot + 2}" borderId="0"'
            f' xfId="0" applyFont="1" applyFill="1"/>'
        )
    return (
        '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
        '<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">'
        '<fonts count="2">'
        '<font><sz val="11"/><color theme="1"/><name val="Calibri"/></font>'
        '<font><b/><sz val="11"/><color rgb="FFFFFFFF"/><name val="Calibri"/></font>'
        '</fonts>'
        f'<fills count="{len(fills)}">{"".join(fills)}</fills>'
        '<borders count="1"><border><left/><right/><top/><bottom/><diagonal/></border></borders>'
        '<cellStyleXfs count="1">'
        '<xf numFmtId="0" fontId="0" fillId="0" borderId="0"/>'
        '</cellStyleXfs>'
        f'<cellXfs count="{len(cell_xfs)}">{"".join(cell_xfs)}</cellXfs>'
        '<cellStyles count="1">'
        '<cellStyle name="Normal" xfId="0" builtinId="0"/>'
        '</cellStyles>'
        '</styleSheet>'
    )


def make_sheet_xml() -> str:
    """A single row of 12 cells, A1..L1. Each cell has style=N+1 (matches
    the cellXfs entry for theme slot N) and an inline string label so the
    test reads the visible text without needing a sharedStrings part."""
    cells = []
    for i, label in enumerate(SLOT_LABELS):
        col = chr(ord("A") + i)
        cells.append(
            f'<c r="{col}1" s="{i + 1}" t="inlineStr">'
            f'<is><t>{i} {label}</t></is></c>'
        )
    cols = "".join(
        f'<col min="{i+1}" max="{i+1}" width="14" customWidth="1"/>'
        for i in range(12)
    )
    return (
        '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
        '<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">'
        '<dimension ref="A1:L1"/>'
        '<sheetViews><sheetView workbookViewId="0"/></sheetViews>'
        f'<cols>{cols}</cols>'
        f'<sheetData><row r="1">{"".join(cells)}</row></sheetData>'
        '</worksheet>'
    )


def main():
    fd, tmp_path = tempfile.mkstemp(
        suffix=".xlsx", dir=os.path.dirname(os.path.abspath(PATH))
    )
    os.close(fd)
    seen = set()
    with zipfile.ZipFile(PATH, "r") as zin, zipfile.ZipFile(
        tmp_path, "w", zipfile.ZIP_DEFLATED
    ) as zout:
        for item in zin.infolist():
            seen.add(item.filename)
            data = zin.read(item.filename)
            if item.filename == "xl/theme/theme1.xml":
                data = patch_theme_xml(data.decode("utf-8")).encode("utf-8")
            elif item.filename == "xl/styles.xml":
                data = make_styles_xml().encode("utf-8")
            elif item.filename.startswith("xl/worksheets/sheet") and item.filename.endswith(".xml"):
                data = make_sheet_xml().encode("utf-8")
            zout.writestr(item, data)
    os.replace(tmp_path, PATH)
    expected = {"xl/theme/theme1.xml", "xl/styles.xml"}
    missing = expected - seen
    if missing:
        sys.exit(f"  ERROR: xlsx missing parts {missing}")
    print(f"  patched theme + styles + sheet1 in {PATH}", file=sys.stderr)


if __name__ == "__main__":
    main()
