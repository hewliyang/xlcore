// chartEx (`cx:`) stat-layout painters split out of `chartEx.ts` to fit
// the per-file LoC budget. Three layouts:
//
//   - histogram   — single clusteredColumn + `<cx:binning>`; we auto-bin
//                   raw observations using a Sturges-derived nice width.
//   - pareto      — primary clusteredColumn + secondary paretoLine; the
//                   line points are computed from cumulative-% at draw
//                   time (the OOXML line series carries no own data).
//   - boxWhisker  — N parallel boxWhisker series; we compute Q1/median/
//                   Q3/whiskers/outliers per `QUARTILE.EXC` semantics
//                   (Excel chartEx default `quartileMethod="exclusive"`).
//
// All three are charted by `drawChartEx` in `chartEx.ts`; this module
// only exports the three painters.

import type { Chart } from "./types.js";
import type { Rect } from "./chart.js";
import { activeThemeColor } from "./color.js";
import { DEFAULT_PIE_ACCENTS } from "./chartAdvanced.js";
import { drawAxisFrame, drawPlaceholderPlot, resolveAxisRange } from "./chartUtils.js";

const AXIS_FONT_SIZE = 10;
const AXIS_LABEL_COLOR = "#52525b";
const AXIS_TICK_COUNT = 5;

// ---------- histogram ----------
//
// Excel chartEx histogram is a `<cx:series layoutId="clusteredColumn">`
// whose `<cx:layoutPr>` carries `<cx:binning>`. The series's data
// dimension is the raw observation list (not pre-binned counts);
// Excel auto-bins at render time using Scott's normal-reference rule
// (bin width = 3.5 * sigma / n^(1/3)) by default, or honours explicit
// `<cx:binCount>` / `<cx:binSize>` / `<cx:overflow>` / `<cx:underflow>`.
// We don't surface those overrides yet — the Excel-authored fixture
// uses default auto-binning, so we follow Scott's rule for parity.
//
// `intervalClosed="r"` (the fixture's setting, also Excel's default)
// makes each bin right-closed: `(low, high]`. The leftmost bin is
// also left-closed at the data minimum so the smallest observation
// isn't dropped.
export function drawHistogramChartEx(
  ctx: CanvasRenderingContext2D,
  chart: Chart,
  rect: Rect,
): void {
  const series = chart.series[0];
  if (!series || series.values.length === 0) {
    drawPlaceholderPlot(ctx, chart, rect);
    return;
  }
  const obs = series.values.filter((v) => Number.isFinite(v));
  const n = obs.length;
  if (n === 0) {
    drawPlaceholderPlot(ctx, chart, rect);
    return;
  }
  const minV = Math.min(...obs);
  const maxV = Math.max(...obs);
  if (minV === maxV) {
    drawPlaceholderPlot(ctx, chart, rect);
    return;
  }

  // Pick a bin count via Sturges (ceil(log2 n) + 1) — conservative
  // for small datasets where Scott's rule rounds up too aggressively
  // and squashes us into one or two bins. Then derive the raw bin
  // width from the data span and round up to a "nice" number (1/2/5
  // × 10^k) so labels read as 10 / 20 / 50 rather than 9.7-and-change.
  const sturges = Math.max(2, Math.ceil(Math.log2(Math.max(2, n)) + 1));
  let binWidth = (maxV - minV) / sturges;
  if (!Number.isFinite(binWidth) || binWidth <= 0) {
    binWidth = Math.max(1, (maxV - minV) / Math.max(1, Math.ceil(Math.sqrt(n))));
  }
  binWidth = niceBinWidth(binWidth);
  // Anchor on a multiple of binWidth at or below minV so bin edges
  // are visually sensible (40/50/60 rather than 42/52/62).
  const startEdge = Math.floor(minV / binWidth) * binWidth;
  const endEdge = Math.ceil(maxV / binWidth) * binWidth;
  const binCount = Math.max(1, Math.round((endEdge - startEdge) / binWidth));
  const edges: number[] = [];
  for (let i = 0; i <= binCount; i++)
    edges.push(parseFloat((startEdge + i * binWidth).toPrecision(12)));

  // Count observations per bin. Right-closed intervals; the leftmost
  // bin also includes its left edge so we don't drop the minimum.
  const counts = new Array<number>(binCount).fill(0);
  for (const v of obs) {
    let idx = Math.floor((v - startEdge) / binWidth);
    if (v === edges[idx]! && idx > 0) idx -= 1; // right-closed
    if (idx < 0) idx = 0;
    if (idx >= binCount) idx = binCount - 1;
    counts[idx]! += 1;
  }

  const maxCount = Math.max(...counts);
  const range = resolveAxisRange(
    0,
    maxCount,
    undefined,
    undefined,
    true,
    AXIS_TICK_COUNT,
    undefined,
  );
  const inner = drawAxisFrame(ctx, chart, rect, range.ticks, range.minV, range.maxV, false, false);

  // Bin labels along the category axis. Decimate when crowded.
  const slotW = inner.w / binCount;
  ctx.font = `${AXIS_FONT_SIZE}px -apple-system, "Helvetica Neue", Arial, sans-serif`;
  ctx.fillStyle = AXIS_LABEL_COLOR;
  ctx.textAlign = "center";
  ctx.textBaseline = "top";
  let lastRight = -Infinity;
  for (let i = 0; i < binCount; i++) {
    const lo = edges[i]!;
    const hi = edges[i + 1]!;
    // Excel-style "(lo, hi]" label, with the leftmost bin shown as
    // "[lo, hi]" to flag its left-closed corner.
    const label =
      i === 0 ? `[${fmtBinEdge(lo)}, ${fmtBinEdge(hi)}]` : `(${fmtBinEdge(lo)}, ${fmtBinEdge(hi)}]`;
    const tw = ctx.measureText(label).width;
    const cx = inner.x + (i + 0.5) * slotW;
    if (cx - tw / 2 < lastRight + 4) continue;
    ctx.fillText(label, cx, inner.y + inner.h + 4);
    lastRight = cx + tw / 2;
  }

  // Bars touch (gapWidth=0 in the fixture; Excel histogram default).
  // A 1px white stroke between bars keeps adjacent counts legible.
  const fill = activeThemeColor(4, "#4472C4");
  for (let i = 0; i < binCount; i++) {
    const c = counts[i]!;
    if (c === 0) continue;
    const yTop = inner.y + (1 - (c - range.minV) / (range.maxV - range.minV)) * inner.h;
    const yBot = inner.y + inner.h;
    const x = inner.x + i * slotW;
    ctx.fillStyle = fill;
    ctx.fillRect(x, yTop, slotW, yBot - yTop);
    ctx.strokeStyle = "#ffffff";
    ctx.lineWidth = 1;
    ctx.strokeRect(x + 0.5, yTop + 0.5, slotW - 1, yBot - yTop - 1);
  }
}

/** Round a bin width up to the next "nice" number (1/2/5 * 10^k). */
function niceBinWidth(raw: number): number {
  if (raw <= 0) return 1;
  const exp = Math.floor(Math.log10(raw));
  const base = Math.pow(10, exp);
  const f = raw / base;
  let nf: number;
  if (f <= 1) nf = 1;
  else if (f <= 2) nf = 2;
  else if (f <= 5) nf = 5;
  else nf = 10;
  return nf * base;
}

/** Format a bin edge: integer when whole, else two-decimal. */
function fmtBinEdge(v: number): string {
  if (Number.isInteger(v)) return v.toString();
  return v.toFixed(2).replace(/0+$/, "").replace(/\.$/, "");
}

// ---------- pareto ----------
//
// Bars + cumulative-percentage line. The bars come from the primary
// `clusteredColumn` series (already sorted descending in the source
// workbook — Excel sorts at chart-creation time and stores the
// post-sort order); the line series carries no own data and we
// compute the cumulative percentage at render time. Two y-axes: the
// primary (left) shows raw counts, the secondary (right) shows 0..100%.
export function drawParetoChartEx(ctx: CanvasRenderingContext2D, chart: Chart, rect: Rect): void {
  const bars = chart.series[0];
  if (!bars || bars.values.length === 0) {
    drawPlaceholderPlot(ctx, chart, rect);
    return;
  }
  const values = bars.values.map((v) => (Number.isFinite(v) && v > 0 ? v : 0));
  const n = values.length;
  const cats = chart.categories ?? [];
  const total = values.reduce((a, b) => a + b, 0);
  if (total <= 0) {
    drawPlaceholderPlot(ctx, chart, rect);
    return;
  }

  // Primary axis (count). Standard auto-range with a zero floor.
  const maxCount = Math.max(...values);
  const primary = resolveAxisRange(
    0,
    maxCount,
    undefined,
    undefined,
    true,
    AXIS_TICK_COUNT,
    undefined,
  );

  // Reserve a strip on the right for the secondary-axis labels.
  ctx.font = `${AXIS_FONT_SIZE}px -apple-system, "Helvetica Neue", Arial, sans-serif`;
  const secAxisLabels = ["0%", "25%", "50%", "75%", "100%"];
  const secAxisW = Math.max(...secAxisLabels.map((s) => ctx.measureText(s).width)) + 10;
  const frameRect: Rect = { x: rect.x, y: rect.y, w: rect.w - secAxisW, h: rect.h };
  const inner = drawAxisFrame(
    ctx,
    chart,
    frameRect,
    primary.ticks,
    primary.minV,
    primary.maxV,
    false,
    false,
  );

  // Paint secondary y-axis baseline + ticks on the right.
  ctx.strokeStyle = "#9ca3af";
  ctx.beginPath();
  ctx.moveTo(Math.round(inner.x + inner.w) + 0.5, inner.y);
  ctx.lineTo(Math.round(inner.x + inner.w) + 0.5, inner.y + inner.h);
  ctx.stroke();
  ctx.fillStyle = AXIS_LABEL_COLOR;
  ctx.textAlign = "left";
  ctx.textBaseline = "middle";
  for (let ti = 0; ti < secAxisLabels.length; ti++) {
    const frac = ti / (secAxisLabels.length - 1);
    const y = inner.y + (1 - frac) * inner.h;
    ctx.fillText(secAxisLabels[ti]!, inner.x + inner.w + 4, y);
  }

  // Category labels along the x-axis.
  const slotW = inner.w / n;
  ctx.textAlign = "center";
  ctx.textBaseline = "top";
  ctx.fillStyle = AXIS_LABEL_COLOR;
  let lastRight = -Infinity;
  for (let i = 0; i < n; i++) {
    const label = cats[i] ?? `${i + 1}`;
    const w = ctx.measureText(label).width;
    const cx = inner.x + (i + 0.5) * slotW;
    if (cx - w / 2 < lastRight + 6) continue;
    ctx.fillText(label, cx, inner.y + inner.h + 4);
    lastRight = cx + w / 2;
  }

  // Bars (touching: pareto gapWidth=0 in the fixture).
  const barColor = activeThemeColor(4, "#4472C4");
  const lineColor = activeThemeColor(5, "#ED7D31");
  ctx.fillStyle = barColor;
  for (let i = 0; i < n; i++) {
    const v = values[i]!;
    if (v <= 0) continue;
    const yTop = inner.y + (1 - (v - primary.minV) / (primary.maxV - primary.minV)) * inner.h;
    const yBot = inner.y + inner.h;
    const x = inner.x + i * slotW;
    ctx.fillStyle = barColor;
    ctx.fillRect(x, yTop, slotW, yBot - yTop);
    ctx.strokeStyle = "#ffffff";
    ctx.lineWidth = 1;
    ctx.strokeRect(x + 0.5, yTop + 0.5, slotW - 1, yBot - yTop - 1);
  }

  // Cumulative-% line: each point sits at the right edge of its bar
  // at y = cumulative/total on the secondary scale. The first point
  // anchors at the *left* edge of the first bar (cumulative = 0) so
  // the line visually starts from the axis baseline, matching Excel.
  ctx.strokeStyle = lineColor;
  ctx.lineWidth = 2;
  ctx.beginPath();
  let cum = 0;
  const yForPct = (p: number) => inner.y + (1 - p) * inner.h;
  ctx.moveTo(inner.x, yForPct(0));
  for (let i = 0; i < n; i++) {
    cum += values[i]!;
    const x = inner.x + (i + 1) * slotW;
    ctx.lineTo(x, yForPct(cum / total));
  }
  ctx.stroke();
  // Markers on each cumulative point.
  cum = 0;
  ctx.fillStyle = lineColor;
  for (let i = 0; i < n; i++) {
    cum += values[i]!;
    const x = inner.x + (i + 1) * slotW;
    const y = yForPct(cum / total);
    ctx.beginPath();
    ctx.arc(x, y, 3, 0, Math.PI * 2);
    ctx.fill();
  }
}

// ---------- box & whisker ----------
//
// One vertical box-and-whisker per series. Quartiles are computed at
// draw time per `<cx:statistics quartileMethod="exclusive"/>` (the
// default Excel emits). Outliers are points outside 1.5 × IQR of the
// hinge; whiskers extend to the most extreme non-outlier values. The
// mean marker (default-on in chartEx) is painted as an ×.
export function drawBoxWhiskerChartEx(
  ctx: CanvasRenderingContext2D,
  chart: Chart,
  rect: Rect,
): void {
  const serieses = chart.series.filter((s) => s.values.length > 0);
  if (serieses.length === 0) {
    drawPlaceholderPlot(ctx, chart, rect);
    return;
  }
  // Compute stats per series.
  const stats = serieses.map((s) => computeBoxStats(s.values));
  // Value-axis range covers every observed value (including outliers).
  let minObs = Infinity;
  let maxObs = -Infinity;
  for (const s of serieses) {
    for (const v of s.values) {
      if (!Number.isFinite(v)) continue;
      if (v < minObs) minObs = v;
      if (v > maxObs) maxObs = v;
    }
  }
  if (!Number.isFinite(minObs) || !Number.isFinite(maxObs)) {
    drawPlaceholderPlot(ctx, chart, rect);
    return;
  }
  const range = resolveAxisRange(
    minObs,
    maxObs,
    chart.valueMin,
    chart.valueMax,
    /*zeroClamp=*/ false,
    AXIS_TICK_COUNT,
    chart.majorUnit,
  );
  const inner = drawAxisFrame(ctx, chart, rect, range.ticks, range.minV, range.maxV, false, false);

  // Category labels = series names.
  const slotW = inner.w / serieses.length;
  ctx.font = `${AXIS_FONT_SIZE}px -apple-system, "Helvetica Neue", Arial, sans-serif`;
  ctx.fillStyle = AXIS_LABEL_COLOR;
  ctx.textAlign = "center";
  ctx.textBaseline = "top";
  for (let i = 0; i < serieses.length; i++) {
    const name = serieses[i]!.name || `Series ${i + 1}`;
    const cx = inner.x + (i + 0.5) * slotW;
    ctx.fillText(name, cx, inner.y + inner.h + 4);
  }

  const yFor = (v: number) =>
    inner.y + (1 - (v - range.minV) / (range.maxV - range.minV)) * inner.h;
  const boxW = Math.max(8, slotW * 0.6);

  for (let i = 0; i < serieses.length; i++) {
    const st = stats[i]!;
    const accent = activeThemeColor(4 + (i % 6), DEFAULT_PIE_ACCENTS[i % 6]!);
    const cx = inner.x + (i + 0.5) * slotW;
    const xLeft = cx - boxW / 2;

    // Whiskers: vertical line from low to high.
    ctx.strokeStyle = "#333333";
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(Math.round(cx) + 0.5, yFor(st.whiskerLow));
    ctx.lineTo(Math.round(cx) + 0.5, yFor(st.whiskerHigh));
    ctx.stroke();
    // Whisker caps.
    const capW = Math.max(4, boxW * 0.4);
    ctx.beginPath();
    ctx.moveTo(cx - capW / 2, yFor(st.whiskerLow));
    ctx.lineTo(cx + capW / 2, yFor(st.whiskerLow));
    ctx.moveTo(cx - capW / 2, yFor(st.whiskerHigh));
    ctx.lineTo(cx + capW / 2, yFor(st.whiskerHigh));
    ctx.stroke();

    // Box: Q1 -> Q3.
    const yQ1 = yFor(st.q1);
    const yQ3 = yFor(st.q3);
    const yTop = Math.min(yQ1, yQ3);
    const yBot = Math.max(yQ1, yQ3);
    ctx.fillStyle = accent;
    ctx.fillRect(xLeft, yTop, boxW, Math.max(2, yBot - yTop));
    ctx.strokeStyle = "#333333";
    ctx.lineWidth = 1;
    ctx.strokeRect(xLeft + 0.5, yTop + 0.5, boxW - 1, Math.max(2, yBot - yTop) - 1);

    // Median line.
    const yMed = yFor(st.median);
    ctx.beginPath();
    ctx.moveTo(xLeft, Math.round(yMed) + 0.5);
    ctx.lineTo(xLeft + boxW, Math.round(yMed) + 0.5);
    ctx.lineWidth = 2;
    ctx.stroke();

    // Mean marker (× at the mean). Default-on for Excel boxWhisker.
    const yMean = yFor(st.mean);
    ctx.lineWidth = 1.5;
    ctx.strokeStyle = "#111827";
    ctx.beginPath();
    const r = 4;
    ctx.moveTo(cx - r, yMean - r);
    ctx.lineTo(cx + r, yMean + r);
    ctx.moveTo(cx - r, yMean + r);
    ctx.lineTo(cx + r, yMean - r);
    ctx.stroke();

    // Outliers as small filled dots.
    ctx.fillStyle = "#111827";
    for (const o of st.outliers) {
      ctx.beginPath();
      ctx.arc(cx, yFor(o), 2.2, 0, Math.PI * 2);
      ctx.fill();
    }
  }
}

interface BoxStats {
  q1: number;
  median: number;
  q3: number;
  whiskerLow: number;
  whiskerHigh: number;
  mean: number;
  outliers: number[];
}

/// Quartile method = "exclusive" (Excel default for chartEx
/// boxWhisker; QUARTILE.EXC semantics). Position p in [0,1] is
/// interpolated at index `p * (N + 1) - 1` in the sorted observations.
/// Whiskers extend to the most extreme observation within 1.5 × IQR
/// of the hinge; everything beyond is an outlier.
function computeBoxStats(raw: number[]): BoxStats {
  const sorted = raw.filter((v) => Number.isFinite(v)).sort((a, b) => a - b);
  const n = sorted.length;
  const quant = (p: number): number => {
    if (n === 0) return 0;
    const pos = p * (n + 1) - 1;
    if (pos <= 0) return sorted[0]!;
    if (pos >= n - 1) return sorted[n - 1]!;
    const lo = Math.floor(pos);
    const hi = lo + 1;
    const frac = pos - lo;
    return sorted[lo]! + (sorted[hi]! - sorted[lo]!) * frac;
  };
  const q1 = quant(0.25);
  const median = quant(0.5);
  const q3 = quant(0.75);
  const iqr = q3 - q1;
  const lowFence = q1 - 1.5 * iqr;
  const highFence = q3 + 1.5 * iqr;
  const inFence = sorted.filter((v) => v >= lowFence && v <= highFence);
  const whiskerLow = inFence.length > 0 ? inFence[0]! : q1;
  const whiskerHigh = inFence.length > 0 ? inFence[inFence.length - 1]! : q3;
  const outliers = sorted.filter((v) => v < lowFence || v > highFence);
  const mean = n > 0 ? sorted.reduce((a, b) => a + b, 0) / n : 0;
  return { q1, median, q3, whiskerLow, whiskerHigh, mean, outliers };
}
