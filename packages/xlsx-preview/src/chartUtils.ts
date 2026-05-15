import type { Chart, ChartSeries, DataLabels } from "./types.js";
import type { Rect } from "./chart.js";

const AXIS_FONT_SIZE = 10;
const LEGEND_FONT_SIZE = 11;
const GRIDLINE_COLOR = "#e5e7eb";
const AXIS_LABEL_COLOR = "#52525b";
/** Heavier than `GRIDLINE_COLOR`; used for the zero baseline when an axis
 *  straddles zero. Painted after fills so it reads as the conceptual zero
 *  line, distinct from the lighter niceTick gridlines. See parity-charts.md
 *  Bug #13 step 1. */
const ZERO_BASELINE_COLOR = "#7a7a7a";
const ZERO_BASELINE_WIDTH = 1.5;
const ZERO_EPS = 1e-9;

/// True iff the value axis straddles zero (zero falls strictly inside
/// the data range). When false, the chart frame's bottom edge already
/// IS the zero baseline and no extra line is needed.
export function axisStraddlesZero(minV: number, maxV: number): boolean {
  return minV < -ZERO_EPS && maxV > ZERO_EPS;
}

/// Shared zero-baseline metrics for a value-axis-on-`inner` projection.
/// Returns the fraction of the axis range that lies at or below zero
/// (`zeroFrac`), and the actual canvas coordinates of the zero line
/// projected onto `inner` for both vertical (`zeroY`, value axis runs
/// up-down) and horizontal (`zeroX`, value axis runs left-right) bar
/// layouts. `straddlesZero` is true iff zero falls strictly inside
/// `[minV, maxV]`; when false, `zeroFrac` is clamped to 0/1 so callers
/// that read `zeroY` / `zeroX` unconditionally still get a sensible
/// edge coordinate (matches Excel: a non-negative axis treats its
/// bottom edge as the conceptual zero). Consolidates the duplicated
/// math that previously lived in every bar/line/area/scatter/combo
/// painter; see parity-charts.md Bug #13 step 3.
export interface ZeroAxisMetrics {
  straddlesZero: boolean;
  zeroFrac: number;
  zeroY: number;
  zeroX: number;
}
export function zeroAxisMetrics(inner: Rect, minV: number, maxV: number): ZeroAxisMetrics {
  const range = maxV - minV;
  // Guard against a zero/NaN range (entirely-constant data); the painter
  // is responsible for never indexing into such a chart but we still
  // return a defined coordinate (the bottom edge) for completeness.
  const rawFrac = range > 0 ? (0 - minV) / range : 0;
  const zeroFrac = Math.max(0, Math.min(1, rawFrac));
  return {
    straddlesZero: axisStraddlesZero(minV, maxV),
    zeroFrac,
    zeroY: inner.y + (1 - zeroFrac) * inner.h,
    zeroX: inner.x + zeroFrac * inner.w,
  };
}

/// Paint the dedicated heavier zero-baseline stroke when an axis
/// straddles zero. No-op otherwise. Callers should invoke this *after*
/// fills so the baseline reads as a conceptual divider on top of
/// negative/positive bars; for line/area painters the call order doesn't
/// matter since the line strokes are thin enough not to obscure the
/// baseline. See parity-charts.md Bug #13 step 1.
export function paintZeroBaseline(
  ctx: CanvasRenderingContext2D,
  inner: Rect,
  minV: number,
  maxV: number,
): void {
  const z = zeroAxisMetrics(inner, minV, maxV);
  if (!z.straddlesZero) return;
  const y = z.zeroY;
  const prevStroke = ctx.strokeStyle;
  const prevWidth = ctx.lineWidth;
  ctx.strokeStyle = ZERO_BASELINE_COLOR;
  ctx.lineWidth = ZERO_BASELINE_WIDTH;
  ctx.beginPath();
  // 1.5px stroke: skip the +0.5 nudge — at non-integer width the canvas
  // anti-aliases either way; centering on an integer keeps both edges
  // sharp-ish.
  ctx.moveTo(inner.x, Math.round(y));
  ctx.lineTo(inner.x + inner.w, Math.round(y));
  ctx.stroke();
  ctx.strokeStyle = prevStroke;
  ctx.lineWidth = prevWidth;
}

/// True iff a tick value lies on the zero baseline of an axis that
/// straddles zero. Used to suppress the lighter niceTick gridline at
/// exactly 0 so it doesn't double-paint with `paintZeroBaseline`.
export function isZeroTickInside(t: number, minV: number, maxV: number): boolean {
  return Math.abs(t) < ZERO_EPS && axisStraddlesZero(minV, maxV);
}
const DATA_LABEL_FONT_SIZE = 9;
const DATA_LABEL_COLOR = "#1f2937";

// ---------- shared helpers ----------

/**
 * Compute clustered/stacked bar geometry inside a single category slot.
 *
 * ECMA-376 §21.2.2.75 (`<c:gapWidth>`) + §21.2.2.108 (`<c:overlap>`):
 * - `gapWidth` = % of *bar width* left as space *between category groups*.
 *   Default 150 (per spec). Range 0..500.
 * - `overlap`  = % adjacent series in the same category overlap each other.
 *   Range -100..100. Default 0 for clustered, 100 for stacked (Excel writes
 *   it explicitly either way).
 *
 * Slot geometry given group count `N` and slot width `slotW`:
 *   - Each subsequent bar is shifted by `barW * (1 - overlap/100)` instead
 *     of `barW` (so positive overlap shrinks the per-bar shift).
 *   - Total span occupied by N bars = `barW * (1 + (N-1) * (1 - overlap/100))`.
 *   - Required free space = `barW * gapWidth/100`.
 *   - Solve: `barW = slotW / (1 + (N-1) * (1 - overlap/100) + gapWidth/100)`.
 *
 * For stacked (or N=1), the formula collapses to `barW = slotW / (1 + gapWidth/100)`.
 *
 * Returns:
 *   - `barW`: width of a single bar in CSS px.
 *   - `firstBarLeftOffset`: offset from the slot's *left edge* to the
 *     left edge of bar index 0. Bar `i` sits at
 *     `slotLeft + firstBarLeftOffset + i * barShift`.
 *   - `barShift`: per-series center-to-center shift = `barW * (1 - overlap/100)`.
 *     For stacked all bars sit at the same x (caller can ignore this).
 */
export interface BarSlotMetrics {
  barW: number;
  firstBarLeftOffset: number;
  barShift: number;
}
export function computeBarSlotMetrics(
  slotW: number,
  seriesCount: number,
  stacked: boolean,
  gapWidthPct: number | undefined,
  overlapPct: number | undefined,
): BarSlotMetrics {
  // Spec defaults. Excel-emitted XML almost always carries explicit values
  // (e.g. `<c:gapWidth val="150"/>` + `<c:overlap val="100"/>` for stacked),
  // so this branch only fires for hand-rolled or sparse files.
  const gw = Math.max(0, Math.min(500, gapWidthPct ?? 150));
  const ov = stacked ? 100 : Math.max(-100, Math.min(100, overlapPct ?? 0));
  const N = stacked ? 1 : Math.max(1, seriesCount);
  const shiftFactor = 1 - ov / 100; // 0 when fully stacked, 1 when no overlap
  const denom = 1 + (N - 1) * shiftFactor + gw / 100;
  const barW = slotW / denom;
  const totalSpan = barW * (1 + (N - 1) * shiftFactor);
  const firstBarLeftOffset = (slotW - totalSpan) / 2;
  const barShift = barW * shiftFactor;
  return { barW, firstBarLeftOffset, barShift };
}

/// Compute (minV, maxV) over a set of parallel rows (each row already at
/// the same x positions; useful for stacked tops + bottoms).
export function valueRange(rows: number[][]): { minV: number; maxV: number } {
  // Seed with +/-Infinity so entirely-positive (or entirely-negative)
  // data isn't silently zero-clamped here. Callers route through
  // `resolveAxisRange`, which handles the bars-default-to-zero rule
  // explicitly. Pre-`resolveAxisRange` line/area charts used to rely
  // on this clamp; the helper now does the same thing in one place.
  let minV = Number.POSITIVE_INFINITY,
    maxV = Number.NEGATIVE_INFINITY;
  for (const r of rows) {
    for (const v of r) {
      if (v > maxV) maxV = v;
      if (v < minV) minV = v;
    }
  }
  if (!Number.isFinite(minV)) minV = 0;
  if (!Number.isFinite(maxV)) maxV = 1;
  return { minV, maxV };
}

/// Build per-series cumulative-top arrays for a stacked plot. For
/// percentStacked each per-category column normalises to 100.
export function buildStackedRows(
  series: ChartSeries[],
  categoryCount: number,
  percent: boolean,
): number[][] {
  const tops: number[][] = series.map((_) => new Array(categoryCount).fill(0));
  for (let i = 0; i < categoryCount; i++) {
    let total = 0;
    if (percent) {
      for (const s of series) total += Math.max(0, s.values[i] ?? 0);
      if (total <= 0) total = 1;
    }
    let acc = 0;
    for (let si = 0; si < series.length; si++) {
      const raw = series[si]!.values[i] ?? 0;
      const v = percent ? (Math.max(0, raw) / total) * 100 : raw;
      acc += v;
      tops[si]![i] = acc;
    }
  }
  return tops;
}

/// Draw the value-axis tick labels + gridlines and return the inner
/// (plot) rectangle. Shared by line / area / scatter (and conceptually
/// bar/column, though that one inlines a similar block).
export function drawAxisFrame(
  ctx: CanvasRenderingContext2D,
  chart: Chart,
  rect: Rect,
  ticks: number[],
  minV: number,
  maxV: number,
  horizontal: boolean,
  percent: boolean,
): Rect {
  ctx.font = `${AXIS_FONT_SIZE}px -apple-system, "Helvetica Neue", Arial, sans-serif`;
  const labelStrings = ticks.map((t) =>
    percent ? `${Math.round(t)}%` : formatAxisValue(t, chart.valueFormat, chart.dispUnits),
  );
  const yAxisW = Math.max(...labelStrings.map((s) => ctx.measureText(s).width)) + 8;
  const xAxisH = AXIS_FONT_SIZE + 8;
  const inner: Rect = horizontal
    ? { x: rect.x + yAxisW, y: rect.y, w: rect.w - yAxisW, h: rect.h - xAxisH }
    : { x: rect.x + yAxisW, y: rect.y, w: rect.w - yAxisW, h: rect.h - xAxisH };

  // Per parity-charts.md Bug #12: only paint gridlines when
  // `<c:majorGridlines>` was present on the value axis (and not
  // explicitly hidden via `<a:noFill/>` on its line). Tick labels
  // always paint; suppressing gridlines doesn't suppress labels in
  // Excel.
  const showGridlines = chart.showMajorGridlines !== false;
  ctx.fillStyle = AXIS_LABEL_COLOR;
  ctx.strokeStyle = GRIDLINE_COLOR;
  ctx.lineWidth = 1;
  ctx.textAlign = horizontal ? "center" : "right";
  ctx.textBaseline = "middle";
  for (let ti = 0; ti < ticks.length; ti++) {
    const t = ticks[ti]!;
    const frac = (t - minV) / (maxV - minV);
    // Bug #13 step 1: when the axis straddles zero, suppress the
    // lighter niceTick gridline at t==0 — the caller will overlay the
    // heavier `paintZeroBaseline` stroke at the same coord after fills.
    const isZeroLine = isZeroTickInside(t, minV, maxV);
    if (horizontal) {
      const x = inner.x + frac * inner.w;
      if (showGridlines && !isZeroLine) {
        ctx.beginPath();
        ctx.moveTo(Math.round(x) + 0.5, inner.y);
        ctx.lineTo(Math.round(x) + 0.5, inner.y + inner.h);
        ctx.stroke();
      }
      ctx.fillText(labelStrings[ti]!, x, inner.y + inner.h + xAxisH / 2);
    } else {
      const y = inner.y + (1 - frac) * inner.h;
      if (showGridlines && !isZeroLine) {
        ctx.beginPath();
        ctx.moveTo(inner.x, Math.round(y) + 0.5);
        ctx.lineTo(inner.x + inner.w, Math.round(y) + 0.5);
        ctx.stroke();
      }
      ctx.fillText(labelStrings[ti]!, inner.x - 4, y);
    }
  }
  // Axis baselines.
  ctx.strokeStyle = "#9ca3af";
  ctx.beginPath();
  ctx.moveTo(inner.x, Math.round(inner.y + inner.h) + 0.5);
  ctx.lineTo(inner.x + inner.w, Math.round(inner.y + inner.h) + 0.5);
  ctx.moveTo(Math.round(inner.x) + 0.5, inner.y);
  ctx.lineTo(Math.round(inner.x) + 0.5, inner.y + inner.h);
  ctx.stroke();
  return inner;
}

export function drawCategoryAxis(
  ctx: CanvasRenderingContext2D,
  chart: Chart,
  inner: Rect,
  categoryCount: number,
  horizontal: boolean,
): void {
  ctx.font = `${AXIS_FONT_SIZE}px -apple-system, "Helvetica Neue", Arial, sans-serif`;
  ctx.fillStyle = AXIS_LABEL_COLOR;
  ctx.textAlign = "center";
  ctx.textBaseline = horizontal ? "middle" : "top";
  // Line/area put points on the cat boundaries (i / (n-1)); bar puts
  // them on cat centers ((i+0.5)/n). The horizontal=false path here is
  // only used by line/area today.
  const denom = Math.max(1, categoryCount - 1);
  // Decimate labels so they don't overlap into an unreadable bar.
  // Walk forward, drawing a label only when the previous one's right
  // edge is comfortably past the new one's left edge.
  const fmt = chart.categoriesFormat;
  const labels = Array.from({ length: categoryCount }, (_, i) => {
    const raw = chart.categories[i] ?? `${i + 1}`;
    if (!fmt) return raw;
    const n = parseFloat(raw);
    if (!Number.isFinite(n)) return raw;
    return formatValue(n, fmt).text;
  });
  const minGapPx = 8;
  let lastRight = -Infinity;
  for (let i = 0; i < categoryCount; i++) {
    const label = labels[i]!;
    const w = ctx.measureText(label).width;
    if (horizontal) {
      ctx.fillText(label, inner.x - 4, inner.y + (i / denom) * inner.h);
      continue;
    }
    const cx = inner.x + (i / denom) * inner.w;
    const left = cx - w / 2;
    if (left < lastRight + minGapPx) continue;
    ctx.fillText(label, cx, inner.y + inner.h + 4);
    lastRight = cx + w / 2;
  }
}

/// Lightly translucent variant of a CSS hex color, for area fills.
/// Pass-through for non-hex; alpha is multiplied into the existing color.
export function withAlpha(color: string, alpha: number): string {
  const m = /^#([0-9a-f]{6})$/i.exec(color);
  if (!m) return color;
  const hex = m[1]!;
  const r = parseInt(hex.slice(0, 2), 16);
  const g = parseInt(hex.slice(2, 4), 16);
  const b = parseInt(hex.slice(4, 6), 16);
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

// Import lazily so chartUtils remains a leaf module for callers that do not
// need category-axis number formatting.
import { formatValue } from "./numfmt.js";

// ---------- legend ----------

/// Legend-swatch kind, per series. Mirrors what the painter draws so
/// the legend reads at a glance:
///   - `swatch`     filled square    (column / bar / area / pie / doughnut)
///   - `line`       horizontal stroke (line/scatter with no markers)
///   - `marker`     filled circle    (scatter marker-only)
///   - `lineMarker` stroke + circle  (line, lineMarker, smoothMarker)
///
/// `chart` is optional so callers that don't have the chart context
/// (e.g. ad-hoc preview tooling) still get the old square behavior.
export type LegendSwatchKind = "swatch" | "line" | "marker" | "lineMarker";

function legendKindFor(chart: Chart | undefined, s: ChartSeries): LegendSwatchKind {
  if (!chart) return "swatch";
  // Mirror `seriesKind` in chart.ts: per-series override > chart-level.
  // For a non-combo chart with unset `s.chartType` we inherit `chart.type`;
  // combo charts always set `s.chartType` per group at extraction time.
  const kind = s.chartType ?? chart.type;
  // ECMA-376 §21.2.3.10 `<c:marker><c:symbol val="none"/>`: explicitly
  // suppress the per-point glyph for this series. Excel still paints
  // the connecting stroke for line series, so the legend swatch drops
  // the marker dot but keeps the line. Source workbook fixture:
  // chart32.xml (Charts_1__Chart_11) where the Technology line series
  // sets `<c:marker><c:symbol val="none"/>` and Excel renders a bare
  // navy stroke with no glyphs.
  const noMarker = s.markerSymbol === "none";
  if (kind === "line") return noMarker ? "line" : "lineMarker";
  if (kind === "scatter") {
    // ECMA-376 §21.2.3.40 c:scatterStyle: none/line/lineMarker/marker/
    // smooth/smoothMarker. Default for our extractor is unset = marker-
    // only (matches the drawScatterChart treatment).
    const style = chart.scatterStyle;
    if (style === "line" || style === "smooth") return "line";
    if (style === "lineMarker" || style === "smoothMarker") {
      return noMarker ? "line" : "lineMarker";
    }
    return noMarker ? "swatch" : "marker";
  }
  return "swatch";
}

/// Paint one legend swatch in the budget `[x, x+w] x [y - w/2, y + w/2]`
/// (width matches the historical 10px square so the legend layout math is
/// pixel-stable regardless of per-series kind).
function paintLegendSwatch(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  w: number,
  color: string,
  kind: LegendSwatchKind,
): void {
  ctx.fillStyle = color;
  if (kind === "swatch") {
    ctx.fillRect(x, y - w / 2, w, w);
    return;
  }
  if (kind === "marker") {
    ctx.beginPath();
    ctx.arc(x + w / 2, y, 3, 0, Math.PI * 2);
    ctx.fill();
    return;
  }
  // `line` or `lineMarker`: short horizontal stroke spanning the swatch.
  const prevStroke = ctx.strokeStyle;
  const prevWidth = ctx.lineWidth;
  ctx.strokeStyle = color;
  ctx.lineWidth = 2;
  ctx.beginPath();
  ctx.moveTo(x, y);
  ctx.lineTo(x + w, y);
  ctx.stroke();
  ctx.strokeStyle = prevStroke;
  ctx.lineWidth = prevWidth;
  if (kind === "lineMarker") {
    ctx.beginPath();
    ctx.arc(x + w / 2, y, 3, 0, Math.PI * 2);
    ctx.fill();
  }
}

export function drawLegend(
  ctx: CanvasRenderingContext2D,
  series: ChartSeries[],
  rect: Rect,
  orientation: "horizontal" | "vertical" = "horizontal",
  chart?: Chart,
): void {
  ctx.font = `${LEGEND_FONT_SIZE}px -apple-system, "Helvetica Neue", Arial, sans-serif`;
  ctx.textBaseline = "middle";
  const swatchW = 10;
  if (orientation === "vertical") {
    // Stack one entry per line beside the plot area.
    const lineH = LEGEND_FONT_SIZE + 6;
    const totalH = series.length * lineH;
    let y = rect.y + Math.max(0, (rect.h - totalH) / 2) + lineH / 2;
    const x = rect.x;
    for (let i = 0; i < series.length; i++) {
      const s = series[i]!;
      paintLegendSwatch(ctx, x, y, swatchW, s.color ?? "#4472C4", legendKindFor(chart, s));
      ctx.fillStyle = AXIS_LABEL_COLOR;
      ctx.textAlign = "left";
      ctx.fillText(s.name || `Series ${i + 1}`, x + swatchW + 4, y);
      y += lineH;
    }
    return;
  }
  const itemPad = 16;
  // Measure total width to center.
  const widths = series.map((s) => swatchW + 6 + ctx.measureText(s.name || "").width);
  const totalW = widths.reduce((a, b) => a + b, 0) + itemPad * (series.length - 1);
  let x = rect.x + (rect.w - totalW) / 2;
  const y = rect.y + rect.h / 2;
  for (let i = 0; i < series.length; i++) {
    const s = series[i]!;
    paintLegendSwatch(ctx, x, y, swatchW, s.color ?? "#4472C4", legendKindFor(chart, s));
    ctx.fillStyle = AXIS_LABEL_COLOR;
    ctx.textAlign = "left";
    ctx.fillText(s.name || `Series ${i + 1}`, x + swatchW + 4, y);
    x += widths[i]! + itemPad;
  }
}

/// Measure the width needed by a vertical legend column, including the
/// swatch, gap, and widest label.
export function measureVerticalLegendWidth(
  ctx: CanvasRenderingContext2D,
  series: ChartSeries[],
): number {
  if (series.length === 0) return 0;
  ctx.font = `${LEGEND_FONT_SIZE}px -apple-system, "Helvetica Neue", Arial, sans-serif`;
  const swatchW = 10;
  let maxLabel = 0;
  for (const s of series) {
    const w = ctx.measureText(s.name || "").width;
    if (w > maxLabel) maxLabel = w;
  }
  return Math.ceil(swatchW + 4 + maxLabel + 4);
}

// ---------- placeholder ----------

export function drawPlaceholderPlot(ctx: CanvasRenderingContext2D, chart: Chart, rect: Rect): void {
  ctx.fillStyle = "#f4f4f5";
  ctx.fillRect(rect.x, rect.y, rect.w, rect.h);
  ctx.fillStyle = AXIS_LABEL_COLOR;
  ctx.font = `12px -apple-system, "Helvetica Neue", Arial, sans-serif`;
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  const label = `${chart.type} chart (renderer v0 stub)`;
  ctx.fillText(label, rect.x + rect.w / 2, rect.y + rect.h / 2);
}

// ---------- data labels ----------

/// Pick the per-series effective DataLabels block (series wins over chart).
export function effectiveLabels(chart: Chart, s: ChartSeries): DataLabels | undefined {
  return s.dataLabels ?? chart.dataLabels;
}

/// Per-data-point resolution of a `DataLabels` block. Returns:
///   - `undefined` — paint the parent block as-is (no override).
///   - `null`       — this point is suppressed (`<c:dLbl><c:delete/>`).
///   - `{ dl, text }` — paint with `dl` (parent merged with point
///                       override) and optional literal `text`.
/// Callers can do:
///   const o = pointLabel(dl, i);
///   if (o === null) continue;
///   const effective = o?.dl ?? dl;
///   const text = o?.text ?? buildLabelText(effective, ...);
export function pointLabel(
  base: DataLabels,
  i: number,
): { dl: DataLabels; text?: string } | null | undefined {
  const po = base.pointOverrides?.find((p) => p.idx === i);
  if (!po) return undefined;
  if (po.delete) return null;
  const merged: DataLabels = {
    showValue: po.showValue ?? base.showValue,
    showCategory: po.showCategory ?? base.showCategory,
    showSeriesName: po.showSeriesName ?? base.showSeriesName,
    showPercent: po.showPercent ?? base.showPercent,
    position: po.position ?? base.position,
    separator: base.separator,
    numFmt: po.numFmt ?? base.numFmt,
    pointOverrides: base.pointOverrides,
  };
  return { dl: merged, text: po.text };
}

/// Build the label string for a single (series, category, value) tuple.
/// Concatenates the enabled show* fields with `dl.separator` (default `", "`).
export function buildLabelText(
  dl: DataLabels,
  chart: Chart,
  series: ChartSeries,
  categoryIdx: number,
  value: number,
  categoryTotal: number,
): string {
  const sep = dl.separator ?? ", ";
  const parts: string[] = [];
  if (dl.showSeriesName && series.name) parts.push(series.name);
  if (dl.showCategory) {
    // chart.categories is `skip_serializing_if = Vec::is_empty` on the
    // Rust side, so the JSON omits it for pie/etc. with no cat axis.
    // Bare-array access would throw "cannot read [i] of undefined" and
    // silently kill the rest of the chart's render.
    const cats = chart.categories ?? [];
    const c = cats[categoryIdx];
    if (c != null && c !== "") parts.push(c);
  }
  // showPercent and showValue can both be on; Excel honors both.
  if (dl.showPercent && categoryTotal > 0) {
    const pct = (value / categoryTotal) * 100;
    parts.push(`${Math.round(pct)}%`);
  }
  if (dl.showValue) {
    const fmt = dl.numFmt ?? chart.valueFormat;
    parts.push(formatAxisValue(value, fmt));
  }
  return parts.join(sep);
}

/// Paint a single label centered at (x, y) with a soft white halo so it
/// stays readable against bar fills / line series colors.
export function drawLabel(
  ctx: CanvasRenderingContext2D,
  text: string,
  x: number,
  y: number,
  align: CanvasTextAlign = "center",
  baseline: CanvasTextBaseline = "middle",
): void {
  if (!text) return;
  ctx.font = `${DATA_LABEL_FONT_SIZE}px -apple-system, "Helvetica Neue", Arial, sans-serif`;
  ctx.textAlign = align;
  ctx.textBaseline = baseline;
  // Halo for legibility on top of colored fills.
  ctx.lineWidth = 3;
  ctx.strokeStyle = "rgba(255,255,255,0.85)";
  ctx.lineJoin = "round";
  ctx.strokeText(text, x, y);
  ctx.lineWidth = 1;
  ctx.fillStyle = DATA_LABEL_COLOR;
  ctx.fillText(text, x, y);
}

// ---------- helpers ----------

/// Resolve a chart axis range, honoring optional explicit min/max from
/// `<c:scaling>`. Behavior:
///
/// - If neither bound is forced, falls back to the data range; when
///   `zeroClamp` is true we additionally pull the floor to 0 (for
///   bars/columns) or the ceiling to 0 (for negative-only data),
///   matching Excel's default for bar/column charts.
/// - If either bound is forced, the axis is treated as "manually
///   scaled" — zero-clamping is suppressed (Excel does the same when
///   the user sets a min or max in the chart pane). The forced
///   endpoint becomes the exact axis terminus; niceTicks fills in
///   round intermediate ticks toward the other (auto) end and the
///   forced endpoint is appended/prepended as the final tick.
///
/// Returns the resolved `[minV, maxV]` plus the tick list ready for
/// gridline + label rendering.
export function resolveAxisRange(
  dataMin: number,
  dataMax: number,
  forcedMin: number | undefined,
  forcedMax: number | undefined,
  zeroClamp: boolean,
  tickCount: number,
  forcedMajorUnit?: number,
): { minV: number; maxV: number; ticks: number[] } {
  let lo = forcedMin ?? dataMin;
  let hi = forcedMax ?? dataMax;
  const userScaled =
    forcedMin !== undefined || forcedMax !== undefined || forcedMajorUnit !== undefined;
  if (zeroClamp && !userScaled) {
    if (lo > 0) lo = 0;
    if (hi < 0) hi = 0;
  }
  if (lo === hi) hi = lo + 1;
  const EPS = 1e-9;
  // `<c:majorUnit>` path: step ticks exactly by the authored unit
  // (ECMA-376 §21.2.2.121). Anchor the cadence on the *forced* min if
  // the workbook pinned one, else on a multiple of majorUnit below
  // dataMin so the bottom tick still lands cleanly; mirror the same
  // logic on the top so a pinned max remains the last tick. Skips
  // pathological cases (non-finite or implausibly tiny step that
  // would produce hundreds of ticks) and falls back to niceTicks.
  let t: number[];
  if (
    forcedMajorUnit !== undefined &&
    Number.isFinite(forcedMajorUnit) &&
    forcedMajorUnit > 0 &&
    (hi - lo) / forcedMajorUnit < 1000
  ) {
    const step = forcedMajorUnit;
    // Anchor the cadence on `forcedMin` when set, else 0 — matches
    // Excel's behaviour where major-unit ticks land on multiples of
    // step counted from the forced bound (or from zero).
    const anchor = forcedMin !== undefined ? forcedMin : 0;
    // When no `forcedMin` was authored, Excel walks the step grid
    // down to zero (or just past dataMin if data straddles zero) so
    // long as the resulting tick count is reasonable — a positive
    // series like 18..43 with `<c:max val="45000"/>` +
    // `<c:majorUnit val="9000"/>` renders 0/9/18/27/36/45, not just
    // 9..45. We cap the implicit extension at 14 ticks total so we
    // don't blow up axes where step is tiny relative to the data
    // (e.g. data 100..200 with step=1 — keep niceTicks-style floor
    // at dataMin instead of dropping all the way to 0).
    let niceMin: number;
    if (forcedMin !== undefined) {
      niceMin = forcedMin;
    } else {
      const floorAtData = anchor + Math.floor((lo - anchor) / step + EPS) * step;
      const floorAtZero = anchor + Math.min(0, Math.floor((lo - anchor) / step + EPS)) * step;
      const tentativeMax = anchor + Math.ceil((hi - anchor) / step - EPS) * step;
      const tickCountToZero = Math.round((tentativeMax - floorAtZero) / step) + 1;
      niceMin = tickCountToZero <= 14 ? floorAtZero : floorAtData;
    }
    const niceMax = anchor + Math.ceil((hi - anchor) / step - EPS) * step;
    t = [];
    for (let v = niceMin; v <= niceMax + step / 2; v += step) {
      t.push(parseFloat(v.toPrecision(12)));
    }
    if (t.length === 0) t.push(lo, hi);
  } else {
    t = niceTicks(lo, hi, tickCount);
  }
  if (forcedMin !== undefined) {
    t = t.filter((v) => v >= forcedMin - EPS);
    if (t.length === 0 || t[0]! > forcedMin + EPS) t.unshift(forcedMin);
  }
  if (forcedMax !== undefined) {
    t = t.filter((v) => v <= forcedMax + EPS);
    if (t.length === 0 || t[t.length - 1]! < forcedMax - EPS) t.push(forcedMax);
  }
  return { minV: t[0]!, maxV: t[t.length - 1]!, ticks: t };
}

export function niceTicks(min: number, max: number, count: number): number[] {
  if (max === min) {
    max = min + 1;
  }
  const range = niceNum(max - min, false);
  const step = niceNum(range / Math.max(1, count - 1), true);
  const niceMin = Math.floor(min / step) * step;
  const niceMax = Math.ceil(max / step) * step;
  const out: number[] = [];
  for (let v = niceMin; v <= niceMax + step / 2; v += step) {
    out.push(parseFloat(v.toPrecision(12)));
  }
  return out;
}

export function niceNum(range: number, round: boolean): number {
  const exp = Math.floor(Math.log10(Math.max(1e-12, Math.abs(range))));
  const f = range / Math.pow(10, exp);
  let nf: number;
  if (round) {
    if (f < 1.5) nf = 1;
    else if (f < 3) nf = 2;
    else if (f < 7) nf = 5;
    else nf = 10;
  } else {
    if (f <= 1) nf = 1;
    else if (f <= 2) nf = 2;
    else if (f <= 5) nf = 5;
    else nf = 10;
  }
  return nf * Math.pow(10, exp);
}

// Inline copy of the simple axis formatter used by cells. Keep it small;
// the Chart only ever passes through `valueFormat` from the value-axis.
//
// `divisor` lowers `<c:dispUnits>` (ECMA-376 §21.2.2.46) onto the tick
// labels: when an axis is authored with `builtInUnit=thousands` (or a
// custom unit), every tick label is divided before formatting so a
// 75,000 axis terminus reads as `75` with an `S$ mn` caption painted
// near the axis. Pass `undefined` (or omit) for the no-op case.
export function formatAxisValue(
  v: number,
  fmt: string | undefined,
  divisor?: number | null,
): string {
  if (divisor != null && Number.isFinite(divisor) && divisor > 0) {
    v = v / divisor;
  }
  if (!fmt || fmt === "General") return formatGeneral(v);
  const stripped = fmt.replace(/\[[^\]]*\]/g, "");
  const section = stripped.split(";")[0] ?? stripped;
  const decimals = decimalsIn(section);
  if (section.includes("%")) return (v * 100).toFixed(decimals) + "%";
  if (section.includes("$")) {
    const grouped = section.includes(",") || section.includes("#,##");
    return "$" + (grouped ? withGrouping(v, decimals) : v.toFixed(decimals));
  }
  if (section.includes(",")) return withGrouping(v, decimals);
  if (section.includes("0") || section.includes("#")) return v.toFixed(decimals);
  return formatGeneral(v);
}
export function formatGeneral(v: number): string {
  if (Number.isInteger(v) && Math.abs(v) < 1e15) return v.toString();
  return parseFloat(v.toPrecision(8)).toString();
}
export function decimalsIn(fmt: string): number {
  const i = fmt.indexOf(".");
  if (i < 0) return 0;
  let n = 0;
  for (let j = i + 1; j < fmt.length; j++) {
    const ch = fmt[j];
    if (ch === "0" || ch === "#") n++;
    else break;
  }
  return n;
}
export function withGrouping(v: number, decimals: number): string {
  const neg = v < 0;
  const abs = Math.abs(v).toFixed(decimals);
  const [intPart, frac] = abs.split(".");
  const grouped = (intPart ?? "0").replace(/\B(?=(\d{3})+(?!\d))/g, ",");
  return (neg ? "-" : "") + grouped + (frac ? "." + frac : "");
}
