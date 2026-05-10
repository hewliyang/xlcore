#!/usr/bin/env bash
# Builds a workbook exercising the dataBar CF kind across the variants
# our renderer should handle in v0:
#
#   row 1: solid bar, automatic min/max          (label "auto")
#   row 2: solid bar, num/num min=0 max=100      (label "0..100")
#   row 3: solid bar, mixed +/- values           (label "neg+pos")
#   row 4: bar with showBarOnly=true             (label "barOnly")
#   row 5: gradient (Excel 2010+ default)        (label "gradient")
#
# The fixture is a pixel-diff target against `hsx`. Open known
# divergences (negative bar coloring, axis positioning, exact
# gradient stops) live in tests/fixtures/cf/TRIAGE.md once the
# renderer ships.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
F="${1:-$HERE/data-bar.xlsx}"
rm -f "$F"
hsx create "$F" >/dev/null

hsx eval "$F" - <<'JS'
const Sheets = GC.Spread.Sheets;
const Cf = Sheets.ConditionalFormatting;
const SVT = Cf.ScaleValueType;
const Range = Sheets.Range;

// Header row.
const headers = ["label", "v1", "v2", "v3", "v4", "v5", "v6"];
for (let c = 0; c < headers.length; c++) {
  sheet.getCell(0, c).value(headers[c]);
  sheet.getCell(0, c).font("bold 11pt Calibri");
}

const rows = [
  { label: "auto",     values: [10, 25, 50, 75, 100, 5],
    rule: () => new Cf.DataBarRule(SVT.lowestValue, null, SVT.highestValue, null, null, "#638EC6") },
  { label: "0..100",   values: [10, 25, 50, 75, 100, 5],
    rule: () => new Cf.DataBarRule(SVT.number, 0, SVT.number, 100, null, "#7AC36A") },
  { label: "neg+pos",  values: [-50, -10, 0, 25, 60, 100],
    rule: () => new Cf.DataBarRule(SVT.lowestValue, null, SVT.highestValue, null, null, "#638EC6") },
  { label: "barOnly",  values: [10, 25, 50, 75, 100, 5],
    rule: () => { const r = new Cf.DataBarRule(SVT.lowestValue, null, SVT.highestValue, null, null, "#F47B5C");
                  r.showBarOnly(true); return r; } },
  { label: "gradient", values: [10, 25, 50, 75, 100, 5],
    rule: () => { const r = new Cf.DataBarRule(SVT.lowestValue, null, SVT.highestValue, null, null, "#A0A0A0");
                  r.gradient(true); return r; } },
];

for (let i = 0; i < rows.length; i++) {
  const r = i + 1;
  sheet.getCell(r, 0).value(rows[i].label);
  for (let c = 0; c < rows[i].values.length; c++) {
    sheet.getCell(r, c + 1).value(rows[i].values[c]);
  }
  const rule = rows[i].rule();
  rule.ranges([new Range(r, 1, 1, rows[i].values.length)]);
  sheet.conditionalFormats.addRule(rule);
}

sheet.setColumnWidth(0, 100);
for (let c = 1; c <= 6; c++) sheet.setColumnWidth(c, 80);
JS

hsx daemon flush >/dev/null 2>&1 || true

# hsx omits the required <color> child inside legacy <dataBar> blocks
# (the canonical color lives in the x14 extension only). ooxmlsdk
# refuses to parse a DataBar without its <color>, so we post-process
# the file to inject Excel's default blue. See tests/fixtures/cf/TRIAGE.md.
python3 - "$F" <<'PY'
import sys, zipfile, re, shutil
path = sys.argv[1]
with zipfile.ZipFile(path, 'r') as z:
    xml = z.read('xl/worksheets/sheet1.xml').decode()

def inject(m):
    attrs, inner = m.group(1), m.group(2)
    if '<color' in inner:
        return m.group(0)
    return f'<dataBar{attrs}>{inner}<color rgb="FF638EC6"/></dataBar>'

# Match only legacy x: namespace dataBar (no x14: prefix).
patched = re.sub(
    r'<dataBar([^>]*)>((?:(?!</dataBar>).)*)</dataBar>',
    inject, xml, flags=re.S)
tmp = path + '.new'
with zipfile.ZipFile(path, 'r') as zin, zipfile.ZipFile(tmp, 'w', zipfile.ZIP_DEFLATED) as zout:
    for it in zin.namelist():
        data = patched.encode() if it == 'xl/worksheets/sheet1.xml' else zin.read(it)
        zout.writestr(it, data)
shutil.move(tmp, path)
PY
echo "wrote $F"
