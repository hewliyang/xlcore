#!/usr/bin/env bash
# Builds a workbook exercising horizontal-alignment indent (1..5 units)
# for left-aligned and right-aligned text. SpreadJS / Excel both
# interpret each indent unit as roughly 3 character widths of the
# default font (~9px at 11pt Calibri). The renderer needs to bias text
# placement on the alignment-anchor side so labels visibly step inward.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
F="${1:-$HERE/indent.xlsx}"
rm -f "$F"
hsx create "$F" >/dev/null

hsx eval "$F" - <<'JS'
sheet.setColumnWidth(0, 220);
sheet.setColumnWidth(1, 220);

// Header.
sheet.getCell(0, 0).value("left + indent N");
sheet.getCell(0, 1).value("right + indent N");
sheet.getCell(0, 0).font("bold 11pt Calibri");
sheet.getCell(0, 1).font("bold 11pt Calibri");

for (let i = 0; i <= 5; i++) {
  const r = i + 1;
  // Left column: hAlign=left, indent=i.
  const lc = sheet.getCell(r, 0);
  lc.value(`indent=${i}`);
  lc.hAlign(GC.Spread.Sheets.HorizontalAlign.left);
  lc.textIndent(i);

  // Right column: hAlign=right, indent=i.
  const rc = sheet.getCell(r, 1);
  rc.value(`indent=${i}`);
  rc.hAlign(GC.Spread.Sheets.HorizontalAlign.right);
  rc.textIndent(i);
}
JS

echo "wrote $F"
