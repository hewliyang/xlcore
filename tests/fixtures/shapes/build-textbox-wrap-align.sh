#!/usr/bin/env bash
# Fixture: textbox-style rectangles exercising DrawingML `txBody`
# wrap / alignment / vertical-anchor / inset variations.
#
# Targets the P0 items in `docs/parity-shapes.md`:
#   - body insets `bodyPr@lIns/tIns/rIns/bIns` (today renderer
#     uses a hardcoded padding → text overflows narrow shapes),
#   - paragraph horizontal alignment `pPr@algn=l/ctr/r`,
#   - body vertical anchor `bodyPr@anchor=t/ctr/b`,
#   - word-wrap `bodyPr@wrap=square` (default) vs `wrap=none`.
#
# Layout: a 4×4 grid of identically-sized rectangles, each with a
# different combination so deltas isolate cleanly when one of those
# attributes regresses.
#
# SpreadJS won't emit explicit insets, `wrap="none"`, or `algn="just"`
# through its public API — so after `hsx` lays the workbook down
# we run `_patch_textbox.py` to splice those attrs directly into
# `xl/drawings/drawing1.xml`. Without that step the inset / wrap-off /
# justify rows would be visually identical to their defaults.
#
#   row 0 — h-align:    left / center / right / justify
#   row 1 — v-anchor:   top / ctr / bottom (+ a narrow "wrap" probe)
#   row 2 — wrap mode:  wrap-on (square) / wrap-off (none) pair × 2
#   row 3 — inset:      default / tight / loose / asymmetric
#
# All shapes carry a multi-line label so wrap behaviour is visible.
# Shapes are sized so the long label needs to wrap (or visibly
# overflow when wrap=none).
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
F="${1:-$HERE/textbox-wrap-align.xlsx}"
rm -f "$F"
hsx create "$F" >/dev/null

hsx eval "$F" - <<'JS'
const sht = workbook.getSheet(0);
const T  = GC.Spread.Sheets.Shapes.AutoShapeType;
const HA = GC.Spread.Sheets.Shapes.HorizontalAlign;
const VA = GC.Spread.Sheets.Shapes.VerticalAlign;
const WT = GC.Spread.Sheets.Shapes.ST_TextWrappingType;

for (let c = 0; c < 16; c++) sht.setColumnWidth(c, 80);
for (let r = 0; r < 20; r++) sht.setRowHeight(r, 22);

const W = 140, H = 80, GX = 18, GY = 22;
const X0 = 14, Y0 = 14;

const LONG = "Lorem ipsum dolor sit amet consectetur.";

function place(name, col, row, label, mutate) {
  const x = X0 + col * (W + GX);
  const y = Y0 + row * (H + GY);
  const s = sht.shapes.add(name, T.rectangle, x, y, W, H);
  s.text(label);
  if (mutate) mutate(s);
  return s;
}

// Row 0: horizontal alignment.
place("hL", 0, 0, "left\n" + LONG,    s => s.style({textFrame:{hAlign: HA.left}}));
place("hC", 1, 0, "center\n" + LONG,  s => s.style({textFrame:{hAlign: HA.center}}));
place("hR", 2, 0, "right\n" + LONG,   s => s.style({textFrame:{hAlign: HA.right}}));
// SpreadJS exposes a "justify" hAlign on textFrame via the enum.
place("hJ", 3, 0, "justify\n" + LONG, s => s.style({textFrame:{hAlign: HA.justify}}));

// Row 1: vertical anchor.
place("vT", 0, 1, "top",    s => s.style({textFrame:{vAlign: VA.top}}));
place("vC", 1, 1, "center", s => s.style({textFrame:{vAlign: VA.middle}}));
place("vB", 2, 1, "bottom", s => s.style({textFrame:{vAlign: VA.bottom}}));
// Narrow shape forces wrap regardless of mode (probes wrap default).
{
  const x = X0 + 3 * (W + GX);
  const y = Y0 + 1 * (H + GY);
  const s = sht.shapes.add("narrow", T.rectangle, x, y, 70, H);
  s.text("narrow wrap " + LONG);
}

// Row 2: wrap on/off pair (label is long; height clipped so any
// wrap=none case overflows past the shape rect).
{
  const wrapOn = (s) => { try { s.textFrame.wordWrap(true); } catch(_){} };
  const wrapOff = (s) => { try { s.textFrame.wordWrap(false); } catch(_){} };
  place("wOn1",  0, 2, "wrap on\n"  + LONG, wrapOn);
  place("wOff1", 1, 2, "wrap off " + LONG, wrapOff);
  place("wOn2",  2, 2, "wrap on\n"  + LONG, wrapOn);
  place("wOff2", 3, 2, "wrap off " + LONG, wrapOff);
}

// Row 3: insets via the runtime `textFrame.padding(left,top,right,bottom)`.
function pad(s, l, t, r, b) {
  try {
    if (typeof s.textFrame.padding === "function") {
      s.textFrame.padding(l, t, r, b);
    }
  } catch(_) {}
}
place("insDefault", 0, 3, "default insets\n" + LONG,                        s => {});
place("insTight",   1, 3, "tight insets\n"   + LONG, s => pad(s, 1,  1,  1,  1));
place("insLoose",   2, 3, "loose insets\n"   + LONG, s => pad(s, 18, 18, 18, 18));
place("insAsym",    3, 3, "asym L/R\n"       + LONG, s => pad(s, 24, 4,  24, 4));
JS

# hsx eval can return before the xlsx is fully flushed (per
# docs/TESTING.md “Why some builders patch the zip directly”).
# Force the daemon to drain pending writes before we read the file.
hsx daemon flush >/dev/null 2>&1 || true
python3 "$HERE/_patch_textbox.py" "$F"
echo "wrote $F"
