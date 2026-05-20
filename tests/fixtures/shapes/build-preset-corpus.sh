#!/usr/bin/env bash
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
F="${1:-$HERE/preset-corpus.xlsx}"
rm -f "$F"
hsx create "$F" >/dev/null

hsx eval "$F" - <<'JS'
const sht = workbook.getSheet(0);
const T = GC.Spread.Sheets.Shapes.AutoShapeType;

const COLS = 8;
const W = 80, H = 60, GUT_X = 16, GUT_Y = 20;
const X0 = 12, Y0 = 14;

for (let c = 0; c < 18; c++) sht.setColumnWidth(c, 64);
for (let r = 0; r < 40; r++) sht.setRowHeight(r, 18);

const grid = [
  [T.flowchartProcess,            "fcProc"],
  [T.flowchartAlternateProcess,   "fcAltP"],
  [T.flowchartDecision,           "fcDec"],
  [T.flowchartData,               "fcData"],
  [T.flowchartPredefinedProcess,  "fcPre"],
  [T.flowchartInternalStorage,    "fcIS"],
  [T.flowchartDocument,           "fcDoc"],
  [T.flowchartMultidocument,      "fcMul"],
  [T.flowchartTerminator,         "fcTerm"],
  [T.flowchartPreparation,        "fcPrep"],
  [T.flowchartManualInput,        "fcMIn"],
  [T.flowchartManualOperation,    "fcMOp"],
  [T.flowchartConnector,          "fcConn"],
  [T.flowchartOffpageConnector,   "fcOff"],
  [T.flowchartPunchedTape,        "fcTape"],
  [T.flowchartSummingJunction,    "fcSum"],
  [T.curvedRightArrow,            "cvRgt"],
  [T.curvedUpArrow,               "cvUp"],
  [T.bentArrow,                   "bent"],
  [T.bentUpArrow,                 "bentUp"],
  [T.uTurnArrow,                  "uturn"],
  [T.stripedRightArrow,           "strRA"],
  [T.notchedRightArrow,           "ntcRA"],
  [T.quadArrow,                   "quad"],
  [T.swooshArrow,                 "swoosh"],
  [T.circularArrow,               "circle→"],
  [T.mathPlus,                    "+"],
  [T.mathMinus,                   "−"],
  [T.mathMultiply,                "×"],
  [T.mathDivide,                  "÷"],
  [T.mathEqual,                   "="],
  [T.mathNotEqual,                "≠"],
  [T.cloud,                       "cloud"],
  [T.heart,                       "♥"],
  [T.lightningBolt,               "lit"],
  [T.smileyFace,                  "smile"],
  [T.sun,                         "sun"],
  [T.moon,                        "moon"],
  [T.donut,                       "donut"],
  [T.blockArc,                    "blkArc"],
  [T.plaque,                      "plaque"],
  [T.bevel,                       "bevel"],
  [T.can,                         "can"],
  [T.cube,                        "cube"],
  [T.foldedCorner,                "fold"],
  [T.frame,                       "frame"],
  [T.halfFrame,                   "halfF"],
  [T.parallelogram,               "parall"],
  [T.trapezoid,                   "trap"],
  [T.tear,                        "tear"],
  [T.pie,                         "pie"],
  [T.chord,                       "chord"],
  [T.arc,                         "arc"],
  [T.noSymbol,                    "noSym"],
  [T.diagonalStripe,              "diag"],
  [T.shape4pointStar,             "★4"],
  [T.star6Point,                  "★6"],
  [T.star7Point,                  "★7"],
  [T.shape8pointStar,             "★8"],
  [T.star10Point,                 "★10"],
  [T.shape16pointStar,            "★16"],
  [T.upRibbon,                    "ribUp"],
  [T.downRibbon,                  "ribDn"],
  [T.curvedUpRibbon,              "ribCvUp"],
  [T.wave,                        "wave"],
  [T.doubleWave,                  "dblWave"],
  [T.horizontalScroll,            "hScroll"],
  [T.verticalScroll,              "vScroll"],
  [T.leftRightRibbon,             "lrRib"],
  [T.rectangularCallout,          "callRct"],
  [T.roundedRectangularCallout,   "callRR"],
  [T.ovalCallout,                 "callOv"],
  [T.cloudCallout,                "callCld"],
  [T.actionButtonHome,            "🏠"],
  [T.actionButtonHelp,            "?"],
  [T.actionButtonInformation,     "i"],
  [T.actionButtonBackorPrevious,  "◀"],
  [T.actionButtonForwardorNext,   "▶"],
  [T.actionButtonBeginning,       "⏮"],
  [T.actionButtonEnd,             "⏭"],
  [T.actionButtonReturn,          "↩"],
  [T.chartPlus,                   "ch+"],
  [T.chartX,                      "chX"],
  [T.chartStar,                   "ch★"],
  [T.round1Rectangle,             "rnd1"],
  [T.round2SameRectangle,         "rnd2s"],
  [T.round2DiagRectangle,         "rnd2d"],
  [T.snipRoundRectangle,          "snipR"],
  [T.snip1Rectangle,              "snip1"],
  [T.snip2SameRectangle,          "snip2s"],
  [T.snip2DiagRectangle,          "snip2d"],
  [T.decagon,                     "10gon"],
  [T.dodecagon,                   "12gon"],
  [T.heptagon,                    "7gon"],
  [T.corner,                      "corner"],
  [T.cornerTabs,                  "cTabs"],
  [T.cross,                       "cross"],
  [T.gear6,                       "gear6"],
  [T.gear9,                       "gear9"],
  [T.funnel,                      "funnel"],
  [T.pieWedge,                    "pieW"],
];

for (let i = 0; i < grid.length; i++) {
  const [type, label] = grid[i];
  const col = i % COLS, row = Math.floor(i / COLS);
  const x = X0 + col * (W + GUT_X);
  const y = Y0 + row * (H + GUT_Y);
  const s = sht.shapes.add("p" + i, type, x, y, W, H);
  if (label) s.text(label);
}
JS

echo "wrote $F"
