// chartEx (`cx:`) painters. Dispatch entrypoint is `drawChartEx`;
// each layout has its own painter below. The legacy (`c:`) painters
// live in `chartAdvanced.ts`. Split out of `chartAdvanced.ts` once the
// chartEx surface (waterfall + funnel + treemap + sunburst) grew past
// the per-file LoC budget. The stat-layout painters (histogram /
// pareto / boxWhisker) live in `chartExStats.ts` for the same reason.
//
// Layouts shipped:
//   - waterfall  (Excel-desktop-authored fixture)
//   - funnel     (SpreadJS-authored fixture)
//   - treemap    (SpreadJS-authored fixture, multi-level hierarchy)
//   - sunburst   (SpreadJS-authored fixture, multi-level hierarchy)
//   - histogram  (Excel-desktop-authored fixture; auto-binned)
//   - pareto     (Excel-desktop-authored fixture; bars + cumulative %)
//   - boxWhisker (Excel-desktop-authored fixture; per-series quartiles)
//
// Layouts shipped (cont'd):
//   - regionMap  (Excel-authored "2-color Map Chart" fixture; uses an
//                 embedded Natural Earth 110m countries dataset)

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
import {
  drawBoxWhiskerChartEx,
  drawHistogramChartEx,
  drawParetoChartEx,
} from "./chartExStats.js";
import { drawRegionMapChartEx } from "./chartExRegionMap.js";

const AXIS_FONT_SIZE = 10;
const AXIS_LABEL_COLOR = "#52525b";
const AXIS_TICK_COUNT = 5;

// ---------- dispatch ----------

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
      // Unknown / future chartEx layout: paint a placeholder frame so
      // the user still sees the chart's anchor + title instead of an
      // empty bbox.
      drawPlaceholderPlot(ctx, chart, rect);
      return;
  }
}

// ---------- waterfall ----------
//
// Office "Waterfall" colors map to the workbook theme via the
// chartEx color-style part (`xl/charts/colors1.xml` typically cycles
// `accent1/accent2/accent3` as Increase/Decrement/Subtotal). We don't
// parse the color-style part yet — it's worth a dedicated extractor
// once we hit a workbook that uses a non-default cycle — but we follow
// Excel's documented "first three accents" convention by routing
// through `activeThemeColor` (indices 4/5/6 in our `theme.colors`
// layout: lt1/dk1/lt2/dk2 then accent1..accent6). Fallbacks match the
// Office 2016 default theme accents.
function waterfallColors() {
  return {
    increment: activeThemeColor(4, "#4472C4"), // accent1
    decrement: activeThemeColor(5, "#ED7D31"), // accent2
    subtotal: activeThemeColor(6, "#A5A5A5"), // accent3
  };
}
const WATERFALL_CONNECTOR_COLOR = "#a6a6a6";

/// Synthetic legend entries for a waterfall chart — Excel paints three
/// swatches (Increase / Decrement / Total) even though the chart is a
/// single OOXML series. Exposed so `chart.ts` can render them through
/// the existing legend code path.
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
  // Waterfall bars: each category's bar runs from `start` (the
  // cumulative running total before this point) to `end` (cumulative
  // after this point). Subtotal bars are absolute (start = 0,
  // end = value); the running total is then reset to that value. The
  // *first* bar is implicitly a subtotal in Excel's waterfall when
  // the workbook author doesn't flag it — but our fixtures always
  // flag explicit subtotals, so we trust the indices and treat
  // unflagged-first as a delta-from-zero (which produces the same
  // visual: 0 -> value).
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

  // Value-axis range covers all bar floors + tops (clamped to zero
  // floor on the positive side per Excel waterfall default).
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
    /*zeroClamp=*/ false,
    AXIS_TICK_COUNT,
    chart.majorUnit,
  );
  minV = range.minV;
  maxV = range.maxV;
  const ticks = range.ticks;

  const inner = drawAxisFrame(ctx, chart, rect, ticks, minV, maxV, false, false);

  // Category axis labels (centered under each bar slot).
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

  // Bar widths: same heuristic as the column painter — 70% of slot.
  const barW = Math.max(2, slotW * 0.7);

  // 1. Connector lines (dashed). Excel draws a thin dashed segment
  //    from the *right edge* of bar i at its end-y to the *left edge*
  //    of bar i+1 at its start-y, except subtotal bars (which start
  //    from zero and don't connect from the prior cumulative).
  ctx.save();
  ctx.strokeStyle = WATERFALL_CONNECTOR_COLOR;
  ctx.lineWidth = 1;
  ctx.setLineDash([3, 2]);
  for (let i = 0; i < n - 1; i++) {
    const next = bars[i + 1]!;
    if (next.subtotal) continue; // next bar starts from 0; no connector
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

  // 2. Bars.
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

  // 3. Zero baseline (paints if the axis range straddles zero).
  paintZeroBaseline(ctx, inner, minV, maxV);

  // 4. Data labels: print each bar's value just outside the bar end.
  ctx.fillStyle = "#262626";
  ctx.font = `${AXIS_FONT_SIZE}px -apple-system, "Helvetica Neue", Arial, sans-serif`;
  ctx.textAlign = "center";
  for (let i = 0; i < n; i++) {
    const b = bars[i]!;
    const v = values[i]!;
    // Subtotal bars show the cumulative value; delta bars show the
    // signed change. Use the chart's valueFormat for both.
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

// ---------- funnel ----------
//
// Funnel charts paint one horizontal bar per category. The widest bar
// (largest value) spans the plot width; every other bar's width is
// scaled relative to that maximum and the bar is centered on the
// plot's vertical axis so the silhouette tapers symmetrically. Excel
// authors don't draw a value axis on funnels — just category labels
// flush-right outside the leftmost bar edge, and an optional in-bar
// value label.
//
// Bars use a single theme accent color (accent1) per Office's default
// chartEx color cycle for funnel. We don't yet parse the chartEx
// colorStyle part; if a workbook overrides via `cx:spPr` on the
// series we pick that up through the existing `series.color` field.
function drawFunnelChartEx(ctx: CanvasRenderingContext2D, chart: Chart, rect: Rect): void {
  const series = chart.series[0];
  if (!series || series.values.length === 0) {
    drawPlaceholderPlot(ctx, chart, rect);
    return;
  }
  const values = series.values;
  const n = values.length;
  const cats = chart.categories ?? [];

  // Reserve space on the left for category labels. Measure all labels
  // and use the widest as the gutter; cap at ~30% of the plot so a
  // single very long label can't starve the bars.
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

  // Vertical layout: equal-height slots per category, 80% bar / 20% gap.
  const slotH = rect.h / n;
  const barH = Math.max(2, slotH * 0.82);

  // Width scaling. Negative / zero values render as a hairline so
  // the slot stays visible.
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

  // Bars + category labels + in-bar value labels.
  for (let i = 0; i < n; i++) {
    const v = values[i] ?? 0;
    const w = widthFor(v);
    const yTop = rect.y + i * slotH + (slotH - barH) / 2;
    const x = cx - w / 2;
    ctx.fillStyle = fill;
    ctx.fillRect(x, yTop, w, barH);

    // Category label, flush-right against the gutter.
    ctx.fillStyle = AXIS_LABEL_COLOR;
    ctx.font = `${AXIS_FONT_SIZE}px -apple-system, "Helvetica Neue", Arial, sans-serif`;
    ctx.textAlign = "right";
    ctx.textBaseline = "middle";
    ctx.fillText(cats[i] ?? `${i + 1}`, plotX - LABEL_PAD, yTop + barH / 2);

    // In-bar value label: centered when it fits, else suppressed.
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

// ---------- treemap ----------
//
// Squarified treemap (Bruls et al., 2000). Each leaf is a rectangle
// whose area is proportional to its value; the layout greedily fills
// the plot rect with rows of rectangles that minimize the worst
// aspect ratio.
//
// Hierarchy: when `chart.cxCategoryLevels` is non-empty, we group
// leaves by their level-0 parent value, lay out parents first
// (squarified across the full plot), then squarify children inside
// each parent's rectangle. Each parent's group gets one theme accent
// color; children share the parent color (matches Excel's default
// chartEx treemap palette). Flat (single-level) treemaps cycle accents
// per leaf.
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

  // Group leaves by parent. Preserves first-appearance order so the
  // visual matches the source-row order Excel uses.
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

  // Outer pass: squarify the groups across the full plot rect.
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
      // Inner pass: squarify children inside the parent rect. Children
      // share the parent's color; we differentiate them with thin
      // white borders (matches hsx / Excel default treemap chrome).
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
      // Outline the parent group with a slightly bolder edge so the
      // grouping reads above the per-leaf borders.
      ctx.strokeStyle = "#ffffff";
      ctx.lineWidth = 3;
      ctx.strokeRect(r.x + 1.5, r.y + 1.5, r.w - 3, r.h - 3);
      // Parent label in the top-left corner of the group rect.
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
      // Flat treemap: each group is one leaf. Cycle accents per cell.
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
  if (cell.w < 24 || cell.h < 14) return; // too small to read
  ctx.font = `${size}px -apple-system, "Helvetica Neue", Arial, sans-serif`;
  const tw = ctx.measureText(text).width;
  if (tw > cell.w - 4) return; // would overflow; skip rather than truncate
  ctx.fillStyle = color;
  ctx.textAlign = align;
  ctx.textBaseline = baseline;
  const x = align === "left" ? cell.x : align === "right" ? cell.x + cell.w : cell.x + cell.w / 2;
  const y =
    baseline === "top" ? cell.y : baseline === "bottom" ? cell.y + cell.h : cell.y + cell.h / 2;
  ctx.fillText(text, x, y);
}

/**
 * Squarified treemap layout. Returns one Rect per input value, in the
 * same order, filling `rect`. Zero/negative values get a `null` slot
 * (caller can skip painting).
 *
 * Algorithm: Bruls/Huijsen/van Wijk 2000. We scale all input values
 * so their sum equals the rect's area, then greedily build rows along
 * the rect's shorter side, extending the current row with the next
 * value until adding it would *worsen* the worst aspect ratio in the
 * row; at that point we lay the row down and start the next one in
 * the remaining sub-rect.
 */
function squarify(values: number[], rect: Rect): (Rect | null)[] {
  const n = values.length;
  const out: (Rect | null)[] = new Array(n).fill(null);
  const totalV = values.reduce((a, b) => a + (b > 0 ? b : 0), 0);
  if (totalV <= 0 || rect.w <= 0 || rect.h <= 0) return out;
  const area = rect.w * rect.h;

  // Pair each positive value with its original index. Sort desc by
  // value for a more squarish layout (Bruls et al. recommends this).
  type Item = { v: number; i: number; scaled: number };
  const items: Item[] = [];
  for (let i = 0; i < n; i++) {
    const v = values[i] ?? 0;
    if (v > 0) items.push({ v, i, scaled: (v / totalV) * area });
  }
  items.sort((a, b) => b.v - a.v);

  // Worst aspect ratio in a candidate row of `row` items packed along
  // the shorter side of length `shortSide`.
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

  // Lay a finished row of items down along the short side of `r`,
  // returning the leftover sub-rect.
  const layoutRow = (row: Item[], r: Rect): Rect => {
    const horizontal = r.w >= r.h; // row runs along the shorter side
    const sumS = row.reduce((a, b) => a + b.scaled, 0);
    const longExt = sumS / Math.min(r.w, r.h);
    let cursor = 0;
    if (horizontal) {
      // Row of items stacked vertically along the left edge of r,
      // each occupying its share of the short (h) side.
      for (const it of row) {
        const itH = (it.scaled / sumS) * r.h;
        out[it.i] = { x: r.x, y: r.y + cursor, w: longExt, h: itH };
        cursor += itH;
      }
      return { x: r.x + longExt, y: r.y, w: r.w - longExt, h: r.h };
    } else {
      // Row of items packed horizontally across the top of r.
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

// ---------- sunburst ----------
//
// Concentric ring chart. Each level in `chart.cxCategoryLevels` maps
// to one ring (level 0 = innermost ring, last level = outermost
// ring). A leaf's angular span is `value / total * 2π`; a parent's
// span is the sum of its children's spans. Excel paints rings with
// one accent color per top-level branch and tints child rings darker
// the deeper they go; we match that with a per-branch theme accent
// and a fixed darken on the innermost ring.
//
// If only one level is authored (no hierarchy, equivalent to a
// doughnut), we render a single ring with per-slice accent cycling.
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
  // Build per-ring node tree. Each leaf carries (path: string[], value).
  type Node = {
    label: string;
    value: number;
    branchIdx: number; // index of the level-0 ancestor; drives color
    children: Node[];
  };
  const root: Node = { label: "", value: 0, branchIdx: -1, children: [] };
  const branchOrder = new Map<string, number>();
  for (let i = 0; i < values.length; i++) {
    const path: string[] =
      levels.length > 0 ? levels.map((lvl) => lvl[i] ?? "") : [flatLabels[i] ?? `${i + 1}`];
    // Branch index from the level-0 ancestor.
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
  // Roll up internal-node values from leaves.
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
  // Small inner hole for visual breathing room (Excel default).
  const innerR = outerR * 0.12;
  const ringStep = (outerR - innerR) / ringCount;

  // Draw each node at depth d (root.children = depth 0) as an arc
  // wedge between innerR + d*ringStep and innerR + (d+1)*ringStep.
  // We use a DFS over the tree so siblings stay contiguous angularly,
  // matching hsx/Excel's quarter → month grouping.
  const drawNode = (node: Node, depth: number, startAngle: number): number => {
    const sweep = (node.value / root.value) * Math.PI * 2;
    const endAngle = startAngle + sweep;
    if (depth >= 0 && node !== root) {
      const rIn = innerR + depth * ringStep;
      const rOut = rIn + ringStep;
      // Per-branch accent color, tinted slightly darker on deeper rings.
      const base = activeThemeColor(
        4 + (node.branchIdx % 6),
        DEFAULT_PIE_ACCENTS[node.branchIdx % 6]!,
      );
      // Outer rings get the unmodified accent; inner ring slightly
      // darker so the hierarchy reads as a gradient inward.
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

      // Slice label (skip if the wedge is too thin to read).
      const arcLen = sweep * ((rIn + rOut) / 2);
      const radialLen = rOut - rIn;
      if (sweep > 0.18 && arcLen > 18 && radialLen > 14) {
        const mid = (startAngle + endAngle) / 2;
        const midR = (rIn + rOut) / 2;
        const lx = cxp + Math.cos(mid) * midR;
        const ly = cyp + Math.sin(mid) * midR;
        ctx.save();
        ctx.translate(lx, ly);
        // Rotate text tangentially to the arc; flip on the bottom half
        // so it reads upright.
        let rot = mid + Math.PI / 2;
        if (Math.sin(mid) > 0) rot -= Math.PI;
        ctx.rotate(rot);
        ctx.fillStyle = "#ffffff";
        ctx.font = `10px -apple-system, "Helvetica Neue", Arial, sans-serif`;
        ctx.textAlign = "center";
        ctx.textBaseline = "middle";
        const text = node.label;
        // Only paint when the label fits inside the wedge tangentially.
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
  // Start at 12 o'clock, sweep clockwise (matches Excel).
  drawNode(root, -1, -Math.PI / 2);
}

/** Mix two hex colors. `t=0` => a, `t=1` => b. */
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
