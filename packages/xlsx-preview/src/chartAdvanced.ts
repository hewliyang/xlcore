import type { Chart, ChartSeries } from "./types.js";
import type { Rect } from "./chart.js";
import { activeThemeColor } from "./color.js";
import {
  buildLabelText,
  drawAxisFrame,
  drawLabel,
  drawPlaceholderPlot,
  effectiveLabels,
  formatAxisValue,
  formatGeneral,
  niceTicks,
  paintZeroBaseline,
  pointLabel,
  resolveAxisRange,
  valueRange,
  withAlpha,
} from "./chartUtils.js";

const AXIS_FONT_SIZE = 10;
const AXIS_LABEL_COLOR = "#52525b";
const AXIS_TICK_COUNT = 5;

export const DEFAULT_PIE_ACCENTS = [
  "#4472C4",
  "#ED7D31",
  "#A5A5A5",
  "#FFC000",
  "#5B9BD5",
  "#70AD47",
];

export function pieSliceColor(index: number, pointColors: readonly (string | undefined)[]): string {
  const explicit = pointColors[index];

  if (explicit && explicit.length > 0 && explicit !== "none") return explicit;
  const accentIndex = 4 + (index % 6);
  return activeThemeColor(accentIndex, DEFAULT_PIE_ACCENTS[index % DEFAULT_PIE_ACCENTS.length]!);
}

export function resolveBarFill(
  s: ChartSeries,
  i: number,
): { skip: true } | { skip: false; color: string } {
  const override = s.pointColors?.[i];
  if (override === "none") return { skip: true };
  if (override && override.length > 0) return { skip: false, color: override };
  return { skip: false, color: s.color ?? "#4472C4" };
}

export function drawPieChart(ctx: CanvasRenderingContext2D, chart: Chart, rect: Rect): void {
  const ser = chart.series[0];
  if (!ser || ser.values.length === 0) {
    drawPlaceholderPlot(ctx, chart, rect);
    return;
  }

  const total = ser.values.reduce((a, b) => a + Math.max(0, b), 0);
  if (total <= 0) {
    drawPlaceholderPlot(ctx, chart, rect);
    return;
  }

  const cx = rect.x + rect.w / 2;
  const cy = rect.y + rect.h / 2;
  const r = Math.min(rect.w, rect.h) / 2 - 8;
  const holeFrac =
    chart.holeSize != null ? Math.max(0, Math.min(90, chart.holeSize)) / 100 : 0.55;
  const innerR = chart.type === "doughnut" ? r * holeFrac : 0;

  const pointColors = ser.pointColors ?? [];

  type SliceGeom = { mid: number; idx: number; v: number };
  const slices: SliceGeom[] = [];
  let start = -Math.PI / 2 + ((chart.firstSliceAngle ?? 0) * Math.PI) / 180;
  for (let i = 0; i < ser.values.length; i++) {
    const v = Math.max(0, ser.values[i] ?? 0);
    if (v <= 0) continue;
    const sweep = (v / total) * Math.PI * 2;
    const end = start + sweep;
    ctx.fillStyle = pieSliceColor(i, pointColors);
    ctx.beginPath();
    ctx.moveTo(cx, cy);
    ctx.arc(cx, cy, r, start, end);
    ctx.closePath();
    ctx.fill();

    ctx.strokeStyle = "#ffffff";
    ctx.lineWidth = 1.5;
    ctx.stroke();
    slices.push({ mid: (start + end) / 2, idx: i, v });
    start = end;
  }

  if (innerR > 0) {
    ctx.fillStyle = "#ffffff";
    ctx.beginPath();
    ctx.arc(cx, cy, innerR, 0, Math.PI * 2);
    ctx.fill();
  }
  ctx.lineWidth = 1;

  const dl = effectiveLabels(chart, ser);
  if (dl) {
    for (const sl of slices) {
      const po = pointLabel(dl, sl.idx);
      if (po === null) continue;
      const edl = po?.dl ?? dl;
      const pos = edl.position ?? "outEnd";
      const labelR =
        pos === "outEnd" || pos === "bestFit" ? r + 12 : pos === "ctr" ? (innerR + r) / 2 : r - 12;
      const text = po?.text ?? buildLabelText(edl, chart, ser, sl.idx, sl.v, total);
      if (!text) continue;
      const lx = cx + Math.cos(sl.mid) * labelR;
      const ly = cy + Math.sin(sl.mid) * labelR;
      const align: CanvasTextAlign =
        pos === "outEnd" || pos === "bestFit"
          ? Math.cos(sl.mid) >= 0
            ? "left"
            : "right"
          : "center";
      drawLabel(ctx, text, lx, ly, align, "middle");
    }
  }
}

export function drawScatterChart(ctx: CanvasRenderingContext2D, chart: Chart, rect: Rect): void {
  const series = chart.series.filter((s) => s.values.length > 0);
  if (series.length === 0) {
    drawPlaceholderPlot(ctx, chart, rect);
    return;
  }

  const xCache: number[][] = series.map((s) => {
    const xs = (s.xValues ?? []) as number[];
    if (xs.length > 0) return xs.slice();

    return s.values.map((_, i) => {
      const c = (chart.categories ?? [])[i];
      const n = c == null ? i + 1 : parseFloat(c);
      return Number.isFinite(n) ? n : i + 1;
    });
  });

  let xMin = Infinity,
    xMax = -Infinity;
  let yMin = Infinity,
    yMax = -Infinity;
  for (let si = 0; si < series.length; si++) {
    const xs = xCache[si]!;
    const ys = series[si]!.values;
    const n = Math.min(xs.length, ys.length);
    for (let i = 0; i < n; i++) {
      const x = xs[i]!,
        y = ys[i]!;
      if (x < xMin) xMin = x;
      if (x > xMax) xMax = x;
      if (y < yMin) yMin = y;
      if (y > yMax) yMax = y;
    }
  }
  if (!Number.isFinite(xMin) || !Number.isFinite(yMin)) {
    drawPlaceholderPlot(ctx, chart, rect);
    return;
  }
  if (xMin === xMax) {
    xMax = xMin + 1;
  }
  if (yMin === yMax) {
    yMax = yMin + 1;
  }
  const xTicks = niceTicks(xMin, xMax, AXIS_TICK_COUNT);
  const yTicks = niceTicks(yMin, yMax, AXIS_TICK_COUNT);
  xMin = xTicks[0]!;
  xMax = xTicks[xTicks.length - 1]!;
  yMin = yTicks[0]!;
  yMax = yTicks[yTicks.length - 1]!;

  const inner = drawAxisFrame(ctx, chart, rect, yTicks, yMin, yMax, false, false);

  ctx.font = `${AXIS_FONT_SIZE}px -apple-system, "Helvetica Neue", Arial, sans-serif`;
  ctx.fillStyle = AXIS_LABEL_COLOR;
  ctx.textAlign = "center";
  ctx.textBaseline = "top";
  for (const t of xTicks) {
    const frac = (t - xMin) / (xMax - xMin);
    const x = inner.x + frac * inner.w;
    ctx.fillText(formatGeneral(t), x, inner.y + inner.h + 4);
  }

  const style = chart.scatterStyle;
  const drawLines = style === "line" || style === "lineMarker";
  const drawSmooth = style === "smooth" || style === "smoothMarker";
  const drawMarkers =
    style == null || style === "marker" || style === "lineMarker" || style === "smoothMarker";

  for (let si = 0; si < series.length; si++) {
    const s = series[si]!;
    const xs = xCache[si]!;
    const ys = s.values;
    const n = Math.min(xs.length, ys.length);
    if (n === 0) continue;
    const color = s.color ?? "#4472C4";
    ctx.fillStyle = color;
    ctx.strokeStyle = color;
    const dl = effectiveLabels(chart, s);

    const pts: { x: number; y: number; v: number; i: number }[] = [];
    for (let i = 0; i < n; i++) {
      const px = inner.x + ((xs[i]! - xMin) / (xMax - xMin)) * inner.w;
      const py = inner.y + (1 - (ys[i]! - yMin) / (yMax - yMin)) * inner.h;
      pts.push({ x: px, y: py, v: ys[i]!, i });
    }

    if ((drawLines || drawSmooth) && pts.length >= 2) {
      const sorted = pts.slice().sort((a, b) => a.x - b.x);
      ctx.lineWidth = 1.5;
      ctx.beginPath();
      ctx.moveTo(sorted[0]!.x, sorted[0]!.y);
      if (drawSmooth) {
        for (let k = 0; k < sorted.length - 1; k++) {
          const p0 = sorted[Math.max(0, k - 1)]!;
          const p1 = sorted[k]!;
          const p2 = sorted[k + 1]!;
          const p3 = sorted[Math.min(sorted.length - 1, k + 2)]!;
          const cp1x = p1.x + (p2.x - p0.x) / 6;
          const cp1y = p1.y + (p2.y - p0.y) / 6;
          const cp2x = p2.x - (p3.x - p1.x) / 6;
          const cp2y = p2.y - (p3.y - p1.y) / 6;
          ctx.bezierCurveTo(cp1x, cp1y, cp2x, cp2y, p2.x, p2.y);
        }
      } else {
        for (let k = 1; k < sorted.length; k++) {
          ctx.lineTo(sorted[k]!.x, sorted[k]!.y);
        }
      }
      ctx.stroke();
    }

    const noMarker = s.markerSymbol === "none";
    for (const p of pts) {
      if (drawMarkers && !noMarker) {
        ctx.beginPath();
        ctx.arc(p.x, p.y, 3.5, 0, Math.PI * 2);
        ctx.fill();
      }
      if (dl) {
        const po = pointLabel(dl, p.i);
        if (po !== null) {
          const edl = po?.dl ?? dl;
          const text = po?.text ?? buildLabelText(edl, chart, s, p.i, p.v, 0);
          if (text) drawLabel(ctx, text, p.x, p.y - 6, "center", "bottom");
        }
      }
    }
  }

  paintZeroBaseline(ctx, inner, yMin, yMax);
}

export function drawBubbleChart(ctx: CanvasRenderingContext2D, chart: Chart, rect: Rect): void {
  const series = chart.series.filter((s) => s.values.length > 0);
  if (series.length === 0) {
    drawPlaceholderPlot(ctx, chart, rect);
    return;
  }

  const xCache: number[][] = series.map((s) => {
    const xs = (s.xValues ?? []) as number[];
    if (xs.length > 0) return xs.slice();
    return s.values.map((_, i) => i + 1);
  });

  const sCache: number[][] = series.map((s) => {
    const bs = (s.bubbleSizes ?? []) as number[];
    if (bs.length > 0) return bs.slice();
    return s.values.map(() => 1);
  });

  let xMin = Infinity,
    xMax = -Infinity;
  let yMin = Infinity,
    yMax = -Infinity;
  let maxSize = -Infinity;
  for (let si = 0; si < series.length; si++) {
    const xs = xCache[si]!;
    const ys = series[si]!.values;
    const sz = sCache[si]!;
    const n = Math.min(xs.length, ys.length);
    for (let i = 0; i < n; i++) {
      const x = xs[i]!,
        y = ys[i]!;
      if (x < xMin) xMin = x;
      if (x > xMax) xMax = x;
      if (y < yMin) yMin = y;
      if (y > yMax) yMax = y;
      const s = sz[i] ?? 0;
      if (Number.isFinite(s) && s > maxSize) maxSize = s;
    }
  }
  if (!Number.isFinite(xMin) || !Number.isFinite(yMin)) {
    drawPlaceholderPlot(ctx, chart, rect);
    return;
  }
  if (xMin === xMax) xMax = xMin + 1;
  if (yMin === yMax) yMax = yMin + 1;
  if (!Number.isFinite(maxSize) || maxSize <= 0) maxSize = 1;
  const xTicks = niceTicks(xMin, xMax, AXIS_TICK_COUNT);
  const yTicks = niceTicks(yMin, yMax, AXIS_TICK_COUNT);
  xMin = xTicks[0]!;
  xMax = xTicks[xTicks.length - 1]!;
  yMin = yTicks[0]!;
  yMax = yTicks[yTicks.length - 1]!;

  const inner = drawAxisFrame(ctx, chart, rect, yTicks, yMin, yMax, false, false);

  ctx.font = `${AXIS_FONT_SIZE}px -apple-system, "Helvetica Neue", Arial, sans-serif`;
  ctx.fillStyle = AXIS_LABEL_COLOR;
  ctx.textAlign = "center";
  ctx.textBaseline = "top";
  for (const t of xTicks) {
    const frac = (t - xMin) / (xMax - xMin);
    const x = inner.x + frac * inner.w;
    ctx.fillText(formatGeneral(t), x, inner.y + inner.h + 4);
  }

  const scalePct = chart.bubbleScale ?? 100;
  const baseR = Math.min(inner.w, inner.h) * 0.11 * (scalePct / 100);

  const minR = 2;

  const byArea = (chart.sizeRepresents ?? "area") !== "w";

  for (let si = 0; si < series.length; si++) {
    const s = series[si]!;
    const xs = xCache[si]!;
    const ys = s.values;
    const sz = sCache[si]!;
    const n = Math.min(xs.length, ys.length);
    if (n === 0) continue;
    const color = s.color ?? "#4472C4";
    const dl = effectiveLabels(chart, s);

    for (let i = 0; i < n; i++) {
      const xv = xs[i]!,
        yv = ys[i]!;
      const bs = sz[i] ?? 0;

      if (!Number.isFinite(bs) || bs <= 0) continue;
      const frac = bs / maxSize;
      const r = Math.max(minR, baseR * (byArea ? Math.sqrt(frac) : frac));
      const px = inner.x + ((xv - xMin) / (xMax - xMin)) * inner.w;
      const py = inner.y + (1 - (yv - yMin) / (yMax - yMin)) * inner.h;

      ctx.beginPath();
      ctx.arc(px, py, r, 0, Math.PI * 2);

      ctx.fillStyle = withAlpha(color, 0.6);
      ctx.fill();
      ctx.strokeStyle = color;
      ctx.lineWidth = 1;
      ctx.stroke();

      if (dl) {
        const po = pointLabel(dl, i);
        if (po !== null) {
          const edl = po?.dl ?? dl;
          const text = po?.text ?? buildLabelText(edl, chart, s, i, yv, 0);
          if (text) drawLabel(ctx, text, px, py, "center", "middle");
        }
      }
    }
  }
  paintZeroBaseline(ctx, inner, yMin, yMax);
}

export function drawRadarChart(ctx: CanvasRenderingContext2D, chart: Chart, rect: Rect): void {
  const series = chart.series.filter((s) => s.values.length > 0);
  if (series.length === 0) {
    drawPlaceholderPlot(ctx, chart, rect);
    return;
  }
  const cats = chart.categories ?? [];
  const categoryCount = Math.max(...series.map((s) => s.values.length), cats.length);
  if (categoryCount < 3) {
    drawPlaceholderPlot(ctx, chart, rect);
    return;
  }

  ctx.font = `${AXIS_FONT_SIZE}px -apple-system, "Helvetica Neue", Arial, sans-serif`;
  let maxLabelW = 0;
  for (let i = 0; i < categoryCount; i++) {
    const t = cats[i] ?? `${i + 1}`;
    maxLabelW = Math.max(maxLabelW, ctx.measureText(t).width);
  }
  const labelPad = 8;
  const inset = Math.min(rect.w, rect.h) * 0.08 + Math.max(maxLabelW / 2, AXIS_FONT_SIZE);
  const cx = rect.x + rect.w / 2;
  const cy = rect.y + rect.h / 2;
  const R = Math.max(20, Math.min(rect.w, rect.h) / 2 - inset);

  const rows = series.map((s) => Array.from({ length: categoryCount }, (_, i) => s.values[i] ?? 0));
  let { minV, maxV } = valueRange(rows);
  const range = resolveAxisRange(
    minV,
    maxV,
    chart.valueMin,
    chart.valueMax,
    false,
    AXIS_TICK_COUNT,
    chart.majorUnit,
  );
  minV = range.minV;
  maxV = range.maxV;
  const ticks = range.ticks;

  const angleFor = (i: number) => -Math.PI / 2 + (i / categoryCount) * Math.PI * 2;
  const radiusFor = (v: number) => {
    if (!Number.isFinite(v)) return 0;
    const span = maxV - minV;
    if (span <= 0) return 0;
    return Math.max(0, ((v - minV) / span) * R);
  };

  ctx.strokeStyle = "#e5e7eb";
  ctx.lineWidth = 1;
  for (let i = 0; i < categoryCount; i++) {
    const a = angleFor(i);
    ctx.beginPath();
    ctx.moveTo(cx, cy);
    ctx.lineTo(cx + Math.cos(a) * R, cy + Math.sin(a) * R);
    ctx.stroke();
  }

  ctx.strokeStyle = "#e5e7eb";
  for (const t of ticks) {
    const r = radiusFor(t);
    if (r <= 0.5) continue;
    ctx.beginPath();
    for (let i = 0; i < categoryCount; i++) {
      const a = angleFor(i);
      const x = cx + Math.cos(a) * r;
      const y = cy + Math.sin(a) * r;
      if (i === 0) ctx.moveTo(x, y);
      else ctx.lineTo(x, y);
    }
    ctx.closePath();
    ctx.stroke();
  }

  ctx.fillStyle = AXIS_LABEL_COLOR;
  ctx.font = `${AXIS_FONT_SIZE}px -apple-system, "Helvetica Neue", Arial, sans-serif`;
  for (let i = 0; i < categoryCount; i++) {
    const a = angleFor(i);
    const lx = cx + Math.cos(a) * (R + labelPad);
    const ly = cy + Math.sin(a) * (R + labelPad);

    const TOL = 0.05;
    let align: CanvasTextAlign = "center";
    if (Math.cos(a) > TOL) align = "left";
    else if (Math.cos(a) < -TOL) align = "right";
    let baseline: CanvasTextBaseline = "middle";
    if (Math.sin(a) < -TOL) baseline = "bottom";
    else if (Math.sin(a) > TOL) baseline = "top";
    ctx.textAlign = align;
    ctx.textBaseline = baseline;
    ctx.fillText(cats[i] ?? `${i + 1}`, lx, ly);
  }

  ctx.fillStyle = AXIS_LABEL_COLOR;
  ctx.textAlign = "right";
  ctx.textBaseline = "middle";
  for (let ti = 0; ti < ticks.length; ti++) {
    if (ti === 0) continue;
    const t = ticks[ti]!;
    const r = radiusFor(t);
    if (r <= 0.5) continue;
    const text = formatAxisValue(t, chart.valueFormat, chart.dispUnits);
    ctx.fillText(text, cx - 3, cy - r);
  }

  const filled = chart.radarStyle === "filled";
  const showMarkers = chart.radarStyle !== "standard";
  for (let si = 0; si < series.length; si++) {
    const s = series[si]!;
    const color = s.color ?? "#4472C4";
    const data = rows[si]!;

    const allFinite = data.every((v) => Number.isFinite(v));

    ctx.beginPath();
    let started = false;
    for (let i = 0; i < categoryCount; i++) {
      const v = data[i] ?? 0;
      if (!Number.isFinite(v)) {
        started = false;
        continue;
      }
      const a = angleFor(i);
      const r = radiusFor(v);
      const x = cx + Math.cos(a) * r;
      const y = cy + Math.sin(a) * r;
      if (!started) {
        ctx.moveTo(x, y);
        started = true;
      } else {
        ctx.lineTo(x, y);
      }
    }
    if (allFinite) ctx.closePath();

    if (filled) {
      ctx.fillStyle = withAlpha(color, 0.45);
      ctx.fill();
    }
    ctx.strokeStyle = color;
    ctx.lineWidth = filled ? 1.25 : 2;
    ctx.stroke();

    if (showMarkers && s.markerSymbol !== "none") {
      ctx.fillStyle = color;
      for (let i = 0; i < categoryCount; i++) {
        const v = data[i] ?? 0;
        if (!Number.isFinite(v)) continue;
        const a = angleFor(i);
        const r = radiusFor(v);
        ctx.beginPath();
        ctx.arc(cx + Math.cos(a) * r, cy + Math.sin(a) * r, 3, 0, Math.PI * 2);
        ctx.fill();
      }
    }

    const dl = effectiveLabels(chart, s);
    if (dl) {
      const PAD = 6;
      for (let i = 0; i < categoryCount; i++) {
        const v = s.values[i];
        if (v == null || !Number.isFinite(v)) continue;
        const po = pointLabel(dl, i);
        if (po === null) continue;
        const edl = po?.dl ?? dl;
        const text = po?.text ?? buildLabelText(edl, chart, s, i, v, 0);
        if (!text) continue;
        const a = angleFor(i);
        const r = radiusFor(v);

        const lx = cx + Math.cos(a) * (r + PAD);
        const ly = cy + Math.sin(a) * (r + PAD);
        const TOL = 0.05;
        let align: CanvasTextAlign = "center";
        if (Math.cos(a) > TOL) align = "left";
        else if (Math.cos(a) < -TOL) align = "right";

        let baseline: CanvasTextBaseline = "middle";
        if (Math.sin(a) < -TOL) baseline = "bottom";
        else if (Math.sin(a) > TOL) baseline = "top";
        drawLabel(ctx, text, lx, ly, align, baseline);
      }
    }
  }

  void drawAxisFrame;
  void niceTicks;
  void formatGeneral;
  void paintZeroBaseline;
}

export { drawStockChart } from "./chartStock.js";

export { drawComboChart } from "./chartCombo.js";

export { drawChartEx, waterfallLegendEntries } from "./chartEx.js";
