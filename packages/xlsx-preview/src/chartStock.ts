import type { Chart } from "./types.js";
import type { Rect } from "./chart.js";
import {
  buildLabelText,
  drawAxisFrame,
  drawCategoryAxisExtraRowsCentered,
  drawLabel,
  drawPlaceholderPlot,
  effectiveLabels,
  formatAxisValue,
  paintZeroBaseline,
  pointLabel,
  resolveAxisRange,
  valueRange,
  withAlpha,
  AXIS_FONT_SIZE,
} from "./chartUtils.js";

const AXIS_LABEL_COLOR = "#52525b";
const AXIS_TICK_COUNT = 5;

export function drawStockChart(ctx: CanvasRenderingContext2D, chart: Chart, rect: Rect): void {
  const series = chart.series.filter((s) => s.values.length > 0);
  if (series.length < 2) {
    drawPlaceholderPlot(ctx, chart, rect);
    return;
  }
  const cats = chart.categories ?? [];
  const categoryCount = Math.max(...series.map((s) => s.values.length), cats.length);
  if (categoryCount === 0) return;

  let openIdx = -1;
  let highIdx = -1;
  let lowIdx = -1;
  let closeIdx = -1;
  let volumeIdx = -1;
  if (series.length === 3) {
    [highIdx, lowIdx, closeIdx] = [0, 1, 2];
  } else if (series.length === 4) {
    [openIdx, highIdx, lowIdx, closeIdx] = [0, 1, 2, 3];
  } else if (series.length >= 5) {
    [volumeIdx, openIdx, highIdx, lowIdx, closeIdx] = [0, 1, 2, 3, 4];
  } else {
    [highIdx, lowIdx] = [0, 1];
  }

  let priceRect: Rect = rect;
  let volumeRect: Rect | null = null;
  if (volumeIdx >= 0) {
    const VOL_FRACTION = 0.22;
    const VOL_GAP = 4;
    const volH = Math.max(40, rect.h * VOL_FRACTION);
    priceRect = { x: rect.x, y: rect.y, w: rect.w, h: rect.h - volH - VOL_GAP };
    volumeRect = { x: rect.x, y: rect.y + rect.h - volH, w: rect.w, h: volH };
  }

  const priceSeries = series.filter((_, i) => i !== volumeIdx);
  const priceRows = priceSeries.map((s) =>
    Array.from({ length: categoryCount }, (_, i) => s.values[i] ?? NaN),
  );
  let { minV, maxV } = valueRange(priceRows.map((r) => r.filter((v) => Number.isFinite(v))));
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

  const inner = drawAxisFrame(ctx, chart, priceRect, ticks, minV, maxV, false, false);

  const slotW = inner.w / categoryCount;
  const xFor = (i: number) => inner.x + (i + 0.5) * slotW;
  const yFor = (v: number) => inner.y + (1 - (v - minV) / (maxV - minV)) * inner.h;

  ctx.font = `${AXIS_FONT_SIZE}px -apple-system, "Helvetica Neue", Arial, sans-serif`;
  ctx.fillStyle = AXIS_LABEL_COLOR;
  ctx.textAlign = "center";
  ctx.textBaseline = "top";
  const fmt = chart.categoriesFormat;
  const labels = Array.from({ length: categoryCount }, (_, i) => {
    const raw = cats[i] ?? `${i + 1}`;
    if (!fmt) return raw;
    const n = parseFloat(raw);
    if (!Number.isFinite(n)) return raw;
    return formatAxisValue(n, fmt);
  });
  {
    let lastRight = -Infinity;
    for (let i = 0; i < categoryCount; i++) {
      const label = labels[i]!;
      const w = ctx.measureText(label).width;
      const cx = xFor(i);
      const left = cx - w / 2;
      if (left < lastRight + 8) continue;
      ctx.fillText(label, cx, inner.y + inner.h + 4);
      lastRight = cx + w / 2;
    }
  }
  drawCategoryAxisExtraRowsCentered(ctx, chart, inner, categoryCount, xFor);

  ctx.save();
  ctx.beginPath();
  ctx.rect(inner.x, inner.y, inner.w, inner.h);
  ctx.clip();

  if (chart.stockHiLowLines && highIdx >= 0 && lowIdx >= 0) {
    ctx.strokeStyle = "#262626";
    ctx.lineWidth = 1;
    for (let i = 0; i < categoryCount; i++) {
      const hi = series[highIdx]!.values[i];
      const lo = series[lowIdx]!.values[i];
      if (hi == null || lo == null || !Number.isFinite(hi) || !Number.isFinite(lo)) continue;
      const x = xFor(i);
      ctx.beginPath();
      ctx.moveTo(x, yFor(Math.max(hi, lo)));
      ctx.lineTo(x, yFor(Math.min(hi, lo)));
      ctx.stroke();
    }
  }

  if (chart.stockUpDownBars && openIdx >= 0 && closeIdx >= 0) {
    const barW = Math.max(2, slotW * 0.55);
    for (let i = 0; i < categoryCount; i++) {
      const o = series[openIdx]!.values[i];
      const c = series[closeIdx]!.values[i];
      if (o == null || c == null || !Number.isFinite(o) || !Number.isFinite(c)) continue;
      const up = c >= o;
      const top = yFor(Math.max(o, c));
      const bot = yFor(Math.min(o, c));
      const x = xFor(i) - barW / 2;
      const h = Math.max(1, bot - top);
      ctx.fillStyle = up ? "#ffffff" : "#262626";
      ctx.strokeStyle = "#262626";
      ctx.lineWidth = 1;
      ctx.fillRect(x, top, barW, h);
      ctx.strokeRect(x + 0.5, top + 0.5, barW - 1, h - 1);
    }
  }

  if (chart.stockDropLines) {
    ctx.strokeStyle = "#a3a3a3";
    ctx.lineWidth = 0.5;
    for (let si = 0; si < series.length; si++) {
      if (si === volumeIdx) continue;
      const s = series[si]!;
      for (let i = 0; i < categoryCount; i++) {
        const v = s.values[i];
        if (v == null || !Number.isFinite(v)) continue;
        const x = xFor(i);
        ctx.beginPath();
        ctx.moveTo(x, yFor(v));
        ctx.lineTo(x, inner.y + inner.h);
        ctx.stroke();
      }
    }
  }

  for (let si = 0; si < series.length; si++) {
    if (si === volumeIdx) continue;
    const s = series[si]!;
    if (s.markerSymbol === "none") continue;
    ctx.fillStyle = s.color ?? "#262626";
    for (let i = 0; i < categoryCount; i++) {
      const v = s.values[i];
      if (v == null || !Number.isFinite(v)) continue;
      ctx.beginPath();
      ctx.arc(xFor(i), yFor(v), 3, 0, Math.PI * 2);
      ctx.fill();
    }
  }

  for (let si = 0; si < series.length; si++) {
    if (si === volumeIdx) continue;
    const s = series[si]!;
    const dl = effectiveLabels(chart, s);
    if (!dl) continue;
    for (let i = 0; i < categoryCount; i++) {
      const v = s.values[i];
      if (v == null || !Number.isFinite(v)) continue;
      const po = pointLabel(dl, i);
      if (po === null) continue;
      const edl = po?.dl ?? dl;
      const text = po?.text ?? buildLabelText(edl, chart, s, i, v, 0);
      if (!text) continue;
      drawLabel(ctx, text, xFor(i), yFor(v) - 5, "center", "bottom");
    }
  }

  ctx.restore();
  paintZeroBaseline(ctx, inner, minV, maxV);

  if (volumeIdx >= 0 && volumeRect) {
    const volSeries = series[volumeIdx]!;
    const volRows = [
      Array.from({ length: categoryCount }, (_, i) => {
        const v = volSeries.values[i];
        return v != null && Number.isFinite(v) ? v : 0;
      }),
    ];
    const { maxV: vMax } = valueRange(volRows);
    const vRange = resolveAxisRange(0, vMax, 0, undefined, 2);
    const vInner = drawAxisFrame(
      ctx,
      chart,
      volumeRect,
      vRange.ticks,
      vRange.minV,
      vRange.maxV,
      false,
      false,
    );
    const vSlotW = vInner.w / categoryCount;
    const vBarW = Math.max(2, vSlotW * 0.7);
    const yV = (v: number) =>
      vInner.y + (1 - (v - vRange.minV) / (vRange.maxV - vRange.minV || 1)) * vInner.h;
    ctx.fillStyle = withAlpha(volSeries.color ?? "#4472C4", 0.65);
    for (let i = 0; i < categoryCount; i++) {
      const v = volRows[0]![i]!;
      if (!Number.isFinite(v) || v <= 0) continue;
      const x = vInner.x + (i + 0.5) * vSlotW - vBarW / 2;
      const y = yV(v);
      ctx.fillRect(x, y, vBarW, vInner.y + vInner.h - y);
    }
  }
}
