import type { Chart } from "./types.js";
import {
  buildLabelText,
  buildStackedRows,
  drawAxisFrame,
  drawCategoryAxis,
  drawLabel,
  drawPlaceholderPlot,
  effectiveLabels,
  paintZeroBaseline,
  pointLabel,
  resolveAxisRange,
  valueRange,
  withAlpha,
} from "./chartUtils.js";
import type { Rect } from "./chart.js";

const AXIS_TICK_COUNT = 5;

// ---------- area ----------

export function drawAreaChart(ctx: CanvasRenderingContext2D, chart: Chart, rect: Rect): void {
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
  // Area uses zero baseline by convention, so zero-clamp unless the
  // workbook overrode with explicit scaling bounds.
  const _aRange = resolveAxisRange(
    minV,
    maxV,
    chart.valueMin,
    chart.valueMax,
    /*zeroClamp=*/ true,
    AXIS_TICK_COUNT,
    chart.majorUnit,
  );
  minV = _aRange.minV;
  maxV = _aRange.maxV;
  const ticks = _aRange.ticks;

  const inner = drawAxisFrame(ctx, chart, rect, ticks, minV, maxV, /*horizontal=*/ false, percent);
  drawCategoryAxis(ctx, chart, inner, categoryCount, /*horizontal=*/ false);

  const xStep = inner.w / Math.max(1, categoryCount - 1);
  const yFor = (v: number) => inner.y + (1 - (v - minV) / (maxV - minV)) * inner.h;

  // Clip area fills + outline strokes to the plot rect so out-of-range
  // peaks (data > pinned `<c:max>`) don't spill above the topmost
  // gridline. Matches Excel's behavior; mirrors the line/bar paths.
  ctx.save();
  ctx.beginPath();
  ctx.rect(inner.x, inner.y, inner.w, inner.h);
  ctx.clip();

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
        const po = pointLabel(dl, i);
        if (po === null) continue;
        const edl = po?.dl ?? dl;
        const v = s.values[i] ?? 0;
        const text = po?.text ?? buildLabelText(edl, chart, s, i, v, 0);
        if (!text) continue;
        const x = inner.x + i * xStep;
        const y = yFor(top[i] ?? 0);
        drawLabel(ctx, text, x, y - PAD, "center", "bottom");
      }
    }
  }
  ctx.restore();
  ctx.lineWidth = 1;
  // Bug #13 step 1: heavier zero baseline when the axis straddles zero.
  paintZeroBaseline(ctx, inner, minV, maxV);
}
