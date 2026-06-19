import type { Chart } from "./types.js";
import type { Rect } from "./chart.js";
import { activeThemeColor } from "./color.js";
import { DEFAULT_PIE_ACCENTS } from "./chartAdvanced.js";
import {
  drawAxisFrame,
  drawPlaceholderPlot,
  resolveAxisRange,
  AXIS_FONT_SIZE,
} from "./chartUtils.js";

const AXIS_LABEL_COLOR = "#52525b";
const AXIS_TICK_COUNT = 5;

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

  const sturges = Math.max(2, Math.ceil(Math.log2(Math.max(2, n)) + 1));
  let binWidth = (maxV - minV) / sturges;
  if (!Number.isFinite(binWidth) || binWidth <= 0) {
    binWidth = Math.max(1, (maxV - minV) / Math.max(1, Math.ceil(Math.sqrt(n))));
  }
  binWidth = niceBinWidth(binWidth);

  const startEdge = Math.floor(minV / binWidth) * binWidth;
  const endEdge = Math.ceil(maxV / binWidth) * binWidth;
  const binCount = Math.max(1, Math.round((endEdge - startEdge) / binWidth));
  const edges: number[] = [];
  for (let i = 0; i <= binCount; i++)
    edges.push(parseFloat((startEdge + i * binWidth).toPrecision(12)));

  const counts = new Array<number>(binCount).fill(0);
  for (const v of obs) {
    let idx = Math.floor((v - startEdge) / binWidth);
    if (v === edges[idx]! && idx > 0) idx -= 1;
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
    AXIS_TICK_COUNT,
    undefined,
  );
  const inner = drawAxisFrame(ctx, chart, rect, range.ticks, range.minV, range.maxV, false, false);

  const slotW = inner.w / binCount;
  ctx.font = `${AXIS_FONT_SIZE}px -apple-system, "Helvetica Neue", Arial, sans-serif`;
  ctx.fillStyle = AXIS_LABEL_COLOR;
  ctx.textAlign = "center";
  ctx.textBaseline = "top";
  let lastRight = -Infinity;
  for (let i = 0; i < binCount; i++) {
    const lo = edges[i]!;
    const hi = edges[i + 1]!;

    const label =
      i === 0 ? `[${fmtBinEdge(lo)}, ${fmtBinEdge(hi)}]` : `(${fmtBinEdge(lo)}, ${fmtBinEdge(hi)}]`;
    const tw = ctx.measureText(label).width;
    const cx = inner.x + (i + 0.5) * slotW;
    if (cx - tw / 2 < lastRight + 4) continue;
    ctx.fillText(label, cx, inner.y + inner.h + 4);
    lastRight = cx + tw / 2;
  }

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

function fmtBinEdge(v: number): string {
  if (Number.isInteger(v)) return v.toString();
  return v.toFixed(2).replace(/0+$/, "").replace(/\.$/, "");
}

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

  const maxCount = Math.max(...values);
  const primary = resolveAxisRange(
    0,
    maxCount,
    undefined,
    undefined,
    AXIS_TICK_COUNT,
    undefined,
  );

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

  const stats = serieses.map((s) => computeBoxStats(s.values));

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
    AXIS_TICK_COUNT,
    chart.majorUnit,
  );
  const inner = drawAxisFrame(ctx, chart, rect, range.ticks, range.minV, range.maxV, false, false);

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

    ctx.strokeStyle = "#333333";
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(Math.round(cx) + 0.5, yFor(st.whiskerLow));
    ctx.lineTo(Math.round(cx) + 0.5, yFor(st.whiskerHigh));
    ctx.stroke();

    const capW = Math.max(4, boxW * 0.4);
    ctx.beginPath();
    ctx.moveTo(cx - capW / 2, yFor(st.whiskerLow));
    ctx.lineTo(cx + capW / 2, yFor(st.whiskerLow));
    ctx.moveTo(cx - capW / 2, yFor(st.whiskerHigh));
    ctx.lineTo(cx + capW / 2, yFor(st.whiskerHigh));
    ctx.stroke();

    const yQ1 = yFor(st.q1);
    const yQ3 = yFor(st.q3);
    const yTop = Math.min(yQ1, yQ3);
    const yBot = Math.max(yQ1, yQ3);
    ctx.fillStyle = accent;
    ctx.fillRect(xLeft, yTop, boxW, Math.max(2, yBot - yTop));
    ctx.strokeStyle = "#333333";
    ctx.lineWidth = 1;
    ctx.strokeRect(xLeft + 0.5, yTop + 0.5, boxW - 1, Math.max(2, yBot - yTop) - 1);

    const yMed = yFor(st.median);
    ctx.beginPath();
    ctx.moveTo(xLeft, Math.round(yMed) + 0.5);
    ctx.lineTo(xLeft + boxW, Math.round(yMed) + 0.5);
    ctx.lineWidth = 2;
    ctx.stroke();

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
