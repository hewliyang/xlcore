import type { Chart, ChartSeries } from "./types.js";
import type { Rect } from "./chart.js";
import { applyTint } from "./color.js";
import { resolveBarFill } from "./chartAdvanced.js";
import {
  drawPlaceholderPlot,
  formatAxisValue,
  resolveAxisRange,
  AXIS_FONT_SIZE,
} from "./chartUtils.js";

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

const SURFACE_BANDS = 8;

function hslHex(h: number, s: number, l: number): string {
  const c = (1 - Math.abs(2 * l - 1)) * s;
  const hp = h / 60;
  const x = c * (1 - Math.abs((hp % 2) - 1));
  let r = 0,
    g = 0,
    b = 0;
  if (hp < 1) [r, g, b] = [c, x, 0];
  else if (hp < 2) [r, g, b] = [x, c, 0];
  else if (hp < 3) [r, g, b] = [0, c, x];
  else if (hp < 4) [r, g, b] = [0, x, c];
  else if (hp < 5) [r, g, b] = [x, 0, c];
  else [r, g, b] = [c, 0, x];
  const m = l - c / 2;
  const to = (v: number) =>
    Math.round((v + m) * 255)
      .toString(16)
      .padStart(2, "0");
  return `#${to(r)}${to(g)}${to(b)}`;
}

function bandColor(band: number): string {
  const t = SURFACE_BANDS <= 1 ? 0 : band / (SURFACE_BANDS - 1);
  return hslHex((1 - t) * 240, 0.62, 0.55);
}

export function drawSurfaceChart(ctx: CanvasRenderingContext2D, chart: Chart, rect: Rect): void {
  const series = chart.series.filter((s) => s.values.length > 0);
  if (series.length === 0) {
    drawPlaceholderPlot(ctx, chart, rect);
    return;
  }
  const cols = Math.max(...series.map((s) => s.values.length), (chart.categories ?? []).length);
  const rows = series.length;
  if (cols === 0) return;

  const grid: number[][] = series.map((s) =>
    Array.from({ length: cols }, (_, c) => s.values[c] ?? 0),
  );

  let minV = Number.POSITIVE_INFINITY,
    maxV = Number.NEGATIVE_INFINITY;
  for (const r of grid)
    for (const v of r) {
      if (v > maxV) maxV = v;
      if (v < minV) minV = v;
    }
  if (!Number.isFinite(minV)) minV = 0;
  if (!Number.isFinite(maxV)) maxV = 1;
  const range = resolveAxisRange(
    minV,
    maxV,
    chart.valueMin,
    chart.valueMax,
    AXIS_TICK_COUNT,
    chart.majorUnit,
  );
  minV = range.minV;
  maxV = range.maxV;
  const ticks = range.ticks;
  const span = maxV - minV || 1;

  ctx.font = `${AXIS_FONT_SIZE}px -apple-system, "Helvetica Neue", Arial, sans-serif`;
  const labelStrings = ticks.map((t) => formatAxisValue(t, chart.valueFormat, chart.dispUnits));
  const catLabels = Array.from(
    { length: cols },
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
  const frontLeft = inner.x;
  const frontRight = inner.x + inner.w - d.dx - 4;
  const floorY = inner.y + inner.h;
  const planeW = frontRight - frontLeft;
  const heightSpan = Math.max(20, inner.h - d.dy - 4);

  const colFrac = (c: number) => (cols > 1 ? c / (cols - 1) : 0);
  const rowFrac = (r: number) => (rows > 1 ? r / (rows - 1) : 0);
  const project = (c: number, r: number, v: number): [number, number] => {
    const rf = rowFrac(r);
    const x = frontLeft + colFrac(c) * planeW + rf * d.dx;
    const baseY = floorY - rf * d.dy;
    return [x, baseY - ((v - minV) / span) * heightSpan];
  };

  const floorFill = normHex(chart.floorFill, "#d9d9d9");
  const backFill = normHex(chart.backWallFill, "#f1f1f1");
  const sideFill = normHex(chart.sideWallFill, "#e6e6e6");
  const wallEdge = "#bdbdbd";

  const f00: [number, number] = [frontLeft, floorY];
  const f10: [number, number] = [frontRight, floorY];
  const f01: [number, number] = [frontLeft + d.dx, floorY - d.dy];
  const f11: [number, number] = [frontRight + d.dx, floorY - d.dy];
  drawParallelogram(ctx, [f00, f10, f11, f01], floorFill, wallEdge);

  const backTop = floorY - d.dy - heightSpan;
  drawParallelogram(
    ctx,
    [f01, f11, [frontRight + d.dx, backTop], [frontLeft + d.dx, backTop]],
    backFill,
    wallEdge,
  );
  drawParallelogram(
    ctx,
    [f00, f01, [frontLeft + d.dx, backTop], [frontLeft, floorY - heightSpan]],
    sideFill,
    wallEdge,
  );

  ctx.fillStyle = AXIS_LABEL_COLOR;
  ctx.textAlign = "right";
  ctx.textBaseline = "middle";
  for (let ti = 0; ti < ticks.length; ti++) {
    const yf = floorY - ((ticks[ti]! - minV) / span) * heightSpan;
    ctx.strokeStyle = GRIDLINE_COLOR;
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(frontLeft + d.dx, yf - d.dy);
    ctx.lineTo(frontRight + d.dx, yf - d.dy);
    ctx.stroke();
    ctx.fillText(labelStrings[ti]!, frontLeft - 4, yf);
  }

  for (let r = rows - 2; r >= 0; r--) {
    for (let c = 0; c < cols - 1; c++) {
      const v00 = grid[r]![c]!;
      const v01 = grid[r]![c + 1]!;
      const v11 = grid[r + 1]![c + 1]!;
      const v10 = grid[r + 1]![c]!;
      const avg = (v00 + v01 + v11 + v10) / 4;
      const band = Math.min(
        SURFACE_BANDS - 1,
        Math.max(0, Math.floor(((avg - minV) / span) * SURFACE_BANDS)),
      );
      const pts: Array<[number, number]> = [
        project(c, r, v00),
        project(c + 1, r, v01),
        project(c + 1, r + 1, v11),
        project(c, r + 1, v10),
      ];
      if (chart.wireframe) {
        drawParallelogram(ctx, pts, "rgba(255,255,255,0.04)", "#3f3f46");
      } else {
        drawParallelogram(ctx, pts, bandColor(band), "#52525b");
      }
    }
  }

  ctx.fillStyle = AXIS_LABEL_COLOR;
  ctx.textAlign = "center";
  ctx.textBaseline = "top";
  for (let c = 0; c < cols; c++) {
    const [x] = project(c, 0, minV);
    ctx.fillText(catLabels[c]!, x, floorY + 4);
  }
}
