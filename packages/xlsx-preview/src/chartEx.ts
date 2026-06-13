import type { Chart, ChartSeries } from "./types.js";
import type { Rect } from "./chart.js";
import { activeThemeColor } from "./color.js";
import { DEFAULT_PIE_ACCENTS } from "./chartAdvanced.js";
import {
  drawAxisFrame,
  drawPlaceholderPlot,
  formatAxisValue,
  formatGeneral,
  paintZeroBaseline,
  resolveAxisRange,
} from "./chartUtils.js";
import { drawBoxWhiskerChartEx, drawHistogramChartEx, drawParetoChartEx } from "./chartExStats.js";
import { drawRegionMapChartEx } from "./chartExRegionMap.js";

const AXIS_FONT_SIZE = 10;
const AXIS_LABEL_COLOR = "#52525b";
const AXIS_TICK_COUNT = 5;

export function drawChartEx(ctx: CanvasRenderingContext2D, chart: Chart, rect: Rect): void {
  switch (chart.cxLayout) {
    case "waterfall":
      drawWaterfallChartEx(ctx, chart, rect);
      return;
    case "funnel":
      drawFunnelChartEx(ctx, chart, rect);
      return;
    case "treemap":
      drawTreemapChartEx(ctx, chart, rect);
      return;
    case "sunburst":
      drawSunburstChartEx(ctx, chart, rect);
      return;
    case "histogram":
      drawHistogramChartEx(ctx, chart, rect);
      return;
    case "pareto":
      drawParetoChartEx(ctx, chart, rect);
      return;
    case "boxWhisker":
      drawBoxWhiskerChartEx(ctx, chart, rect);
      return;
    case "regionMap":
      drawRegionMapChartEx(ctx, chart, rect);
      return;

    default:
      drawPlaceholderPlot(ctx, chart, rect);
      return;
  }
}

function waterfallColors() {
  return {
    increment: activeThemeColor(4, "#4472C4"),
    decrement: activeThemeColor(5, "#ED7D31"),
    subtotal: activeThemeColor(6, "#A5A5A5"),
  };
}
const WATERFALL_CONNECTOR_COLOR = "#a6a6a6";

export function waterfallLegendEntries(chart: Chart): ChartSeries[] {
  const c = waterfallColors();
  const inc = chart.cxWaterfallIncrementColor || c.increment;
  const dec = chart.cxWaterfallDecrementColor || c.decrement;
  const sub = chart.cxWaterfallSubtotalColor || c.subtotal;
  const mk = (name: string, color: string): ChartSeries => ({
    name,
    color,
    values: [],
    xValues: [],
    bubbleSizes: [],
    pointColors: [],
    pointExplosions: [],
    trendlines: [],
  });
  return [mk("Increase", inc), mk("Decrease", dec), mk("Total", sub)];
}

function drawWaterfallChartEx(ctx: CanvasRenderingContext2D, chart: Chart, rect: Rect): void {
  const series = chart.series[0];
  if (!series || series.values.length === 0) {
    drawPlaceholderPlot(ctx, chart, rect);
    return;
  }
  const values = series.values;
  const n = values.length;
  const cats = chart.categories ?? [];
  const subtotalSet = new Set<number>(chart.cxSubtotalIndices ?? []);

  const bars: { start: number; end: number; subtotal: boolean }[] = [];
  let running = 0;
  for (let i = 0; i < n; i++) {
    const v = values[i]!;
    const sub = subtotalSet.has(i);
    if (sub) {
      bars.push({ start: 0, end: v, subtotal: true });
      running = v;
    } else {
      bars.push({ start: running, end: running + v, subtotal: false });
      running += v;
    }
  }

  let minV = 0;
  let maxV = 0;
  for (const b of bars) {
    minV = Math.min(minV, b.start, b.end);
    maxV = Math.max(maxV, b.start, b.end);
  }
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

  const inner = drawAxisFrame(ctx, chart, rect, ticks, minV, maxV, false, false);

  const slotW = inner.w / n;
  const xFor = (i: number) => inner.x + (i + 0.5) * slotW;
  const yFor = (v: number) => inner.y + (1 - (v - minV) / (maxV - minV)) * inner.h;
  ctx.font = `${AXIS_FONT_SIZE}px -apple-system, "Helvetica Neue", Arial, sans-serif`;
  ctx.fillStyle = AXIS_LABEL_COLOR;
  ctx.textAlign = "center";
  ctx.textBaseline = "top";
  let lastRight = -Infinity;
  for (let i = 0; i < n; i++) {
    const label = cats[i] ?? `${i + 1}`;
    const w = ctx.measureText(label).width;
    const cx = xFor(i);
    if (cx - w / 2 < lastRight + 8) continue;
    ctx.fillText(label, cx, inner.y + inner.h + 4);
    lastRight = cx + w / 2;
  }

  const barW = Math.max(2, slotW * 0.7);

  ctx.save();
  ctx.strokeStyle = WATERFALL_CONNECTOR_COLOR;
  ctx.lineWidth = 1;
  ctx.setLineDash([3, 2]);
  for (let i = 0; i < n - 1; i++) {
    const next = bars[i + 1]!;
    if (next.subtotal) continue;
    const cur = bars[i]!;
    const xRight = xFor(i) + barW / 2;
    const xLeft = xFor(i + 1) - barW / 2;
    const y = yFor(cur.end);
    ctx.beginPath();
    ctx.moveTo(xRight, y);
    ctx.lineTo(xLeft, y);
    ctx.stroke();
  }
  ctx.restore();

  const palette = waterfallColors();
  const incColor = chart.cxWaterfallIncrementColor || palette.increment;
  const decColor = chart.cxWaterfallDecrementColor || palette.decrement;
  const subColor = chart.cxWaterfallSubtotalColor || palette.subtotal;
  for (let i = 0; i < n; i++) {
    const b = bars[i]!;
    const color = b.subtotal ? subColor : b.end >= b.start ? incColor : decColor;
    const x = xFor(i) - barW / 2;
    const yTop = yFor(Math.max(b.start, b.end));
    const yBot = yFor(Math.min(b.start, b.end));
    const h = Math.max(1, yBot - yTop);
    ctx.fillStyle = color;
    ctx.fillRect(x, yTop, barW, h);
  }

  paintZeroBaseline(ctx, inner, minV, maxV);

  ctx.fillStyle = "#262626";
  ctx.font = `${AXIS_FONT_SIZE}px -apple-system, "Helvetica Neue", Arial, sans-serif`;
  ctx.textAlign = "center";
  for (let i = 0; i < n; i++) {
    const b = bars[i]!;
    const v = values[i]!;

    const labelValue = b.subtotal ? b.end : v;
    const text = chart.valueFormat
      ? formatAxisValue(labelValue, chart.valueFormat)
      : formatGeneral(labelValue);
    const above = b.end >= b.start;
    const yEdge = yFor(above ? b.end : b.end);
    ctx.textBaseline = above ? "bottom" : "top";
    ctx.fillText(text, xFor(i), yEdge + (above ? -3 : 3));
  }
}

function drawFunnelChartEx(ctx: CanvasRenderingContext2D, chart: Chart, rect: Rect): void {
  const series = chart.series[0];
  if (!series || series.values.length === 0) {
    drawPlaceholderPlot(ctx, chart, rect);
    return;
  }
  const values = series.values;
  const n = values.length;
  const cats = chart.categories ?? [];

  ctx.font = `${AXIS_FONT_SIZE}px -apple-system, "Helvetica Neue", Arial, sans-serif`;
  let labelW = 0;
  for (let i = 0; i < n; i++) {
    const t = cats[i] ?? `${i + 1}`;
    labelW = Math.max(labelW, ctx.measureText(t).width);
  }
  const LABEL_PAD = 8;
  const gutter = Math.min(rect.w * 0.3, labelW + LABEL_PAD * 2);
  const plotX = rect.x + gutter;
  const plotW = Math.max(20, rect.w - gutter - LABEL_PAD);
  const cx = plotX + plotW / 2;

  const slotH = rect.h / n;
  const barH = Math.max(2, slotH * 0.82);

  const maxV = Math.max(...values.map((v) => Math.abs(v)));
  if (!Number.isFinite(maxV) || maxV <= 0) {
    drawPlaceholderPlot(ctx, chart, rect);
    return;
  }
  const widthFor = (v: number) => {
    if (!Number.isFinite(v) || v <= 0) return 1;
    return Math.max(1, (v / maxV) * plotW);
  };

  const fill = series.color ?? activeThemeColor(4, "#4472C4");
  ctx.fillStyle = fill;

  for (let i = 0; i < n; i++) {
    const v = values[i] ?? 0;
    const w = widthFor(v);
    const yTop = rect.y + i * slotH + (slotH - barH) / 2;
    const x = cx - w / 2;
    ctx.fillStyle = fill;
    ctx.fillRect(x, yTop, w, barH);

    ctx.fillStyle = AXIS_LABEL_COLOR;
    ctx.font = `${AXIS_FONT_SIZE}px -apple-system, "Helvetica Neue", Arial, sans-serif`;
    ctx.textAlign = "right";
    ctx.textBaseline = "middle";
    ctx.fillText(cats[i] ?? `${i + 1}`, plotX - LABEL_PAD, yTop + barH / 2);

    const text = chart.valueFormat ? formatAxisValue(v, chart.valueFormat) : formatGeneral(v);
    const textW = ctx.measureText(text).width;
    if (textW + 8 <= w) {
      ctx.fillStyle = "#ffffff";
      ctx.textAlign = "center";
      ctx.textBaseline = "middle";
      ctx.fillText(text, cx, yTop + barH / 2);
    }
  }
}

function drawTreemapChartEx(ctx: CanvasRenderingContext2D, chart: Chart, rect: Rect): void {
  const series = chart.series[0];
  if (!series || series.values.length === 0) {
    drawPlaceholderPlot(ctx, chart, rect);
    return;
  }
  const values = series.values.map((v) => (Number.isFinite(v) && v > 0 ? v : 0));
  const total = values.reduce((a, b) => a + b, 0);
  if (total <= 0) {
    drawPlaceholderPlot(ctx, chart, rect);
    return;
  }

  const levels = chart.cxCategoryLevels ?? [];
  const leafLabels =
    chart.categories.length > 0
      ? chart.categories
      : levels.length > 0
        ? (levels[levels.length - 1] ?? [])
        : values.map((_, i) => `${i + 1}`);
  const parents = levels.length > 1 ? (levels[0] ?? []) : null;

  type Leaf = { label: string; value: number; idx: number };
  type Group = { label: string; total: number; leaves: Leaf[] };
  const groups: Group[] = [];
  const groupByName = new Map<string, Group>();
  for (let i = 0; i < values.length; i++) {
    const pname = parents ? (parents[i] ?? "") : `__leaf_${i}`;
    let g = groupByName.get(pname);
    if (!g) {
      g = { label: parents ? pname : (leafLabels[i] ?? `${i + 1}`), total: 0, leaves: [] };
      groupByName.set(pname, g);
      groups.push(g);
    }
    g.total += values[i]!;
    g.leaves.push({ label: leafLabels[i] ?? `${i + 1}`, value: values[i]!, idx: i });
  }

  const groupRects = squarify(
    groups.map((g) => g.total),
    rect,
  );

  for (let gi = 0; gi < groups.length; gi++) {
    const g = groups[gi]!;
    const r = groupRects[gi];
    if (!r) continue;
    const groupColor = activeThemeColor(4 + (gi % 6), DEFAULT_PIE_ACCENTS[gi % 6]!);

    if (parents && g.leaves.length > 0) {
      const childRects = squarify(
        g.leaves.map((l) => l.value),
        r,
      );
      for (let ci = 0; ci < g.leaves.length; ci++) {
        const cr = childRects[ci];
        if (!cr) continue;
        ctx.fillStyle = groupColor;
        ctx.fillRect(cr.x, cr.y, cr.w, cr.h);
        ctx.strokeStyle = "#ffffff";
        ctx.lineWidth = 1.5;
        ctx.strokeRect(cr.x + 0.75, cr.y + 0.75, cr.w - 1.5, cr.h - 1.5);
        paintTreemapLabel(ctx, cr, g.leaves[ci]!.label, "#ffffff");
      }

      ctx.strokeStyle = "#ffffff";
      ctx.lineWidth = 3;
      ctx.strokeRect(r.x + 1.5, r.y + 1.5, r.w - 3, r.h - 3);

      if (r.w > 60 && r.h > 24) {
        paintTreemapLabel(
          ctx,
          { x: r.x + 6, y: r.y + 2, w: r.w - 12, h: 18 },
          g.label,
          "#ffffff",
          "left",
          "top",
          12,
        );
      }
    } else {
      ctx.fillStyle = groupColor;
      ctx.fillRect(r.x, r.y, r.w, r.h);
      ctx.strokeStyle = "#ffffff";
      ctx.lineWidth = 1.5;
      ctx.strokeRect(r.x + 0.75, r.y + 0.75, r.w - 1.5, r.h - 1.5);
      paintTreemapLabel(ctx, r, g.label, "#ffffff");
    }
  }
}

function paintTreemapLabel(
  ctx: CanvasRenderingContext2D,
  cell: Rect,
  text: string,
  color: string,
  align: CanvasTextAlign = "center",
  baseline: CanvasTextBaseline = "middle",
  size = 11,
): void {
  if (cell.w < 24 || cell.h < 14) return;
  ctx.font = `${size}px -apple-system, "Helvetica Neue", Arial, sans-serif`;
  const tw = ctx.measureText(text).width;
  if (tw > cell.w - 4) return;
  ctx.fillStyle = color;
  ctx.textAlign = align;
  ctx.textBaseline = baseline;
  const x = align === "left" ? cell.x : align === "right" ? cell.x + cell.w : cell.x + cell.w / 2;
  const y =
    baseline === "top" ? cell.y : baseline === "bottom" ? cell.y + cell.h : cell.y + cell.h / 2;
  ctx.fillText(text, x, y);
}

function squarify(values: number[], rect: Rect): (Rect | null)[] {
  const n = values.length;
  const out: (Rect | null)[] = new Array(n).fill(null);
  const totalV = values.reduce((a, b) => a + (b > 0 ? b : 0), 0);
  if (totalV <= 0 || rect.w <= 0 || rect.h <= 0) return out;
  const area = rect.w * rect.h;

  type Item = { v: number; i: number; scaled: number };
  const items: Item[] = [];
  for (let i = 0; i < n; i++) {
    const v = values[i] ?? 0;
    if (v > 0) items.push({ v, i, scaled: (v / totalV) * area });
  }
  items.sort((a, b) => b.v - a.v);

  const worst = (row: Item[], shortSide: number): number => {
    if (row.length === 0) return Infinity;
    let s = 0;
    let rmax = -Infinity;
    let rmin = Infinity;
    for (const it of row) {
      s += it.scaled;
      if (it.scaled > rmax) rmax = it.scaled;
      if (it.scaled < rmin) rmin = it.scaled;
    }
    const w = shortSide * shortSide;
    return Math.max((w * rmax) / (s * s), (s * s) / (w * rmin));
  };

  const layoutRow = (row: Item[], r: Rect): Rect => {
    const horizontal = r.w >= r.h;
    const sumS = row.reduce((a, b) => a + b.scaled, 0);
    const longExt = sumS / Math.min(r.w, r.h);
    let cursor = 0;
    if (horizontal) {
      for (const it of row) {
        const itH = (it.scaled / sumS) * r.h;
        out[it.i] = { x: r.x, y: r.y + cursor, w: longExt, h: itH };
        cursor += itH;
      }
      return { x: r.x + longExt, y: r.y, w: r.w - longExt, h: r.h };
    } else {
      for (const it of row) {
        const itW = (it.scaled / sumS) * r.w;
        out[it.i] = { x: r.x + cursor, y: r.y, w: itW, h: longExt };
        cursor += itW;
      }
      return { x: r.x, y: r.y + longExt, w: r.w, h: r.h - longExt };
    }
  };

  let remaining: Rect = { ...rect };
  let i = 0;
  let row: Item[] = [];
  while (i < items.length) {
    const shortSide = Math.min(remaining.w, remaining.h);
    if (shortSide <= 0) break;
    const candidate = [...row, items[i]!];
    const wCur = row.length === 0 ? Infinity : worst(row, shortSide);
    const wNext = worst(candidate, shortSide);
    if (wNext <= wCur) {
      row = candidate;
      i++;
    } else {
      remaining = layoutRow(row, remaining);
      row = [];
    }
  }
  if (row.length > 0) {
    layoutRow(row, remaining);
  }
  return out;
}

function drawSunburstChartEx(ctx: CanvasRenderingContext2D, chart: Chart, rect: Rect): void {
  const series = chart.series[0];
  if (!series || series.values.length === 0) {
    drawPlaceholderPlot(ctx, chart, rect);
    return;
  }
  const values = series.values.map((v) => (Number.isFinite(v) && v > 0 ? v : 0));
  const total = values.reduce((a, b) => a + b, 0);
  if (total <= 0) {
    drawPlaceholderPlot(ctx, chart, rect);
    return;
  }

  const levels = chart.cxCategoryLevels ?? [];
  const flatLabels =
    chart.categories.length > 0 ? chart.categories : values.map((_, i) => `${i + 1}`);

  type Node = {
    label: string;
    value: number;
    branchIdx: number;
    children: Node[];
  };
  const root: Node = { label: "", value: 0, branchIdx: -1, children: [] };
  const branchOrder = new Map<string, number>();
  for (let i = 0; i < values.length; i++) {
    const path: string[] =
      levels.length > 0 ? levels.map((lvl) => lvl[i] ?? "") : [flatLabels[i] ?? `${i + 1}`];

    let branchIdx = branchOrder.get(path[0] ?? "");
    if (branchIdx == null) {
      branchIdx = branchOrder.size;
      branchOrder.set(path[0] ?? "", branchIdx);
    }
    let cur = root;
    for (let d = 0; d < path.length; d++) {
      const lbl = path[d] ?? "";
      let next = cur.children.find((n) => n.label === lbl);
      if (!next) {
        next = { label: lbl, value: 0, branchIdx, children: [] };
        cur.children.push(next);
      }
      cur = next;
    }
    cur.value += values[i]!;
  }

  const rollUp = (n: Node): number => {
    if (n.children.length === 0) return n.value;
    n.value = n.children.reduce((s, c) => s + rollUp(c), 0);
    return n.value;
  };
  rollUp(root);
  if (root.value <= 0) {
    drawPlaceholderPlot(ctx, chart, rect);
    return;
  }

  const cxp = rect.x + rect.w / 2;
  const cyp = rect.y + rect.h / 2;
  const ringCount = Math.max(1, levels.length || 1);
  const outerR = Math.max(20, Math.min(rect.w, rect.h) / 2 - 8);

  const innerR = outerR * 0.12;
  const ringStep = (outerR - innerR) / ringCount;

  const drawNode = (node: Node, depth: number, startAngle: number): number => {
    const sweep = (node.value / root.value) * Math.PI * 2;
    const endAngle = startAngle + sweep;
    if (depth >= 0 && node !== root) {
      const rIn = innerR + depth * ringStep;
      const rOut = rIn + ringStep;

      const base = activeThemeColor(
        4 + (node.branchIdx % 6),
        DEFAULT_PIE_ACCENTS[node.branchIdx % 6]!,
      );

      const fill = ringCount > 1 && depth === 0 ? mixColor(base, "#000000", 0.15) : base;
      ctx.fillStyle = fill;
      ctx.beginPath();
      ctx.arc(cxp, cyp, rOut, startAngle, endAngle);
      ctx.arc(cxp, cyp, rIn, endAngle, startAngle, true);
      ctx.closePath();
      ctx.fill();
      ctx.strokeStyle = "#ffffff";
      ctx.lineWidth = 1.5;
      ctx.stroke();

      const arcLen = sweep * ((rIn + rOut) / 2);
      const radialLen = rOut - rIn;
      if (sweep > 0.18 && arcLen > 18 && radialLen > 14) {
        const mid = (startAngle + endAngle) / 2;
        const midR = (rIn + rOut) / 2;
        const lx = cxp + Math.cos(mid) * midR;
        const ly = cyp + Math.sin(mid) * midR;
        ctx.save();
        ctx.translate(lx, ly);

        let rot = mid + Math.PI / 2;
        if (Math.sin(mid) > 0) rot -= Math.PI;
        ctx.rotate(rot);
        ctx.fillStyle = "#ffffff";
        ctx.font = `10px -apple-system, "Helvetica Neue", Arial, sans-serif`;
        ctx.textAlign = "center";
        ctx.textBaseline = "middle";
        const text = node.label;

        if (ctx.measureText(text).width <= arcLen - 4) {
          ctx.fillText(text, 0, 0);
        }
        ctx.restore();
      }
    }
    let cursor = startAngle;
    for (const child of node.children) {
      cursor = drawNode(child, depth + 1, cursor);
    }
    return endAngle;
  };

  drawNode(root, -1, -Math.PI / 2);
}

function mixColor(a: string, b: string, t: number): string {
  const pa = parseHex(a);
  const pb = parseHex(b);
  if (!pa || !pb) return a;
  const r = Math.round(pa[0] + (pb[0] - pa[0]) * t);
  const g = Math.round(pa[1] + (pb[1] - pa[1]) * t);
  const bl = Math.round(pa[2] + (pb[2] - pa[2]) * t);
  return `rgb(${r},${g},${bl})`;
}
function parseHex(c: string): [number, number, number] | null {
  const m = /^#?([0-9a-f]{6})$/i.exec(c.trim());
  if (!m) return null;
  const v = parseInt(m[1]!, 16);
  return [(v >> 16) & 0xff, (v >> 8) & 0xff, v & 0xff];
}
