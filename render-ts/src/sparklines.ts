// Sparkline painter. One mini-chart per anchored cell; layout/colors
// come from the parent SparklineGroup. Three OOXML types: line / column
// / stacked (win-loss). All three respect the group's marker toggles
// (high/low/first/last/negative + plain markers for line) and color
// overrides.
//
// We render strictly inside `cellRect(r,c)` with a 2px margin on all
// sides so the chart doesn't kiss the cell border. Tiny cells (< ~20px
// in either axis) just suppress drawing — Excel itself bails at that
// scale.

import type { Sheet, SparklineGroup, Sparkline } from "./types.js";

import type { Grid } from "./grid.js";
import { cellRect } from "./geometry.js";
import type { CellRect } from "./geometry.js";
import type { Visible } from "./renderTypes.js";

const PAD = 2;
const MIN_CELL_W = 14;
const MIN_CELL_H = 10;

// Defaults that match Excel's "out of the box" sparkline look on a
// freshly-inserted sparkline (single accent series, no markers, no
// negative-color override).
const DEFAULT_SERIES = "#376092"; // Excel's default sparkline blue
const DEFAULT_NEGATIVE = "#FF0000";
const DEFAULT_AXIS = "#000000";
const DEFAULT_MARKERS = "#D00000";
const DEFAULT_HIGH = "#00B050";
const DEFAULT_LOW = "#FF0000";
const DEFAULT_FIRST = "#92D050";
const DEFAULT_LAST = "#92D050";

export function drawSparklines(
  ctx: CanvasRenderingContext2D,
  sheet: Sheet,
  g: Grid,
  vis: Visible,
): void {
  const groups = sheet.sparklineGroups;
  if (!groups || groups.length === 0) return;

  for (const group of groups) {
    for (const sp of group.sparklines) {
      // Cull off-screen (cheap reject).
      if (sp.r < vis.firstRow || sp.r > vis.lastRow) continue;
      if (sp.c < vis.firstCol || sp.c > vis.lastCol) continue;
      const rect = cellRect(g, sp.r, sp.c);
      if (rect.w < MIN_CELL_W || rect.h < MIN_CELL_H) continue;
      // Header strips never host sparklines, but defend anyway.
      if (rect.y < g.originY || rect.x < g.originX) continue;

      const inner: CellRect = {
        x: rect.x + PAD,
        y: rect.y + PAD,
        w: Math.max(1, rect.w - 2 * PAD),
        h: Math.max(1, rect.h - 2 * PAD),
      };

      ctx.save();
      // Clip strictly to the cell rect so a long line/column never
      // bleeds into the next cell.
      ctx.beginPath();
      ctx.rect(rect.x, rect.y, rect.w, rect.h);
      ctx.clip();

      switch (group.sparkType) {
        case "column":
          drawColumnSparkline(ctx, group, sp, inner);
          break;
        case "stacked":
          drawWinLossSparkline(ctx, group, sp, inner);
          break;
        case "line":
        default:
          drawLineSparkline(ctx, group, sp, inner);
          break;
      }

      ctx.restore();
    }
  }
}

// ---------------------------------------------------------------------
// Axis resolution

interface AxisRange {
  min: number;
  max: number;
}

function resolveRange(group: SparklineGroup, values: ReadonlyArray<number | null>): AxisRange {
  const present = values.filter((v): v is number => v != null);
  // Per-cell defaults from the values themselves.
  let lo = present.length ? Math.min(...present) : 0;
  let hi = present.length ? Math.max(...present) : 1;

  switch (group.minAxisType) {
    case "group":
      if (group.groupMin != null) lo = group.groupMin;
      break;
    case "custom":
      if (group.manualMin != null) lo = group.manualMin;
      break;
    // "individual" (default): use per-cell min already computed.
  }
  switch (group.maxAxisType) {
    case "group":
      if (group.groupMax != null) hi = group.groupMax;
      break;
    case "custom":
      if (group.manualMax != null) hi = group.manualMax;
      break;
  }
  // If the axis crosses zero, Excel auto-extends so 0 is included.
  // (Otherwise the negative-color column trick wouldn't have an axis
  // to flip across.) Only do this when the range came from data, not
  // when the user pinned both ends.
  if (lo > 0 && group.minAxisType === "individual") lo = 0;
  if (hi < 0 && group.maxAxisType === "individual") hi = 0;
  // Degenerate range (all same value): expand by 1 so the line shows.
  if (hi - lo < 1e-12) {
    hi = lo + 0.5;
    lo = lo - 0.5;
  }
  return { min: lo, max: hi };
}

// ---------------------------------------------------------------------
// Line sparklines

function drawLineSparkline(
  ctx: CanvasRenderingContext2D,
  group: SparklineGroup,
  sp: Sparkline,
  rect: CellRect,
): void {
  const values = sp.values ?? [];
  if (values.length === 0) return;

  // Build x/y points; honor displayEmptyCellsAs.
  const range = resolveRange(group, values);
  const yOf = (v: number) => {
    const t = (v - range.min) / (range.max - range.min);
    // Note: y axis flipped (canvas y grows downward).
    return rect.y + (1 - t) * rect.h;
  };
  const xOf = (i: number) => {
    if (values.length === 1) return rect.x + rect.w / 2;
    return rect.x + (i / (values.length - 1)) * rect.w;
  };

  const seriesColor = group.colorSeries ? `#${group.colorSeries}` : DEFAULT_SERIES;
  ctx.lineWidth = Math.max(0.5, group.lineWeight ?? 0.75);
  ctx.strokeStyle = seriesColor;

  // Stroke segments. "gap" => break the line at nulls; "zero" =>
  // draw at zero; "span" => skip the null and connect across it.
  const empty = group.displayEmptyCellsAs || "gap";
  ctx.beginPath();
  let prevDrawn = false;
  for (let i = 0; i < values.length; i++) {
    const v = values[i];
    if (v == null) {
      if (empty === "zero") {
        const x = xOf(i);
        const y = yOf(0);
        if (prevDrawn) ctx.lineTo(x, y);
        else ctx.moveTo(x, y);
        prevDrawn = true;
      } else if (empty === "gap") {
        prevDrawn = false;
      }
      // "span": just skip; next non-null lineTo() bridges across.
      continue;
    }
    const x = xOf(i);
    const y = yOf(v);
    if (prevDrawn) ctx.lineTo(x, y);
    else ctx.moveTo(x, y);
    prevDrawn = true;
  }
  ctx.stroke();

  // Optional axis line at zero, only when the data crosses zero.
  if (group.displayXAxis && range.min < 0 && range.max > 0) {
    const y = yOf(0);
    ctx.strokeStyle = group.colorAxis ? `#${group.colorAxis}` : DEFAULT_AXIS;
    ctx.lineWidth = 0.5;
    ctx.beginPath();
    ctx.moveTo(rect.x, y);
    ctx.lineTo(rect.x + rect.w, y);
    ctx.stroke();
  }

  // Markers. The OOXML spec lets six marker categories light up
  // independently: plain markers (every point), high, low, first,
  // last, negative. Plain markers paint first so the special-case
  // colored markers overpaint cleanly.
  const markerR = Math.max(1.25, Math.min(rect.w, rect.h) * 0.08);
  if (group.markers) {
    paintLineMarkers(ctx, values, xOf, yOf, markerR, group.colorMarkers ? `#${group.colorMarkers}` : DEFAULT_MARKERS);
  }
  paintExtremaMarkers(ctx, group, values, xOf, yOf, markerR);
}

function paintLineMarkers(
  ctx: CanvasRenderingContext2D,
  values: ReadonlyArray<number | null>,
  xOf: (i: number) => number,
  yOf: (v: number) => number,
  r: number,
  color: string,
): void {
  ctx.fillStyle = color;
  for (let i = 0; i < values.length; i++) {
    const v = values[i];
    if (v == null) continue;
    ctx.beginPath();
    ctx.arc(xOf(i), yOf(v), r, 0, Math.PI * 2);
    ctx.fill();
  }
}

function paintExtremaMarkers(
  ctx: CanvasRenderingContext2D,
  group: SparklineGroup,
  values: ReadonlyArray<number | null>,
  xOf: (i: number) => number,
  yOf: (v: number) => number,
  r: number,
): void {
  const present: { i: number; v: number }[] = [];
  for (let i = 0; i < values.length; i++) {
    const v = values[i];
    if (v != null) present.push({ i, v });
  }
  if (present.length === 0) return;

  const drawDot = (i: number, v: number, color: string) => {
    ctx.fillStyle = color;
    ctx.beginPath();
    ctx.arc(xOf(i), yOf(v), r, 0, Math.PI * 2);
    ctx.fill();
  };

  if (group.high) {
    let hi = present[0]!;
    for (const p of present) if (p.v > hi.v) hi = p;
    drawDot(hi.i, hi.v, group.colorHigh ? `#${group.colorHigh}` : DEFAULT_HIGH);
  }
  if (group.low) {
    let lo = present[0]!;
    for (const p of present) if (p.v < lo.v) lo = p;
    drawDot(lo.i, lo.v, group.colorLow ? `#${group.colorLow}` : DEFAULT_LOW);
  }
  if (group.negative) {
    const color = group.colorNegative ? `#${group.colorNegative}` : DEFAULT_NEGATIVE;
    for (const p of present) if (p.v < 0) drawDot(p.i, p.v, color);
  }
  if (group.first) {
    const f = present[0]!;
    drawDot(f.i, f.v, group.colorFirst ? `#${group.colorFirst}` : DEFAULT_FIRST);
  }
  if (group.last) {
    const l = present[present.length - 1]!;
    drawDot(l.i, l.v, group.colorLast ? `#${group.colorLast}` : DEFAULT_LAST);
  }
}

// ---------------------------------------------------------------------
// Column sparklines

function drawColumnSparkline(
  ctx: CanvasRenderingContext2D,
  group: SparklineGroup,
  sp: Sparkline,
  rect: CellRect,
): void {
  const values = sp.values ?? [];
  if (values.length === 0) return;
  const range = resolveRange(group, values);
  // Baseline = max(0, range.min); columns grow from there.
  const baseline = Math.max(range.min, Math.min(0, range.max));
  const yOf = (v: number) => {
    const t = (v - range.min) / (range.max - range.min);
    return rect.y + (1 - t) * rect.h;
  };
  const yBase = yOf(baseline);

  // 1px gap between bars; bar width adapts.
  const total = values.length;
  const slotW = rect.w / total;
  const barW = Math.max(1, Math.floor(slotW) - 1);
  const seriesColor = group.colorSeries ? `#${group.colorSeries}` : DEFAULT_SERIES;
  const negColor = group.colorNegative ? `#${group.colorNegative}` : DEFAULT_NEGATIVE;

  // Compute extrema indices for high/low/first/last paint
  let hiIdx = -1;
  let loIdx = -1;
  let firstIdx = -1;
  let lastIdx = -1;
  for (let i = 0; i < values.length; i++) {
    const v = values[i];
    if (v == null) continue;
    if (firstIdx === -1) firstIdx = i;
    lastIdx = i;
    if (hiIdx === -1 || (values[hiIdx]! < v)) hiIdx = i;
    if (loIdx === -1 || (values[loIdx]! > v)) loIdx = i;
  }

  for (let i = 0; i < values.length; i++) {
    const v = values[i];
    if (v == null) continue;
    const x = rect.x + i * slotW + Math.floor((slotW - barW) / 2);
    const yv = yOf(v);
    const top = Math.min(yBase, yv);
    const h = Math.max(1, Math.abs(yBase - yv));
    let color = seriesColor;
    if (group.negative && v < 0) color = negColor;
    if (group.high && i === hiIdx) color = group.colorHigh ? `#${group.colorHigh}` : DEFAULT_HIGH;
    if (group.low && i === loIdx) color = group.colorLow ? `#${group.colorLow}` : DEFAULT_LOW;
    if (group.first && i === firstIdx)
      color = group.colorFirst ? `#${group.colorFirst}` : DEFAULT_FIRST;
    if (group.last && i === lastIdx)
      color = group.colorLast ? `#${group.colorLast}` : DEFAULT_LAST;
    ctx.fillStyle = color;
    ctx.fillRect(Math.round(x), Math.round(top), barW, Math.round(h));
  }

  if (group.displayXAxis && range.min < 0 && range.max > 0) {
    ctx.strokeStyle = group.colorAxis ? `#${group.colorAxis}` : DEFAULT_AXIS;
    ctx.lineWidth = 0.5;
    ctx.beginPath();
    const y = yBase + 0.5;
    ctx.moveTo(rect.x, y);
    ctx.lineTo(rect.x + rect.w, y);
    ctx.stroke();
  }
}

// ---------------------------------------------------------------------
// Win/loss sparklines (OOXML "stacked")
//
// All positive values become identical-height up-bars; all negatives
// become identical-height down-bars; zero/empty becomes a gap. Color
// override comes from `colorNegative` for the down-bars.

function drawWinLossSparkline(
  ctx: CanvasRenderingContext2D,
  group: SparklineGroup,
  sp: Sparkline,
  rect: CellRect,
): void {
  const values = sp.values ?? [];
  if (values.length === 0) return;
  const slotW = rect.w / values.length;
  const barW = Math.max(1, Math.floor(slotW) - 1);
  // Bar height = ~45% of the rect, drawn from the vertical center.
  const halfH = Math.max(1, Math.floor(rect.h * 0.45));
  const midY = rect.y + rect.h / 2;
  const seriesColor = group.colorSeries ? `#${group.colorSeries}` : DEFAULT_SERIES;
  const negColor = group.colorNegative ? `#${group.colorNegative}` : DEFAULT_NEGATIVE;

  let firstIdx = -1;
  let lastIdx = -1;
  for (let i = 0; i < values.length; i++) {
    if (values[i] != null && values[i] !== 0) {
      if (firstIdx === -1) firstIdx = i;
      lastIdx = i;
    }
  }

  for (let i = 0; i < values.length; i++) {
    const v = values[i];
    if (v == null || v === 0) continue;
    const x = Math.round(rect.x + i * slotW + (slotW - barW) / 2);
    let color = v > 0 ? seriesColor : negColor;
    if (group.first && i === firstIdx)
      color = group.colorFirst ? `#${group.colorFirst}` : DEFAULT_FIRST;
    if (group.last && i === lastIdx)
      color = group.colorLast ? `#${group.colorLast}` : DEFAULT_LAST;
    ctx.fillStyle = color;
    if (v > 0) {
      ctx.fillRect(x, Math.round(midY - halfH), barW, halfH);
    } else {
      ctx.fillRect(x, Math.round(midY), barW, halfH);
    }
  }

  if (group.displayXAxis) {
    ctx.strokeStyle = group.colorAxis ? `#${group.colorAxis}` : DEFAULT_AXIS;
    ctx.lineWidth = 0.5;
    ctx.beginPath();
    const y = Math.round(midY) + 0.5;
    ctx.moveTo(rect.x, y);
    ctx.lineTo(rect.x + rect.w, y);
    ctx.stroke();
  }
}
