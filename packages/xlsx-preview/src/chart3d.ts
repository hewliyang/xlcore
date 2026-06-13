import type { Chart, ChartSeries } from "./types.js";
import type { Rect } from "./chart.js";
import { applyTint } from "./color.js";
import { resolveBarFill } from "./chartAdvanced.js";
import { drawPlaceholderPlot, formatAxisValue, resolveAxisRange } from "./chartUtils.js";

const AXIS_FONT_SIZE = 10;
const AXIS_TICK_COUNT = 5;
const GRIDLINE_COLOR = "#e5e7eb";
const AXIS_LABEL_COLOR = "#52525b";

interface Depth {
  dx: number;
  dy: number;
}

function normHex(c: string | undefined, fallback: string): string {
  if (!c || c === "none") return fallback;
  if (c.startsWith("#")) return c;
  if (/^[0-9a-fA-F]{6}$/.test(c)) return `#${c}`;
  if (/^[0-9a-fA-F]{8}$/.test(c)) return `#${c.slice(2)}`;
  return fallback;
}

function depthVector(chart: Chart, plot: Rect): Depth {
  const v = chart.view3d ?? {};
  const rotX = v.rotX ?? 15;
  const rotY = v.rotY ?? 20;
  const base = Math.min(plot.w, plot.h) * 0.26;
  const depthScale = Math.min(1.6, Math.max(0.4, (v.depthPercent ?? 100) / 100));
  const gapScale = Math.min(1.4, Math.max(0.6, 1 - (chart.gapDepth ?? 150) / 1000));
  const depth = base * depthScale * gapScale;
  const ax = (rotY * Math.PI) / 180;
  const ay = (rotX * Math.PI) / 180;
  return {
    dx: Math.max(6, depth * Math.sin(ax)),
    dy: Math.max(6, depth * Math.sin(ay)),
  };
}

function drawParallelogram(
  ctx: CanvasRenderingContext2D,
  pts: Array<[number, number]>,
  fill: string,
  stroke: string,
): void {
  ctx.beginPath();
  ctx.moveTo(pts[0]![0], pts[0]![1]);
  for (let i = 1; i < pts.length; i++) ctx.lineTo(pts[i]![0], pts[i]![1]);
  ctx.closePath();
  ctx.fillStyle = fill;
  ctx.fill();
  ctx.strokeStyle = stroke;
  ctx.lineWidth = 1;
  ctx.stroke();
}

function drawBox(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  w: number,
  h: number,
  d: Depth,
  base: string,
): void {
  const front = normHex(base, "#4472C4");
  const top = applyTint(front, 0.28);
  const side = applyTint(front, -0.28);
  const edge = applyTint(front, -0.45);
  drawParallelogram(
    ctx,
    [
      [x, y],
      [x + w, y],
      [x + w + d.dx, y - d.dy],
      [x + d.dx, y - d.dy],
    ],
    top,
    edge,
  );
  drawParallelogram(
    ctx,
    [
      [x + w, y],
      [x + w + d.dx, y - d.dy],
      [x + w + d.dx, y + h - d.dy],
      [x + w, y + h],
    ],
    side,
    edge,
  );
  drawParallelogram(
    ctx,
    [
      [x, y],
      [x + w, y],
      [x + w, y + h],
      [x, y + h],
    ],
    front,
    edge,
  );
}

export function drawBar3DChart(ctx: CanvasRenderingContext2D, chart: Chart, rect: Rect): void {
  const horizontal = chart.type === "bar";
  const series = chart.series.filter((s) => s.values.length > 0);
  if (series.length === 0) {
    drawPlaceholderPlot(ctx, chart, rect);
    return;
  }
  const categoryCount = Math.max(
    ...series.map((s) => s.values.length),
    (chart.categories ?? []).length,
  );
  if (categoryCount === 0) return;

  let minV = Number.POSITIVE_INFINITY,
    maxV = Number.NEGATIVE_INFINITY;
  for (const s of series) {
    for (const v of s.values) {
      if (v > maxV) maxV = v;
      if (v < minV) minV = v;
    }
  }
  if (!Number.isFinite(minV)) minV = 0;
  if (!Number.isFinite(maxV)) maxV = 1;
  const range = resolveAxisRange(
    minV,
    maxV,
    chart.valueMin,
    chart.valueMax,
    true,
    AXIS_TICK_COUNT,
    chart.majorUnit,
  );
  minV = range.minV;
  maxV = range.maxV;
  const ticks = range.ticks;

  ctx.font = `${AXIS_FONT_SIZE}px -apple-system, "Helvetica Neue", Arial, sans-serif`;
  const labelStrings = ticks.map((t) => formatAxisValue(t, chart.valueFormat, chart.dispUnits));
  const catLabels = Array.from(
    { length: categoryCount },
    (_, i) => (chart.categories ?? [])[i] ?? `${i + 1}`,
  );

  const yAxisW = Math.max(...labelStrings.map((s) => ctx.measureText(s).width)) + 10;
  const xAxisH = AXIS_FONT_SIZE + 12;
  const inner: Rect = {
    x: rect.x + yAxisW,
    y: rect.y,
    w: rect.w - yAxisW,
    h: rect.h - xAxisH,
  };

  const d = depthVector(chart, inner);
  if (horizontal) {
    drawHorizontal(ctx, chart, series, inner, d, minV, maxV, ticks, labelStrings, catLabels);
  } else {
    drawVertical(ctx, chart, series, inner, d, minV, maxV, ticks, labelStrings, catLabels);
  }
}

function drawVertical(
  ctx: CanvasRenderingContext2D,
  chart: Chart,
  series: ChartSeries[],
  inner: Rect,
  d: Depth,
  minV: number,
  maxV: number,
  ticks: number[],
  labelStrings: string[],
  catLabels: string[],
): void {
  const categoryCount = catLabels.length;
  const frontTop = inner.y + d.dy + 4;
  const frontBottom = inner.y + inner.h;
  const frontLeft = inner.x;
  const frontRight = inner.x + inner.w - d.dx - 4;
  const planeH = frontBottom - frontTop;
  const span = maxV - minV || 1;
  const yFor = (v: number) => frontBottom - ((v - minV) / span) * planeH;
  const zeroY = yFor(Math.max(minV, Math.min(maxV, 0)));

  const floorFill = normHex(chart.floorFill, "#d9d9d9");
  const backFill = normHex(chart.backWallFill, "#f1f1f1");
  const sideFill = normHex(chart.sideWallFill, "#e6e6e6");
  const wallEdge = "#bdbdbd";

  drawParallelogram(
    ctx,
    [
      [frontLeft, zeroY],
      [frontRight, zeroY],
      [frontRight + d.dx, zeroY - d.dy],
      [frontLeft + d.dx, zeroY - d.dy],
    ],
    floorFill,
    wallEdge,
  );
  drawParallelogram(
    ctx,
    [
      [frontLeft + d.dx, frontBottom - d.dy],
      [frontRight + d.dx, frontBottom - d.dy],
      [frontRight + d.dx, frontTop - d.dy],
      [frontLeft + d.dx, frontTop - d.dy],
    ],
    backFill,
    wallEdge,
  );
  drawParallelogram(
    ctx,
    [
      [frontLeft, frontBottom],
      [frontLeft + d.dx, frontBottom - d.dy],
      [frontLeft + d.dx, frontTop - d.dy],
      [frontLeft, frontTop],
    ],
    sideFill,
    wallEdge,
  );

  ctx.fillStyle = AXIS_LABEL_COLOR;
  ctx.textAlign = "right";
  ctx.textBaseline = "middle";
  for (let ti = 0; ti < ticks.length; ti++) {
    const y = yFor(ticks[ti]!);
    ctx.strokeStyle = GRIDLINE_COLOR;
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(frontLeft + d.dx, y - d.dy);
    ctx.lineTo(frontRight + d.dx, y - d.dy);
    ctx.stroke();
    ctx.fillText(labelStrings[ti]!, frontLeft - 4, y);
  }

  const bandW = (frontRight - frontLeft) / categoryCount;
  const slotW = (bandW * 0.8) / series.length;
  const slotPad = bandW * 0.1;

  for (let i = 0; i < categoryCount; i++) {
    for (let si = 0; si < series.length; si++) {
      const s = series[si]!;
      const v = s.values[i] ?? 0;
      const fill = resolveBarFill(s, i);
      if (fill.skip) continue;
      const x = frontLeft + i * bandW + slotPad + si * slotW;
      const yTop = yFor(v);
      const yBase = zeroY;
      const y = Math.min(yTop, yBase);
      const h = Math.abs(yBase - yTop);
      drawBox(ctx, x, y, slotW * 0.92, h, d, fill.color);
    }
  }

  ctx.fillStyle = AXIS_LABEL_COLOR;
  ctx.textAlign = "center";
  ctx.textBaseline = "top";
  for (let i = 0; i < categoryCount; i++) {
    const cx = frontLeft + (i + 0.5) * bandW;
    ctx.fillText(catLabels[i]!, cx, frontBottom + 4);
  }
}

function drawHorizontal(
  ctx: CanvasRenderingContext2D,
  chart: Chart,
  series: ChartSeries[],
  inner: Rect,
  d: Depth,
  minV: number,
  maxV: number,
  ticks: number[],
  labelStrings: string[],
  catLabels: string[],
): void {
  const categoryCount = catLabels.length;
  const frontTop = inner.y + d.dy + 4;
  const frontBottom = inner.y + inner.h;
  const frontLeft = inner.x;
  const frontRight = inner.x + inner.w - d.dx - 4;
  const planeW = frontRight - frontLeft;
  const planeH = frontBottom - frontTop;
  const span = maxV - minV || 1;
  const xFor = (v: number) => frontLeft + ((v - minV) / span) * planeW;
  const zeroX = xFor(Math.max(minV, Math.min(maxV, 0)));

  const floorFill = normHex(chart.floorFill, "#d9d9d9");
  const backFill = normHex(chart.backWallFill, "#f1f1f1");
  const sideFill = normHex(chart.sideWallFill, "#e6e6e6");
  const wallEdge = "#bdbdbd";

  drawParallelogram(
    ctx,
    [
      [frontLeft, frontBottom],
      [frontRight, frontBottom],
      [frontRight + d.dx, frontBottom - d.dy],
      [frontLeft + d.dx, frontBottom - d.dy],
    ],
    floorFill,
    wallEdge,
  );
  drawParallelogram(
    ctx,
    [
      [frontLeft + d.dx, frontBottom - d.dy],
      [frontRight + d.dx, frontBottom - d.dy],
      [frontRight + d.dx, frontTop - d.dy],
      [frontLeft + d.dx, frontTop - d.dy],
    ],
    backFill,
    wallEdge,
  );
  drawParallelogram(
    ctx,
    [
      [frontLeft, frontBottom],
      [frontLeft + d.dx, frontBottom - d.dy],
      [frontLeft + d.dx, frontTop - d.dy],
      [frontLeft, frontTop],
    ],
    sideFill,
    wallEdge,
  );

  ctx.fillStyle = AXIS_LABEL_COLOR;
  ctx.textAlign = "center";
  ctx.textBaseline = "top";
  for (let ti = 0; ti < ticks.length; ti++) {
    const x = xFor(ticks[ti]!);
    ctx.strokeStyle = GRIDLINE_COLOR;
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(x + d.dx, frontTop - d.dy);
    ctx.lineTo(x + d.dx, frontBottom - d.dy);
    ctx.stroke();
    ctx.fillText(labelStrings[ti]!, x, frontBottom + 4);
  }

  const bandH = planeH / categoryCount;
  const slotH = (bandH * 0.8) / series.length;
  const slotPad = bandH * 0.1;

  for (let i = 0; i < categoryCount; i++) {
    for (let si = 0; si < series.length; si++) {
      const s = series[si]!;
      const v = s.values[i] ?? 0;
      const fill = resolveBarFill(s, i);
      if (fill.skip) continue;
      const y = frontTop + i * bandH + slotPad + si * slotH;
      const xEnd = xFor(v);
      const x = Math.min(zeroX, xEnd);
      const w = Math.abs(xEnd - zeroX);
      drawBox(ctx, x, y, w, slotH * 0.92, d, fill.color);
    }
  }

  ctx.fillStyle = AXIS_LABEL_COLOR;
  ctx.textAlign = "right";
  ctx.textBaseline = "middle";
  for (let i = 0; i < categoryCount; i++) {
    const cy = frontTop + (i + 0.5) * bandH;
    ctx.fillText(catLabels[i]!, frontLeft - 4, cy);
  }
}
