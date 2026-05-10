#!/usr/bin/env bash
# Build a feature-rich workbook to test fidelity. Uses hsx (SpreadJS).
set -euo pipefail
F=${1:-$(dirname "$0")/kitchensink.xlsx}
rm -f "$F"
hsx create "$F"

# ---- Sheet1: data + formulas + dynamic arrays ----
hsx set "$F" "Sheet1!A1:E1" '[
  [{"value":"Region","style":{"fontStyle":{"bold":true},"backColor":"#1F2937","foreColor":"#FFFFFF"}},
   {"value":"Q1","style":{"fontStyle":{"bold":true},"backColor":"#1F2937","foreColor":"#FFFFFF"}},
   {"value":"Q2","style":{"fontStyle":{"bold":true},"backColor":"#1F2937","foreColor":"#FFFFFF"}},
   {"value":"Q3","style":{"fontStyle":{"bold":true},"backColor":"#1F2937","foreColor":"#FFFFFF"}},
   {"value":"Q4","style":{"fontStyle":{"bold":true},"backColor":"#1F2937","foreColor":"#FFFFFF"}}]
]'
hsx set "$F" "Sheet1!A2:E5" '[
  [{"value":"North"},{"value":120},{"value":138},{"value":151},{"value":169}],
  [{"value":"South"},{"value":98},{"value":110},{"value":121},{"value":135}],
  [{"value":"East"}, {"value":145},{"value":158},{"value":172},{"value":189}],
  [{"value":"West"}, {"value":88}, {"value":94}, {"value":102},{"value":119}]
]'

# Currency formatting
hsx set "$F" "Sheet1!B2:E5" '[
  [{"value":120,"style":{"formatter":"$#,##0"}},{"value":138,"style":{"formatter":"$#,##0"}},{"value":151,"style":{"formatter":"$#,##0"}},{"value":169,"style":{"formatter":"$#,##0"}}],
  [{"value":98, "style":{"formatter":"$#,##0"}},{"value":110,"style":{"formatter":"$#,##0"}},{"value":121,"style":{"formatter":"$#,##0"}},{"value":135,"style":{"formatter":"$#,##0"}}],
  [{"value":145,"style":{"formatter":"$#,##0"}},{"value":158,"style":{"formatter":"$#,##0"}},{"value":172,"style":{"formatter":"$#,##0"}},{"value":189,"style":{"formatter":"$#,##0"}}],
  [{"value":88, "style":{"formatter":"$#,##0"}},{"value":94, "style":{"formatter":"$#,##0"}},{"value":102,"style":{"formatter":"$#,##0"}},{"value":119,"style":{"formatter":"$#,##0"}}]
]'

# Totals row using SUMPRODUCT (hostile to IronCalc)
hsx set "$F" "Sheet1!A7:E7" '[
  [{"value":"Total","style":{"fontStyle":{"bold":true}}},
   {"formula":"=SUM(B2:B5)","style":{"fontStyle":{"bold":true},"formatter":"$#,##0"}},
   {"formula":"=SUM(C2:C5)","style":{"fontStyle":{"bold":true},"formatter":"$#,##0"}},
   {"formula":"=SUM(D2:D5)","style":{"fontStyle":{"bold":true},"formatter":"$#,##0"}},
   {"formula":"=SUM(E2:E5)","style":{"fontStyle":{"bold":true},"formatter":"$#,##0"}}]
]'
hsx set "$F" "Sheet1!A8:B8" '[[{"value":"Weighted","style":{"fontStyle":{"italic":true}}},{"formula":"=SUMPRODUCT(B2:B5,C2:C5)/SUM(C2:C5)","style":{"formatter":"$#,##0.00"}}]]'

# LET formula (may fail on engines without LET)
hsx set "$F" "Sheet1!A9:B9" '[[{"value":"Growth (LET)","style":{"fontStyle":{"italic":true}}},{"formula":"=LET(a,SUM(B2:B5),b,SUM(E2:E5),b/a-1)","style":{"formatter":"0.0%"}}]]' || echo "  (LET unsupported — skipped)"

# Dynamic array: SORT + FILTER spill
hsx set "$F" "Sheet1!G1" '[[{"value":"Sorted by Q4 desc","style":{"fontStyle":{"bold":true}}}]]'
hsx set "$F" "Sheet1!G2" '[[{"formula":"=SORT(A2:E5,5,-1)"}]]'

hsx set "$F" "Sheet1!G8" '[[{"value":"FILTER Q4>120","style":{"fontStyle":{"bold":true}}}]]'
hsx set "$F" "Sheet1!G9" '[[{"formula":"=FILTER(A2:E5,E2:E5>120)"}]]'

hsx set "$F" "Sheet1!G15" '[[{"value":"UNIQUE","style":{"fontStyle":{"bold":true}}}]]'
hsx set "$F" "Sheet1!G16" '[[{"formula":"=UNIQUE(A2:A5)"}]]'

hsx set "$F" "Sheet1!I1" '[[{"value":"SEQUENCE","style":{"fontStyle":{"bold":true}}}]]'
hsx set "$F" "Sheet1!I2" '[[{"formula":"=SEQUENCE(4,1,10,5)"}]]'

# Merged range header
hsx eval "$F" 'workbook.getSheet(0).addSpan(10, 0, 1, 5); workbook.getSheet(0).setValue(10, 0, "FY24 Regional Performance"); workbook.getSheet(0).getCell(10,0).hAlign(1).fontStyle({bold:true,italic:false}).backColor("#FEF3C7");'

# Comment
hsx eval "$F" 'const s=workbook.getSheet(0); if (s.comments && typeof s.comments==="function") {s.comments().add(1,4,"Forecast — needs review");} else {const c=new GC.Spread.Sheets.Comments.Comment(); c.text("Forecast — needs review"); s.comments.add(1,4,c);} ' || echo "  (comment failed)"

# Data validation
hsx eval "$F" '
  const sht = workbook.getSheet(0);
  const dv = GC.Spread.Sheets.DataValidation.createListValidator("North,South,East,West");
  sht.setDataValidator(1,0,4,1,dv);
'

# Named range
hsx eval "$F" 'workbook.addCustomName("RegionList","=Sheet1!$A$2:$A$5",0,"")'

# Conditional formatting (color scale on B2:E5)
hsx eval "$F" '
  const sht = workbook.getSheet(0);
  const cs = sht.conditionalFormats;
  cs.add2ScaleRule(GC.Spread.Sheets.ConditionalFormatting.ScaleValueType.lowestValue,null,"#FECACA",
                   GC.Spread.Sheets.ConditionalFormatting.ScaleValueType.highestValue,null,"#86EFAC",
                   [new GC.Spread.Sheets.Range(1,1,4,4)]);
'

# Chart: column chart over A1:E5
hsx eval "$F" '
  const sht = workbook.getSheet(0);
  const c = sht.charts.add("chart1", GC.Spread.Sheets.Charts.ChartType.columnClustered, 50, 380, 480, 280, "A1:E5");
  c.title({text:"Quarterly Sales by Region"});
'

# Frozen pane + column widths
hsx eval "$F" 'const s=workbook.getSheet(0); s.frozenRowCount(1); s.frozenColumnCount(1); for(let c=0;c<5;c++) s.setColumnWidth(c, c===0?100:90);'

# ---- Sheet2: ListObject (table) ----
hsx sheet "$F" create "TableSheet"
hsx set "$F" "TableSheet!A1:C5" '[
  [{"value":"Item"},{"value":"Qty"},{"value":"Price"}],
  [{"value":"Apple"},{"value":10},{"value":1.25}],
  [{"value":"Pear"},{"value":7},{"value":2.10}],
  [{"value":"Plum"},{"value":15},{"value":0.85}],
  [{"value":"Kiwi"},{"value":4},{"value":3.40}]
]'
hsx eval "$F" '
  const s = workbook.getSheetFromName("TableSheet");
  s.tables.add("Inventory", 0,0,5,3,
    GC.Spread.Sheets.Tables.TableThemes.medium2);
'

echo "Built $F"
ls -la "$F"
