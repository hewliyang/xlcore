import type { Chart, ChartSeries, DataLabels } from "./types.js";
import type { Rect } from "./chart.js";

const AXIS_FONT_SIZE = 10;
const LEGEND_FONT_SIZE = 11;
const GRIDLINE_COLOR = "#e5e7eb";
const AXIS_LABEL_COLOR = "#52525b";
const DATA_LABEL_FONT_SIZE = 9;
const DATA_LABEL_COLOR = "#1f2937";

// ---------- shared helpers ----------

/// Compute (minV, maxV) over a set of parallel rows (each row already at
/// the same x positions; useful for stacked tops + bottoms).
export function valueRange(rows: number[][]): { minV: number; maxV: number } {
  let minV = 0,
    maxV = 0;
  for (const r of rows) {
    for (const v of r) {
      if (v > maxV) maxV = v;
      if (v < minV) minV = v;
    }
  }
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
    percent ? `${Math.round(t)}%` : formatAxisValue(t, chart.valueFormat),
  );
  const yAxisW = Math.max(...labelStrings.map((s) => ctx.measureText(s).width)) + 8;
  const xAxisH = AXIS_FONT_SIZE + 8;
  const inner: Rect = horizontal
    ? { x: rect.x + yAxisW, y: rect.y, w: rect.w - yAxisW, h: rect.h - xAxisH }
    : { x: rect.x + yAxisW, y: rect.y, w: rect.w - yAxisW, h: rect.h - xAxisH };

  ctx.fillStyle = AXIS_LABEL_COLOR;
  ctx.strokeStyle = GRIDLINE_COLOR;
  ctx.lineWidth = 1;
  ctx.textAlign = horizontal ? "center" : "right";
  ctx.textBaseline = "middle";
  for (let ti = 0; ti < ticks.length; ti++) {
    const t = ticks[ti]!;
    const frac = (t - minV) / (maxV - minV);
    if (horizontal) {
      const x = inner.x + frac * inner.w;
      ctx.beginPath();
      ctx.moveTo(Math.round(x) + 0.5, inner.y);
      ctx.lineTo(Math.round(x) + 0.5, inner.y + inner.h);
      ctx.stroke();
      ctx.fillText(labelStrings[ti]!, x, inner.y + inner.h + xAxisH / 2);
    } else {
      const y = inner.y + (1 - frac) * inner.h;
      ctx.beginPath();
      ctx.moveTo(inner.x, Math.round(y) + 0.5);
      ctx.lineTo(inner.x + inner.w, Math.round(y) + 0.5);
      ctx.stroke();
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

export function drawLegend(
  ctx: CanvasRenderingContext2D,
  series: ChartSeries[],
  rect: Rect,
  orientation: "horizontal" | "vertical" = "horizontal",
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
      ctx.fillStyle = s.color ?? "#4472C4";
      ctx.fillRect(x, y - swatchW / 2, swatchW, swatchW);
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
    ctx.fillStyle = s.color ?? "#4472C4";
    ctx.fillRect(x, y - swatchW / 2, swatchW, swatchW);
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
export function formatAxisValue(v: number, fmt: string | undefined): string {
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
