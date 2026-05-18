// stockChart (`c:stockChart`, ECMA-376 §21.2.2.198) painter. Split
// out of `chartAdvanced.ts` to keep that file under its per-file LOC
// budget once chartEx layouts landed in `chartEx.ts`.

import type { Chart } from "./types.js";
import type { Rect } from "./chart.js";
import {
  buildLabelText,
  drawAxisFrame,
  drawLabel,
  drawPlaceholderPlot,
  effectiveLabels,
  formatAxisValue,
  paintZeroBaseline,
  pointLabel,
  resolveAxisRange,
  valueRange,
  withAlpha,
} from "./chartUtils.js";

const AXIS_FONT_SIZE = 10;
const AXIS_LABEL_COLOR = "#52525b";
const AXIS_TICK_COUNT = 5;

// ---------- stock ----------

/**
 * Stock chart painter (ECMA-376 §21.2.2.198). Series count implies
 * subtype:
 *   3 → High-Low-Close (HLC)        — series [high, low, close]
 *   4 → Open-High-Low-Close (OHLC)  — series [open, high, low, close]
 *      (also Volume-High-Low-Close if a parallel `<c:barChart>` carries
 *      volume; the combo path handles that case, not this painter)
 *   5 → Volume-Open-High-Low-Close  — series [volume, open, high, low, close]
 *
 * Decoration toggles (extracted from `<c:hiLowLines/>` / `<c:upDownBars/>`):
 *   - `stockHiLowLines`: vertical line from category low to high.
 *   - `stockUpDownBars`: rectangle between open and close, white-filled
 *     for up days (close ≥ open), black-filled for down days. Only
 *     meaningful for OHLC/VOHLC (need open + close).
 *
 * Per-series `<c:spPr><a:ln><a:noFill/></a:ln></c:spPr>` on high/low
 * (xlsxwriter's default) means we should *not* connect those points
 * with a line. We honor that by simply never drawing a per-series
 * polyline — the hi-low lines + markers carry the visual.
 */
export function drawStockChart(ctx: CanvasRenderingContext2D, chart: Chart, rect: Rect): void {
  const series = chart.series.filter((s) => s.values.length > 0);
  if (series.length < 2) {
    drawPlaceholderPlot(ctx, chart, rect);
    return;
  }
  const cats = chart.categories ?? [];
  const categoryCount = Math.max(...series.map((s) => s.values.length), cats.length);
  if (categoryCount === 0) return;

  // Subtype dispatch by series count. We assume the OOXML series order
  // matches Excel's convention (xlsxwriter / Excel both emit in this
  // order). When the workbook authored a parallel `<c:barChart>` for
  // volume, those series ride the combo path instead.
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
    // 2 series — treat as High/Low only.
    [highIdx, lowIdx] = [0, 1];
  }

  // If we have a volume series, carve off a bottom band (≈22% of the
  // plot rect) for it so price + volume don't share a y-scale. Excel
  // does this with two value axes; we approximate with a split rect.
  let priceRect: Rect = rect;
  let volumeRect: Rect | null = null;
  if (volumeIdx >= 0) {
    const VOL_FRACTION = 0.22;
    const VOL_GAP = 4;
    const volH = Math.max(40, rect.h * VOL_FRACTION);
    priceRect = { x: rect.x, y: rect.y, w: rect.w, h: rect.h - volH - VOL_GAP };
    volumeRect = { x: rect.x, y: rect.y + rect.h - volH, w: rect.w, h: volH };
  }

  // Build value rows for the price-axis range. Price uses all
  // non-volume series. We don't zero-clamp — stocks rarely touch
  // zero, and forcing it wastes most of the band.
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
    /*zeroClamp=*/ false,
    AXIS_TICK_COUNT,
    chart.majorUnit,
  );
  minV = range.minV;
  maxV = range.maxV;
  const ticks = range.ticks;

  const inner = drawAxisFrame(
    ctx,
    chart,
    priceRect,
    ticks,
    minV,
    maxV,
    /*horizontal=*/ false,
    /*percent=*/ false,
  );

  // Stock charts use bar-style category placement (Excel centers the
  // hi-low marks on the category, not on the boundary). Mirror the
  // bar painter's denom: `i + 0.5` over `n` slots.
  const slotW = inner.w / categoryCount;
  const xFor = (i: number) => inner.x + (i + 0.5) * slotW;
  const yFor = (v: number) => inner.y + (1 - (v - minV) / (maxV - minV)) * inner.h;

  // Category axis labels (bar-style: i+0.5 / n centers).
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

  // Clip to plot area; up/down bars and hi-low lines outside the
  // value range get cropped instead of bleeding into the axis.
  ctx.save();
  ctx.beginPath();
  ctx.rect(inner.x, inner.y, inner.w, inner.h);
  ctx.clip();

  // 1. Hi-low lines (one vertical segment per category).
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

  // 2. Up/down bars (open→close), white-filled when close ≥ open
  // ("up day") and black-filled when close < open ("down day").
  // Excel's default bar width is 150% of the gap — we use 55% of
  // the slot, which reads well across category counts.
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

  // 3. Drop lines (rare): connect each value point straight down to
  // the category axis. Authored via `<c:dropLines/>`. We draw them
  // from each series's value at i down to the inner baseline.
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

  // 4. Per-series markers. xlsxwriter authors `<c:marker><c:symbol
  // val="none"/></c:marker>` on high/low and `<c:symbol val="dot"/>`
  // on close — markerSymbol=="none" suppresses, anything else paints.
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

  // 5. Data labels (optional). Same default position as line: above.
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

  // 6. Volume sub-plot (when series.length >= 5). Painted as a column
  // chart sharing the price-chart's category axis. Color: theme accent
  // 1 at low alpha so it visually subordinates to price.
  if (volumeIdx >= 0 && volumeRect) {
    const volSeries = series[volumeIdx]!;
    const volRows = [
      Array.from({ length: categoryCount }, (_, i) => {
        const v = volSeries.values[i];
        return v != null && Number.isFinite(v) ? v : 0;
      }),
    ];
    const { maxV: vMax } = valueRange(volRows);
    const vRange = resolveAxisRange(0, vMax, 0, undefined, /*zeroClamp=*/ true, 2);
    const vInner = drawAxisFrame(
      ctx,
      chart,
      volumeRect,
      vRange.ticks,
      vRange.minV,
      vRange.maxV,
      /*horizontal=*/ false,
      /*percent=*/ false,
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
