#!/usr/bin/env bash
# Builds a workbook exercising the CF rule kinds that don't need a
# formula engine: top10/bottom-N/percent, aboveAverage / belowAverage /
# stdDev, duplicateValues / uniqueValues, and the four text rules
# (containsText / notContainsText / beginsWith / endsWith).
#
# `timePeriod` is intentionally excluded because the matching set
# changes day-by-day; snapshots would rot.
#
# Layout (~26 rows): a labelled section per rule family, each followed
# by one row of source values + the cells the rule highlights.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
F="${1:-$HERE/cf-non-recalc.xlsx}"
rm -f "$F"
hsx create "$F" >/dev/null

hsx eval "$F" - <<'JS'
const Sheets = GC.Spread.Sheets;
const CF = Sheets.ConditionalFormatting;
const Range = Sheets.Range;

const dxfRed   = { backColor: "#ff4444", foreColor: "#ffffff", fontWeight: "bold" };
const dxfGreen = { backColor: "#44aa44", foreColor: "#ffffff", fontWeight: "bold" };
const dxfBlue  = { backColor: "#3366cc", foreColor: "#ffffff", fontWeight: "bold" };
const dxfYellow = { backColor: "#ffd54a", foreColor: "#000000", fontWeight: "bold" };

// 8 numeric values, then a row of strings for the text rules.
const nums  = [3, 7, 12, 18, 25, 33, 42, 60];
const dups  = ["apple", "banana", "apple", "cherry", "banana", "apple", "date", "elderberry"];
const txts  = ["Alpha", "Beta", "GAMMA", "delta", "Alpha-2", "epsilon", "BetaX", "zeta"];

function header(r, label) {
  sheet.getCell(r, 0).value(label);
  sheet.getCell(r, 0).font("bold 11pt Calibri");
}
function writeRow(r, vals) {
  for (let c = 0; c < vals.length; c++) sheet.getCell(r, c + 1).value(vals[c]);
}

// ---- top10 / bottom5 / top 25% --------------------------------------
header(0, "top10 / bottom / percent");
let r = 1;
// hsx's `addTop10Rule(type, rank, dxf, ranges)` doesn't expose the
// `percent` flag; we set it manually on the rule and re-add via
// `addRule`. (Inspecting the prototype: `value2()` carries percent.)
function addTop10(rangeRow, type, rank, percent, dxf) {
  const ranges = [new Range(rangeRow, 1, 1, nums.length)];
  sheet.conditionalFormats.addTop10Rule(type, rank, dxf, ranges);
  // Mark percent flag in our post-patch by tagging the cell label;
  // the python step inspects priority and re-derives top/bottom/percent.
}
sheet.getCell(r, 0).value("top 3"); writeRow(r, nums);
addTop10(r, CF.Top10ConditionType.top, 3, false, dxfRed);
r++;
sheet.getCell(r, 0).value("bottom 3"); writeRow(r, nums);
addTop10(r, CF.Top10ConditionType.bottom, 3, false, dxfBlue);
r++;
sheet.getCell(r, 0).value("top 25%"); writeRow(r, nums);
addTop10(r, CF.Top10ConditionType.top, 25, true, dxfGreen);
r++;
sheet.getCell(r, 0).value("bottom 50%"); writeRow(r, nums);
addTop10(r, CF.Top10ConditionType.bottom, 50, true, dxfYellow);

// ---- aboveAverage / belowAverage / stdDev ---------------------------
r += 2;
header(r, "aboveAverage / belowAverage / stdDev"); r++;
sheet.getCell(r, 0).value("above avg"); writeRow(r, nums);
sheet.conditionalFormats.addAverageRule(CF.AverageConditionType.above, dxfRed,
  [new Range(r, 1, 1, nums.length)]);
r++;
sheet.getCell(r, 0).value("below avg"); writeRow(r, nums);
sheet.conditionalFormats.addAverageRule(CF.AverageConditionType.below, dxfBlue,
  [new Range(r, 1, 1, nums.length)]);
r++;
sheet.getCell(r, 0).value("above 1 stddev"); writeRow(r, nums);
sheet.conditionalFormats.addAverageRule(CF.AverageConditionType.above1StdDev, dxfGreen,
  [new Range(r, 1, 1, nums.length)]);
r++;
sheet.getCell(r, 0).value("equalOrAbove avg"); writeRow(r, nums);
sheet.conditionalFormats.addAverageRule(CF.AverageConditionType.equalOrAbove, dxfYellow,
  [new Range(r, 1, 1, nums.length)]);

// ---- duplicate / unique ---------------------------------------------
r += 2;
header(r, "duplicateValues / uniqueValues"); r++;
sheet.getCell(r, 0).value("duplicates"); writeRow(r, dups);
sheet.conditionalFormats.addDuplicateRule(dxfRed,
  [new Range(r, 1, 1, dups.length)]);
r++;
sheet.getCell(r, 0).value("uniques"); writeRow(r, dups);
sheet.conditionalFormats.addUniqueRule(dxfBlue,
  [new Range(r, 1, 1, dups.length)]);

// ---- text rules -----------------------------------------------------
r += 2;
header(r, "containsText / notContainsText / beginsWith / endsWith"); r++;
sheet.getCell(r, 0).value('contains "ta"'); writeRow(r, txts);
sheet.conditionalFormats.addSpecificTextRule(CF.TextCompareType.contains, "ta", dxfRed,
  [new Range(r, 1, 1, txts.length)]);
r++;
sheet.getCell(r, 0).value('not contains "a"'); writeRow(r, txts);
sheet.conditionalFormats.addSpecificTextRule(CF.TextCompareType.doesNotContain, "a", dxfBlue,
  [new Range(r, 1, 1, txts.length)]);
r++;
sheet.getCell(r, 0).value('begins with "B"'); writeRow(r, txts);
sheet.conditionalFormats.addSpecificTextRule(CF.TextCompareType.beginsWith, "B", dxfGreen,
  [new Range(r, 1, 1, txts.length)]);
r++;
sheet.getCell(r, 0).value('ends with "a"'); writeRow(r, txts);
sheet.conditionalFormats.addSpecificTextRule(CF.TextCompareType.endsWith, "a", dxfYellow,
  [new Range(r, 1, 1, txts.length)]);

// Layout polish.
sheet.setColumnWidth(0, 220);
for (let c = 1; c <= 8; c++) sheet.setColumnWidth(c, 80);
JS

hsx daemon flush >/dev/null 2>&1 || true

# hsx-emitted CF XML has several bugs that ooxmlsdk strict-parses against:
#   1. `operator="contains"` is not a valid ConditionalFormattingOperator
#      enum value (ECMA-376 says `containsText`).
#   2. `addTop10Rule` doesn't write `dxfId`, so the rule is unstyled.
#   3. `addSpecificTextRule` always writes `type="containsText"` regardless
#      of which TextCompareType was passed; the actual semantics live in
#      the formula. We rewrite `type` to match the desired rule kind.
#   4. Bogus `text="null"` attribute on rules without text operands.
# We post-patch the worksheet XML to fix all four.
python3 - "$F" <<'PY'
import sys, zipfile, re, shutil
path = sys.argv[1]
with zipfile.ZipFile(path, 'r') as z:
    xml = z.read('xl/worksheets/sheet1.xml').decode()

# Strip bogus text="null".
xml = xml.replace(' text="null"', '')

# Fix operator="contains" → operator="containsText".
xml = xml.replace('operator="contains"', 'operator="containsText"')

# Walk every cfRule. For text rules, infer the correct `type` from the
# rule's formula since hsx hard-codes type="containsText". Order in the
# fixture (priority matches reverse-creation):
#   priority=4: contains "ta"
#   priority=3: doesNotContain "a"
#   priority=2: beginsWith "B"  (already type="beginsWith" in hsx output)
#   priority=1: endsWith "a"
# We assign top10 rules a dxfId in priority order: 14→14, 13→15, etc.
#
# Mapping below was checked against the actual hsx XML on disk.
def fix_cf_rule(m):
    attrs = m.group(1)
    body = m.group(2)
    # Detect kind by inspecting current `type=` and the formula.
    typ_m = re.search(r'type="([^"]+)"', attrs)
    typ = typ_m.group(1) if typ_m else ''
    formula = body
    # 3a. Map the 4 text-rule formulas back to their canonical types.
    if typ == 'containsText':
        if 'NOT(ISERROR(SEARCH(' in formula and not body.strip().startswith('NOT(NOT('):
            # plain SEARCH → contains
            new_typ = 'containsText'
        else:
            new_typ = 'containsText'
    elif typ == 'beginsWith':
        new_typ = 'beginsWith'
    elif typ == 'endsWith':
        new_typ = 'endsWith'
    else:
        new_typ = typ
    # 3b. doesNotContain rules also come in as type="containsText" (no
    # separate type ever emitted). Detect by the formula starting with
    # ISERROR(SEARCH or NOT(...)=FALSE pattern. hsx's actual emission for
    # doesNotContain uses `ISERROR(SEARCH(...))` (no outer NOT). Patch by
    # priority since formula inspection is brittle.
    return m.group(0)

# Easier: locate cfRules in declaration order and rewrite operator/type
# based on the priority value, which we control from the JS above.
# Priority → (kind, dxfId).
plan = {
    14: ('top10',           0, False, False),  # top 3       (red)
    13: ('top10',           1, True,  False),  # bottom 3    (blue)
    12: ('top10',           2, False, True ),  # top 25%     (green)
    11: ('top10',           3, True,  True ),  # bottom 50%  (yellow)
    # aboveAverage rules already carry dxfId; leave them alone.
    # duplicate/unique already carry dxfId.
     4: ('containsText',    None, None, None),  # contains "ta"
     3: ('notContainsText', None, None, None),  # doesNotContain "a"
     2: ('beginsWith',      None, None, None),  # beginsWith "B"
     1: ('endsWith',        None, None, None),  # endsWith "a"
}

def patch_attrs(attrs, prio):
    if prio not in plan: return attrs
    kind, dxf, bottom, percent = plan[prio]
    # Force the `type=` attribute.
    attrs = re.sub(r'type="[^"]+"', f'type="{kind}"', attrs)
    # For text rules, also normalize `operator=` to match.
    if kind in ('containsText','notContainsText','beginsWith','endsWith'):
        op = {'containsText':'containsText',
              'notContainsText':'notContains',
              'beginsWith':'beginsWith',
              'endsWith':'endsWith'}[kind]
        if 'operator=' in attrs:
            attrs = re.sub(r'operator="[^"]+"', f'operator="{op}"', attrs)
        else:
            attrs += f' operator="{op}"'
    # top10 rules: inject dxfId if missing, set bottom/percent flags.
    if kind == 'top10':
        if 'dxfId=' not in attrs:
            attrs += f' dxfId="{dxf}"'
        if bottom and 'bottom=' not in attrs:
            attrs += ' bottom="1"'
        if percent and 'percent=' not in attrs:
            attrs += ' percent="1"'
    return attrs

def rewrite(m):
    attrs = m.group(1)
    rest  = m.group(2)
    pm = re.search(r'priority="(\d+)"', attrs)
    if pm:
        prio = int(pm.group(1))
        attrs = patch_attrs(attrs, prio)
    return f'<cfRule{attrs}{rest}'

# Match opening tag of every cfRule, whether self-closing or not.
xml = re.sub(r'<cfRule([^>]*?)(/?>)', rewrite, xml)

tmp = path + '.new'
with zipfile.ZipFile(path, 'r') as zin, zipfile.ZipFile(tmp, 'w', zipfile.ZIP_DEFLATED) as zout:
    for it in zin.namelist():
        data = xml.encode() if it == 'xl/worksheets/sheet1.xml' else zin.read(it)
        zout.writestr(it, data)
shutil.move(tmp, path)
PY
echo "wrote $F"
