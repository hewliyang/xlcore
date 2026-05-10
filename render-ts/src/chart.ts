// Canvas chart renderer. v0 covers:
//   - column / bar (clustered + stacked)
//   - line (standard / stacked / percentStacked) with optional markers
//   - area (standard / stacked / percentStacked)
//   - pie / doughnut (one series, slice-per-category)
//   - scatter (xy points, optional connecting lines)
// Other types fall back to a placeholder box+title.
//
// Geometry: the host calls `drawChart(ctx, chart, rect)` with a logical-
// pixel rectangle; we lay out the title, plot area, value-axis ticks, x-axis
// labels, bars and legend inside it.
//
// Number formatting reuses the same subset as the cell renderer, so axis
// labels match cell-level "$#,##0" formatting.

import type { Chart } from "./types.js";
import {
  buildLabelText,
  buildStackedRows,
  drawAxisFrame,
  drawCategoryAxis,
  drawLabel,
  drawLegend,
  drawPlaceholderPlot,
  effectiveLabels,
  formatAxisValue,
  formatGeneral,
  niceTicks,
  valueRange,
  withAlpha,
} from "./chartUtils.js";

const TITLE_PAD = 8;
const TITLE_FONT_SIZE = 14;
const AXIS_FONT_SIZE = 10;
const LEGEND_FONT_SIZE = 11;
const PLOT_PAD_LEFT = 8;
const PLOT_PAD_RIGHT = 12;
const AXIS_TICK_COUNT = 5;
const GRIDLINE_COLOR = "#e5e7eb";
const AXIS_LABEL_COLOR = "#52525b";
const TITLE_COLOR = "#262626";

export interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

export function drawChart(ctx: CanvasRenderingContext2D, chart: Chart, rect: Rect): void {
  // Frame: white fill + faint border (matches Excel default chart frame).
  ctx.fillStyle = "#ffffff";
  ctx.fillRect(rect.x, rect.y, rect.w, rect.h);
  ctx.strokeStyle = "#d4d4d8";
  ctx.lineWidth = 1;
  ctx.strokeRect(rect.x + 0.5, rect.y + 0.5, rect.w - 1, rect.h - 1);

  // Title strip
  let cursorY = rect.y + TITLE_PAD;
  if (chart.title) {
    ctx.fillStyle = TITLE_COLOR;
    ctx.font = `${TITLE_FONT_SIZE}px -apple-system, "Helvetica Neue", Arial, sans-serif`;
    ctx.textAlign = "center";
    ctx.textBaseline = "top";
    ctx.fillText(chart.title, rect.x + rect.w / 2, cursorY);
    cursorY += TITLE_FONT_SIZE + TITLE_PAD;
  }

  // Legend strip (bottom default)
  const legendH = chart.series.length > 0 ? LEGEND_FONT_SIZE + 14 : 0;
  const legendRect: Rect = {
    x: rect.x,
    y: rect.y + rect.h - legendH,
    w: rect.w,
    h: legendH,
  };

  const plotRect: Rect = {
    x: rect.x + PLOT_PAD_LEFT,
    y: cursorY,
    w: rect.w - PLOT_PAD_LEFT - PLOT_PAD_RIGHT,
    h: rect.y + rect.h - cursorY - legendH - 4,
  };
  if (plotRect.w <= 20 || plotRect.h <= 20) return;

  switch (chart.type) {
    case "column":
    case "bar":
      drawBarColumnChart(ctx, chart, plotRect);
      break;
    case "line":
      drawLineChart(ctx, chart, plotRect);
      break;
    case "area":
      drawAreaChart(ctx, chart, plotRect);
      break;
    case "pie":
    case "doughnut":
      drawPieChart(ctx, chart, plotRect);
      break;
    case "scatter":
      drawScatterChart(ctx, chart, plotRect);
      break;
    default:
      drawPlaceholderPlot(ctx, chart, plotRect);
  }

  if (legendH > 0) drawLegend(ctx, chart.series, legendRect);
}

// ---------- bar/column ----------

function drawBarColumnChart(ctx: CanvasRenderingContext2D, chart: Chart, rect: Rect): void {
  const horizontal = chart.type === "bar";
  const stacked = chart.grouping === "stacked" || chart.grouping === "percentstacked";

  const series = chart.series.filter((s) => s.values.length > 0);
  if (series.length === 0) {
    drawPlaceholderPlot(ctx, chart, rect);
    return;
  }
  const categoryCount = Math.max(...series.map((s) => s.values.length), chart.categories.length);
  if (categoryCount === 0) return;

  // Compute value range.
  let minV = 0,
    maxV = 0;
  if (stacked) {
    for (let i = 0; i < categoryCount; i++) {
      let pos = 0,
        neg = 0;
      for (const s of series) {
        const v = s.values[i] ?? 0;
        if (v >= 0) pos += v;
        else neg += v;
      }
      if (pos > maxV) maxV = pos;
      if (neg < minV) minV = neg;
    }
  } else {
    for (const s of series) {
      for (const v of s.values) {
        if (v > maxV) maxV = v;
        if (v < minV) minV = v;
      }
    }
  }
  // Always include zero.
  if (minV > 0) minV = 0;
  if (maxV < 0) maxV = 0;
  if (minV === 0 && maxV === 0) maxV = 1;
  const ticks = niceTicks(minV, maxV, AXIS_TICK_COUNT);
  minV = ticks[0]!;
  maxV = ticks[ticks.length - 1]!;

  // Measure the value-axis label width so we can carve out a y-axis gutter.
  ctx.font = `${AXIS_FONT_SIZE}px -apple-system, "Helvetica Neue", Arial, sans-serif`;
  const labelStrings = ticks.map((t) => formatAxisValue(t, chart.valueFormat));
  const yAxisW = Math.max(...labelStrings.map((s) => ctx.measureText(s).width)) + 8;
  const xAxisH = AXIS_FONT_SIZE + 8;

  const innerRect: Rect = horizontal
    ? { x: rect.x + yAxisW, y: rect.y, w: rect.w - yAxisW, h: rect.h - xAxisH }
    : { x: rect.x + yAxisW, y: rect.y, w: rect.w - yAxisW, h: rect.h - xAxisH };

  // Gridlines + value-axis labels
  ctx.fillStyle = AXIS_LABEL_COLOR;
  ctx.strokeStyle = GRIDLINE_COLOR;
  ctx.lineWidth = 1;
  ctx.textAlign = horizontal ? "center" : "right";
  ctx.textBaseline = "middle";
  for (let ti = 0; ti < ticks.length; ti++) {
    const t = ticks[ti]!;
    const frac = (t - minV) / (maxV - minV);
    if (horizontal) {
      const x = innerRect.x + frac * innerRect.w;
      ctx.beginPath();
      ctx.moveTo(Math.round(x) + 0.5, innerRect.y);
      ctx.lineTo(Math.round(x) + 0.5, innerRect.y + innerRect.h);
      ctx.stroke();
      ctx.fillText(labelStrings[ti]!, x, innerRect.y + innerRect.h + xAxisH / 2);
    } else {
      const y = innerRect.y + (1 - frac) * innerRect.h;
      ctx.beginPath();
      ctx.moveTo(innerRect.x, Math.round(y) + 0.5);
      ctx.lineTo(innerRect.x + innerRect.w, Math.round(y) + 0.5);
      ctx.stroke();
      ctx.fillText(labelStrings[ti]!, innerRect.x - 4, y);
    }
  }

  // Bars
  const groupSize = stacked ? 1 : series.length;
  const groupGap = horizontal ? innerRect.h / categoryCount : innerRect.w / categoryCount;
  const barGapFrac = 0.25; // share of group reserved for between-group spacing
  const innerGapFrac = 0.05; // between bars in a group (clustered only)
  const usableGroup = groupGap * (1 - barGapFrac);
  const barSize = stacked
    ? usableGroup
    : (usableGroup * (1 - innerGapFrac * (groupSize - 1))) / groupSize;

  // Precompute zero baseline.
  const zeroFrac = (0 - minV) / (maxV - minV);
  const zeroY = innerRect.y + (1 - zeroFrac) * innerRect.h;

  // Category labels along axis.
  ctx.fillStyle = AXIS_LABEL_COLOR;
  ctx.textAlign = "center";
  ctx.textBaseline = horizontal ? "middle" : "top";
  for (let i = 0; i < categoryCount; i++) {
    const center = horizontal
      ? innerRect.y + (i + 0.5) * groupGap
      : innerRect.x + (i + 0.5) * groupGap;
    const label = chart.categories[i] ?? `${i + 1}`;
    if (horizontal) {
      ctx.textAlign = "right";
      ctx.fillText(label, innerRect.x - 4, center);
    } else {
      ctx.fillText(label, center, innerRect.y + innerRect.h + 4);
    }
  }
  ctx.textAlign = "left";

  // Draw bars
  if (stacked) {
    for (let i = 0; i < categoryCount; i++) {
      const groupCenter = horizontal
        ? innerRect.y + (i + 0.5) * groupGap
        : innerRect.x + (i + 0.5) * groupGap;
      let pos = 0,
        neg = 0;
      // Per-category total for showPercent (positive contributions only,
      // matching Excel for stacked bars).
      let catTotal = 0;
      for (const s of series) catTotal += Math.max(0, s.values[i] ?? 0);
      for (const s of series) {
        const v = s.values[i] ?? 0;
        const start = v >= 0 ? pos : neg;
        const end = v >= 0 ? pos + v : neg + v;
        if (v >= 0) pos += v;
        else neg += v;
        const sFrac = (start - minV) / (maxV - minV);
        const eFrac = (end - minV) / (maxV - minV);
        ctx.fillStyle = s.color ?? "#4472C4";
        let bx = 0,
          by = 0,
          bw = 0,
          bh = 0;
        if (horizontal) {
          const xa = innerRect.x + sFrac * innerRect.w;
          const xb = innerRect.x + eFrac * innerRect.w;
          bx = Math.min(xa, xb);
          by = groupCenter - barSize / 2;
          bw = Math.abs(xb - xa);
          bh = barSize;
        } else {
          const ya = innerRect.y + (1 - sFrac) * innerRect.h;
          const yb = innerRect.y + (1 - eFrac) * innerRect.h;
          bx = groupCenter - barSize / 2;
          by = Math.min(ya, yb);
          bw = barSize;
          bh = Math.abs(yb - ya);
        }
        ctx.fillRect(bx, by, bw, bh);
        // Stacked label: position default `ctr` (in-bar center).
        const dl = effectiveLabels(chart, s);
        if (dl) {
          const text = buildLabelText(dl, chart, s, i, v, catTotal);
          drawLabel(ctx, text, bx + bw / 2, by + bh / 2);
        }
      }
    }
  } else {
    for (let i = 0; i < categoryCount; i++) {
      for (let si = 0; si < series.length; si++) {
        const s = series[si]!;
        const v = s.values[i] ?? 0;
        const frac = (v - minV) / (maxV - minV);
        ctx.fillStyle = s.color ?? "#4472C4";
        let bx = 0,
          by = 0,
          bw = 0,
          bh = 0;
        if (horizontal) {
          const groupTop = innerRect.y + i * groupGap + (groupGap - usableGroup) / 2;
          const top = groupTop + si * (barSize + barSize * innerGapFrac);
          const x1 = innerRect.x + ((0 - minV) / (maxV - minV)) * innerRect.w;
          const x2 = innerRect.x + frac * innerRect.w;
          bx = Math.min(x1, x2);
          by = top;
          bw = Math.abs(x2 - x1);
          bh = barSize;
        } else {
          const groupLeft = innerRect.x + i * groupGap + (groupGap - usableGroup) / 2;
          const left = groupLeft + si * (barSize + barSize * innerGapFrac);
          const yTop = innerRect.y + (1 - frac) * innerRect.h;
          const yBot = zeroY;
          bx = left;
          by = Math.min(yTop, yBot);
          bw = barSize;
          bh = Math.abs(yBot - yTop);
        }
        ctx.fillRect(bx, by, bw, bh);
        const dl = effectiveLabels(chart, s);
        if (dl) {
          const text = buildLabelText(dl, chart, s, i, v, /*catTotal=*/ 0);
          // Default position: outEnd. `inEnd`/`ctr`/`inBase` honored.
          const pos = dl.position ?? "outEnd";
          let lx = bx + bw / 2,
            ly = by + bh / 2;
          const PAD = 3;
          if (horizontal) {
            // value axis runs left-right.
            if (pos === "outEnd") {
              lx = v >= 0 ? bx + bw + PAD : bx - PAD;
            } else if (pos === "inEnd") {
              lx = v >= 0 ? bx + bw - PAD : bx + PAD;
            } else if (pos === "inBase") {
              lx = v >= 0 ? bx + PAD : bx + bw - PAD;
            }
            const align: CanvasTextAlign =
              pos === "outEnd"
                ? v >= 0
                  ? "left"
                  : "right"
                : pos === "inEnd"
                  ? v >= 0
                    ? "right"
                    : "left"
                  : pos === "inBase"
                    ? v >= 0
                      ? "left"
                      : "right"
                    : "center";
            drawLabel(ctx, text, lx, ly, align, "middle");
          } else {
            // value axis runs top-bottom.
            if (pos === "outEnd") {
              ly = v >= 0 ? by - PAD : by + bh + PAD;
            } else if (pos === "inEnd") {
              ly = v >= 0 ? by + PAD : by + bh - PAD;
            } else if (pos === "inBase") {
              ly = v >= 0 ? by + bh - PAD : by + PAD;
            }
            const baseline: CanvasTextBaseline =
              pos === "outEnd"
                ? v >= 0
                  ? "bottom"
                  : "top"
                : pos === "inEnd"
                  ? v >= 0
                    ? "top"
                    : "bottom"
                  : pos === "inBase"
                    ? v >= 0
                      ? "bottom"
                      : "top"
                    : "middle";
            drawLabel(ctx, text, lx, ly, "center", baseline);
          }
        }
      }
    }
  }

  // Axis baselines.
  ctx.strokeStyle = "#9ca3af";
  ctx.beginPath();
  ctx.moveTo(innerRect.x, Math.round(zeroY) + 0.5);
  ctx.lineTo(innerRect.x + innerRect.w, Math.round(zeroY) + 0.5);
  ctx.moveTo(Math.round(innerRect.x) + 0.5, innerRect.y);
  ctx.lineTo(Math.round(innerRect.x) + 0.5, innerRect.y + innerRect.h);
  ctx.stroke();
}

// ---------- line ----------
//
// Standard / stacked / percentStacked. Stacked is per-category cumulative;
// percentStacked normalises each category column to 100. Categories are
// equispaced on the x-axis.

function drawLineChart(ctx: CanvasRenderingContext2D, chart: Chart, rect: Rect): void {
  const series = chart.series.filter((s) => s.values.length > 0);
  if (series.length === 0) {
    drawPlaceholderPlot(ctx, chart, rect);
    return;
  }
  const categoryCount = Math.max(...series.map((s) => s.values.length), chart.categories.length);
  if (categoryCount === 0) return;

  const stacked = chart.grouping === "stacked" || chart.grouping === "percentstacked";
  const percent = chart.grouping === "percentstacked";

  // Cumulative per-category stacks (only used for stacked / percentStacked).
  const stackedSeries: number[][] = stacked
    ? buildStackedRows(series, categoryCount, percent)
    : series.map((s) => Array.from({ length: categoryCount }, (_, i) => s.values[i] ?? 0));

  let { minV, maxV } = valueRange(stackedSeries);
  if (minV > 0) minV = 0;
  if (maxV < 0) maxV = 0;
  if (minV === maxV) {
    maxV = minV + 1;
  }
  const ticks = niceTicks(minV, maxV, AXIS_TICK_COUNT);
  minV = ticks[0]!;
  maxV = ticks[ticks.length - 1]!;

  const inner = drawAxisFrame(ctx, chart, rect, ticks, minV, maxV, /*horizontal=*/ false, percent);

  // Category x-axis labels.
  drawCategoryAxis(ctx, chart, inner, categoryCount, /*horizontal=*/ false);

  const xStep = inner.w / Math.max(1, categoryCount - 1);
  const yFor = (v: number) => inner.y + (1 - (v - minV) / (maxV - minV)) * inner.h;

  for (let si = 0; si < series.length; si++) {
    const s = series[si]!;
    const data = stackedSeries[si]!;
    ctx.strokeStyle = s.color ?? "#4472C4";
    ctx.lineWidth = 2;
    ctx.beginPath();
    for (let i = 0; i < categoryCount; i++) {
      const x = inner.x + i * xStep;
      const y = yFor(data[i] ?? 0);
      if (i === 0) ctx.moveTo(x, y);
      else ctx.lineTo(x, y);
    }
    ctx.stroke();
    // Markers (small circles).
    ctx.fillStyle = s.color ?? "#4472C4";
    for (let i = 0; i < categoryCount; i++) {
      const x = inner.x + i * xStep;
      const y = yFor(data[i] ?? 0);
      ctx.beginPath();
      ctx.arc(x, y, 3, 0, Math.PI * 2);
      ctx.fill();
    }
    // Data labels (default position `t` above the marker).
    const dl = effectiveLabels(chart, s);
    if (dl) {
      const pos = dl.position ?? "t";
      const PAD = 5;
      for (let i = 0; i < categoryCount; i++) {
        const v = s.values[i] ?? 0;
        const text = buildLabelText(dl, chart, s, i, v, 0);
        if (!text) continue;
        const x = inner.x + i * xStep;
        const y = yFor(data[i] ?? 0);
        let lx = x,
          ly = y;
        let baseline: CanvasTextBaseline = "bottom";
        if (pos === "b") {
          ly = y + PAD;
          baseline = "top";
        } else if (pos === "ctr") {
          baseline = "middle";
        } else if (pos === "l") {
          lx = x - PAD;
          baseline = "middle";
        } else if (pos === "r") {
          lx = x + PAD;
          baseline = "middle";
        } else {
          ly = y - PAD;
          baseline = "bottom";
        }
        const align: CanvasTextAlign = pos === "l" ? "right" : pos === "r" ? "left" : "center";
        drawLabel(ctx, text, lx, ly, align, baseline);
      }
    }
  }
  ctx.lineWidth = 1;
}

// ---------- area ----------

function drawAreaChart(ctx: CanvasRenderingContext2D, chart: Chart, rect: Rect): void {
  const series = chart.series.filter((s) => s.values.length > 0);
  if (series.length === 0) {
    drawPlaceholderPlot(ctx, chart, rect);
    return;
  }
  const categoryCount = Math.max(...series.map((s) => s.values.length), chart.categories.length);
  if (categoryCount === 0) return;

  const stacked = chart.grouping !== "standard"; // default for area is stacked in Excel
  const percent = chart.grouping === "percentstacked";

  // For stacked area we want per-series cumulative top edges; for unstacked
  // we just plot raw y values from a baseline of 0.
  const tops: number[][] = stacked
    ? buildStackedRows(series, categoryCount, percent)
    : series.map((s) => Array.from({ length: categoryCount }, (_, i) => s.values[i] ?? 0));
  // Bottom of each series's polygon: 0 for the first stacked series; the
  // previous series's top otherwise. Unstacked: always 0.
  const bottoms: number[][] = stacked
    ? series.map((_, si) => (si === 0 ? new Array(categoryCount).fill(0) : tops[si - 1]!.slice()))
    : series.map((_) => new Array(categoryCount).fill(0));

  let { minV, maxV } = valueRange([...tops, ...bottoms]);
  if (minV > 0) minV = 0;
  if (maxV < 0) maxV = 0;
  if (minV === maxV) maxV = minV + 1;
  const ticks = niceTicks(minV, maxV, AXIS_TICK_COUNT);
  minV = ticks[0]!;
  maxV = ticks[ticks.length - 1]!;

  const inner = drawAxisFrame(ctx, chart, rect, ticks, minV, maxV, /*horizontal=*/ false, percent);
  drawCategoryAxis(ctx, chart, inner, categoryCount, /*horizontal=*/ false);

  const xStep = inner.w / Math.max(1, categoryCount - 1);
  const yFor = (v: number) => inner.y + (1 - (v - minV) / (maxV - minV)) * inner.h;

  for (let si = 0; si < series.length; si++) {
    const s = series[si]!;
    const top = tops[si]!;
    const bot = bottoms[si]!;
    ctx.fillStyle = withAlpha(s.color ?? "#4472C4", stacked ? 0.85 : 0.55);
    ctx.beginPath();
    for (let i = 0; i < categoryCount; i++) {
      const x = inner.x + i * xStep;
      const y = yFor(top[i] ?? 0);
      if (i === 0) ctx.moveTo(x, y);
      else ctx.lineTo(x, y);
    }
    for (let i = categoryCount - 1; i >= 0; i--) {
      const x = inner.x + i * xStep;
      const y = yFor(bot[i] ?? 0);
      ctx.lineTo(x, y);
    }
    ctx.closePath();
    ctx.fill();
    // Outline along the top edge.
    ctx.strokeStyle = s.color ?? "#4472C4";
    ctx.lineWidth = 1.5;
    ctx.beginPath();
    for (let i = 0; i < categoryCount; i++) {
      const x = inner.x + i * xStep;
      const y = yFor(top[i] ?? 0);
      if (i === 0) ctx.moveTo(x, y);
      else ctx.lineTo(x, y);
    }
    ctx.stroke();
    // Data labels for area: print at the top edge of each segment.
    const dl = effectiveLabels(chart, s);
    if (dl) {
      const PAD = 4;
      for (let i = 0; i < categoryCount; i++) {
        const v = s.values[i] ?? 0;
        const text = buildLabelText(dl, chart, s, i, v, 0);
        if (!text) continue;
        const x = inner.x + i * xStep;
        const y = yFor(top[i] ?? 0);
        drawLabel(ctx, text, x, y - PAD, "center", "bottom");
      }
    }
  }
  ctx.lineWidth = 1;
}

// ---------- pie / doughnut ----------

function drawPieChart(ctx: CanvasRenderingContext2D, chart: Chart, rect: Rect): void {
  // Pie uses series[0] only; data points become slices, one per category.
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
  const innerR = chart.type === "doughnut" ? r * 0.55 : 0;

  // Excel cycles accents per slice, not per series. When the workbook
  // serialises explicit `<c:dPt>` fills we use those (extractor surfaces
  // them as `series.pointColors[i]`); otherwise we fall back to a fixed
  // 6-color palette mirroring the Office accent ramp.
  const palette = ["#4472C4", "#ED7D31", "#A5A5A5", "#FFC000", "#5B9BD5", "#70AD47"];
  const pointColors = ser.pointColors ?? [];

  // First pass: paint slices. Second pass: paint labels (so labels
  // never sit beneath the next slice's fill on overlap).
  type SliceGeom = { mid: number; idx: number; v: number };
  const slices: SliceGeom[] = [];
  let start = -Math.PI / 2; // 12 o'clock
  for (let i = 0; i < ser.values.length; i++) {
    const v = Math.max(0, ser.values[i] ?? 0);
    if (v <= 0) continue;
    const sweep = (v / total) * Math.PI * 2;
    const end = start + sweep;
    const explicit = pointColors[i];
    ctx.fillStyle = explicit && explicit.length > 0 ? explicit : palette[i % palette.length]!;
    ctx.beginPath();
    ctx.moveTo(cx, cy);
    ctx.arc(cx, cy, r, start, end);
    ctx.closePath();
    ctx.fill();
    // Slice border for separation.
    ctx.strokeStyle = "#ffffff";
    ctx.lineWidth = 1.5;
    ctx.stroke();
    slices.push({ mid: (start + end) / 2, idx: i, v });
    start = end;
  }

  if (innerR > 0) {
    // Punch out the center for a doughnut.
    ctx.fillStyle = "#ffffff";
    ctx.beginPath();
    ctx.arc(cx, cy, innerR, 0, Math.PI * 2);
    ctx.fill();
  }
  ctx.lineWidth = 1;

  // Data labels per slice. `outEnd` (default for pie) places the label
  // just outside the arc; `ctr` / `inEnd` place it inside.
  const dl = effectiveLabels(chart, ser);
  if (dl) {
    const pos = dl.position ?? "outEnd";
    const labelR =
      pos === "outEnd" || pos === "bestFit" ? r + 12 : pos === "ctr" ? (innerR + r) / 2 : r - 12; // inEnd
    for (const sl of slices) {
      const text = buildLabelText(dl, chart, ser, sl.idx, sl.v, total);
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

// ---------- scatter ----------

function drawScatterChart(ctx: CanvasRenderingContext2D, chart: Chart, rect: Rect): void {
  const series = chart.series.filter((s) => s.values.length > 0);
  if (series.length === 0) {
    drawPlaceholderPlot(ctx, chart, rect);
    return;
  }

  // X data: prefer per-series xValues; else parse the chart-level
  // categories array (first series's xVal cache in our extractor).
  const xCache: number[][] = series.map((s) => {
    const xs = (s.xValues ?? []) as number[];
    if (xs.length > 0) return xs.slice();
    // Fallback: index labels from chart.categories, parsed as numbers.
    return s.values.map((_, i) => {
      const c = chart.categories[i];
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

  // Y-axis frame + gridlines.
  const inner = drawAxisFrame(ctx, chart, rect, yTicks, yMin, yMax, /*horizontal=*/ false, false);

  // Numeric x-axis labels (scatter has them; bar/line/area pull from categories).
  ctx.font = `${AXIS_FONT_SIZE}px -apple-system, "Helvetica Neue", Arial, sans-serif`;
  ctx.fillStyle = AXIS_LABEL_COLOR;
  ctx.textAlign = "center";
  ctx.textBaseline = "top";
  for (const t of xTicks) {
    const frac = (t - xMin) / (xMax - xMin);
    const x = inner.x + frac * inner.w;
    ctx.fillText(formatGeneral(t), x, inner.y + inner.h + 4);
  }

  // ECMA-376 §21.2.2.193 ScatterStyle. Excel's UI default for new
  // scatter charts is `marker` only; OOXML enum default is `line`.
  // We treat an *unset* style as marker-only (matches the existing
  // visual contract + Excel UI), and only draw connecting lines /
  // smooth curves when the workbook explicitly asked for one.
  const style = chart.scatterStyle;
  const drawLines = style === "line" || style === "lineMarker";
  const drawSmooth = style === "smooth" || style === "smoothMarker";
  const drawMarkers = style == null || style === "marker" ||
    style === "lineMarker" || style === "smoothMarker";

  // Plot points (and optional connecting lines).
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

    // Project points to canvas space once.
    const pts: { x: number; y: number; v: number; i: number }[] = [];
    for (let i = 0; i < n; i++) {
      const px = inner.x + ((xs[i]! - xMin) / (xMax - xMin)) * inner.w;
      const py = inner.y + (1 - (ys[i]! - yMin) / (yMax - yMin)) * inner.h;
      pts.push({ x: px, y: py, v: ys[i]!, i });
    }

    // Lines connect points in x-sorted order (Excel sorts xy series
    // before stroking; otherwise back-and-forth x produces a tangled
    // path).
    if ((drawLines || drawSmooth) && pts.length >= 2) {
      const sorted = pts.slice().sort((a, b) => a.x - b.x);
      ctx.lineWidth = 1.5;
      ctx.beginPath();
      ctx.moveTo(sorted[0]!.x, sorted[0]!.y);
      if (drawSmooth) {
        // Catmull-Rom -> Bezier (tension 0.5). Robust + monotone in x
        // because input is already x-sorted.
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

    // Markers + per-point labels.
    for (const p of pts) {
      if (drawMarkers) {
        ctx.beginPath();
        ctx.arc(p.x, p.y, 3.5, 0, Math.PI * 2);
        ctx.fill();
      }
      if (dl) {
        const text = buildLabelText(dl, chart, s, p.i, p.v, 0);
        if (text) drawLabel(ctx, text, p.x, p.y - 6, "center", "bottom");
      }
    }
  }
}
