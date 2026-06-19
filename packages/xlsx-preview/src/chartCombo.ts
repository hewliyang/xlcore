import type { Chart, ChartSeries } from "./types.js";
import type { Rect } from "./chart.js";
import { resolveBarFill } from "./chartAdvanced.js";
import { advancedPointFill } from "./chartFills.js";
import {
  buildLabelText,
  categoryAxisExtraHeight,
  computeBarSlotMetrics,
  drawCategoryAxisExtraRowsCentered,
  drawLabel,
  drawPlaceholderPlot,
  effectiveLabels,
  formatAxisValue,
  isZeroTickInside,
  paintZeroBaseline,
  pointLabel,
  resolveAxisRange,
  seriesLineDash,
  seriesLineWidth,
  withAlpha,
  AXIS_FONT_SIZE,
} from "./chartUtils.js";

const AXIS_LABEL_COLOR = "#52525b";
const AXIS_TICK_COUNT = 5;
const GRIDLINE_COLOR = "#e5e7eb";

export function drawComboChart(ctx: CanvasRenderingContext2D, chart: Chart, rect: Rect): void {
  const allSeries = chart.series.filter((s) => s.values.length > 0);
  if (allSeries.length === 0) {
    drawPlaceholderPlot(ctx, chart, rect);
    return;
  }
  const categoryCount = Math.max(
    ...allSeries.map((s) => s.values.length),
    (chart.categories ?? []).length,
  );
  if (categoryCount === 0) return;

  const primarySeries = allSeries.filter((s) => s.axisGroup !== "secondary");
  const secondarySeries = allSeries.filter((s) => s.axisGroup === "secondary");

  const seriesKind = (s: ChartSeries): string =>
    s.chartType ?? (chart.type === "combo" ? "column" : chart.type);

  function rangeFor(side: ChartSeries[]): { minV: number; maxV: number } {
    let minV = Number.POSITIVE_INFINITY,
      maxV = Number.NEGATIVE_INFINITY;
    const bars = side.filter((s) => {
      const k = seriesKind(s);
      return k === "column" || k === "bar";
    });
    const others = side.filter((s) => !bars.includes(s));
    const stacked = chart.grouping === "stacked" || chart.grouping === "percentstacked";
    if (stacked && bars.length > 1) {
      for (let i = 0; i < categoryCount; i++) {
        let pos = 0,
          neg = 0;
        for (const s of bars) {
          const v = s.values[i] ?? 0;
          if (v >= 0) pos += v;
          else neg += v;
        }
        if (pos > maxV) maxV = pos;
        if (neg < minV) minV = neg;
      }
    } else {
      for (const s of bars) {
        for (const v of s.values) {
          if (v > maxV) maxV = v;
          if (v < minV) minV = v;
        }
      }
    }
    for (const s of others) {
      for (const v of s.values) {
        if (v > maxV) maxV = v;
        if (v < minV) minV = v;
      }
    }

    if (!Number.isFinite(minV)) minV = 0;
    if (!Number.isFinite(maxV)) maxV = 1;

    if (minV === maxV) maxV = minV + 1;
    return { minV, maxV };
  }

  const primaryRange = rangeFor(primarySeries);
  const secondaryRange = secondarySeries.length > 0 ? rangeFor(secondarySeries) : null;

  const axisHasBaselineSeries = (group: typeof primarySeries) =>
    group.some((s) => {
      const k = seriesKind(s);
      return k === "column" || k === "bar" || k === "area";
    });
  const primaryHasBars = axisHasBaselineSeries(primarySeries);
  const secondaryHasBars = axisHasBaselineSeries(secondarySeries);
  const pResolved = resolveAxisRange(
    primaryRange.minV,
    primaryRange.maxV,
    chart.valueMin,
    chart.valueMax,
    AXIS_TICK_COUNT,
    chart.majorUnit,
  );
  const primaryTicks = pResolved.ticks;
  const pMin = pResolved.minV;
  const pMax = pResolved.maxV;
  const sResolved = secondaryRange
    ? resolveAxisRange(
        secondaryRange.minV,
        secondaryRange.maxV,
        chart.valueMinSecondary,
        chart.valueMaxSecondary,
        AXIS_TICK_COUNT,
        chart.majorUnitSecondary,
      )
    : null;
  const secondaryTicks = sResolved ? sResolved.ticks : null;
  const sMin = sResolved ? sResolved.minV : 0;
  const sMax = sResolved ? sResolved.maxV : 1;

  ctx.font = `${AXIS_FONT_SIZE}px -apple-system, "Helvetica Neue", Arial, sans-serif`;
  const pLabels = primaryTicks.map((t) => formatAxisValue(t, chart.valueFormat, chart.dispUnits));
  const sLabels = secondaryTicks
    ? secondaryTicks.map((t) =>
        formatAxisValue(
          t,
          chart.valueFormatSecondary ?? chart.valueFormat,
          chart.dispUnitsSecondary,
        ),
      )
    : [];
  const leftGutter = Math.max(...pLabels.map((s) => ctx.measureText(s).width)) + 8;
  const rightGutter =
    sLabels.length > 0 ? Math.max(...sLabels.map((s) => ctx.measureText(s).width)) + 8 : 4;
  const xAxisH = AXIS_FONT_SIZE + 8 + categoryAxisExtraHeight(chart);

  const inner: Rect = {
    x: rect.x + leftGutter,
    y: rect.y,
    w: rect.w - leftGutter - rightGutter,
    h: rect.h - xAxisH,
  };

  const showPrimaryGridlines = chart.showMajorGridlines !== false;
  ctx.fillStyle = AXIS_LABEL_COLOR;
  ctx.strokeStyle = GRIDLINE_COLOR;
  ctx.lineWidth = 1;
  ctx.textAlign = "right";
  ctx.textBaseline = "middle";
  for (let ti = 0; ti < primaryTicks.length; ti++) {
    const t = primaryTicks[ti]!;
    const frac = (t - pMin) / (pMax - pMin);
    const y = inner.y + (1 - frac) * inner.h;

    const isZeroLine = isZeroTickInside(t, pMin, pMax);
    if (showPrimaryGridlines && !isZeroLine) {
      ctx.beginPath();
      ctx.moveTo(inner.x, Math.round(y) + 0.5);
      ctx.lineTo(inner.x + inner.w, Math.round(y) + 0.5);
      ctx.stroke();
    }
    ctx.fillText(pLabels[ti]!, inner.x - 4, y);
  }

  if (secondaryTicks) {
    ctx.textAlign = "left";
    for (let ti = 0; ti < secondaryTicks.length; ti++) {
      const t = secondaryTicks[ti]!;
      const frac = (t - sMin) / (sMax - sMin);
      const y = inner.y + (1 - frac) * inner.h;
      ctx.fillText(sLabels[ti]!, inner.x + inner.w + 4, y);
    }
  }

  ctx.strokeStyle = "#9ca3af";
  ctx.beginPath();
  ctx.moveTo(inner.x, Math.round(inner.y + inner.h) + 0.5);
  ctx.lineTo(inner.x + inner.w, Math.round(inner.y + inner.h) + 0.5);
  ctx.moveTo(Math.round(inner.x) + 0.5, inner.y);
  ctx.lineTo(Math.round(inner.x) + 0.5, inner.y + inner.h);
  if (secondaryTicks) {
    ctx.moveTo(Math.round(inner.x + inner.w) + 0.5, inner.y);
    ctx.lineTo(Math.round(inner.x + inner.w) + 0.5, inner.y + inner.h);
  }
  ctx.stroke();

  const groupGap = inner.w / categoryCount;
  ctx.fillStyle = AXIS_LABEL_COLOR;
  ctx.textAlign = "center";
  ctx.textBaseline = "top";
  const fmt = chart.categoriesFormat;
  const catLabel = (i: number): string => {
    const raw = (chart.categories ?? [])[i] ?? `${i + 1}`;
    if (!fmt) return raw;
    const n = parseFloat(raw);
    if (!Number.isFinite(n)) return raw;

    return raw;
  };
  for (let i = 0; i < categoryCount; i++) {
    ctx.fillText(catLabel(i), inner.x + (i + 0.5) * groupGap, inner.y + inner.h + 4);
  }
  drawCategoryAxisExtraRowsCentered(
    ctx,
    chart,
    inner,
    categoryCount,
    (i) => inner.x + (i + 0.5) * groupGap,
  );

  const xAt = (i: number) => inner.x + (i + 0.5) * groupGap;
  const yPrim = (v: number) => inner.y + (1 - (v - pMin) / (pMax - pMin)) * inner.h;
  const ySec = (v: number) =>
    secondaryTicks ? inner.y + (1 - (v - sMin) / (sMax - sMin)) * inner.h : yPrim(v);

  function drawBarsForSide(sideSeries: ChartSeries[], side: "primary" | "secondary"): void {
    const bars = sideSeries.filter((s) => {
      const k = seriesKind(s);
      return k === "column" || k === "bar";
    });
    if (bars.length === 0) return;
    const stacked = chart.grouping === "stacked" || chart.grouping === "percentstacked";
    const yFor = side === "secondary" ? ySec : yPrim;
    const minV = side === "secondary" ? sMin : pMin;
    const maxV = side === "secondary" ? sMax : pMax;
    const slot = computeBarSlotMetrics(
      groupGap,
      bars.length,
      stacked,
      chart.barGapWidth,
      chart.barOverlap,
    );
    const barW = slot.barW;

    type BarLabel = {
      s: ChartSeries;
      i: number;
      v: number;
      catTotal: number;
      bx: number;
      by: number;
      bw: number;
      bh: number;
      stacked: boolean;
    };
    const pending: BarLabel[] = [];

    const _plotTop = inner.y;
    const _plotBot = inner.y + inner.h;
    const _plotLeft = inner.x;
    const _plotRight = inner.x + inner.w;
    const clampFill = (bx: number, by: number, bw: number, bh: number) => {
      const x1 = Math.max(_plotLeft, bx);
      const x2 = Math.min(_plotRight, bx + bw);
      const y1 = Math.max(_plotTop, by);
      const y2 = Math.min(_plotBot, by + bh);
      return { x: x1, y: y1, w: Math.max(0, x2 - x1), h: Math.max(0, y2 - y1) };
    };

    if (stacked) {
      for (let i = 0; i < categoryCount; i++) {
        let pos = 0,
          neg = 0;

        let catTotal = 0;
        for (const s of bars) catTotal += Math.max(0, s.values[i] ?? 0);
        for (const s of bars) {
          const v = s.values[i] ?? 0;
          const start = v >= 0 ? pos : neg;
          const end = v >= 0 ? pos + v : neg + v;

          if (v >= 0) pos += v;
          else neg += v;
          const fill = resolveBarFill(s, i);
          const yA = yFor(start);
          const yB = yFor(end);
          const bx = xAt(i) - barW / 2;
          const by = Math.min(yA, yB);
          const bh = Math.abs(yB - yA);
          if (fill.skip) continue;
          const c = clampFill(bx, by, barW, bh);
          const adv = advancedPointFill(ctx, s, i, { x: c.x, y: c.y, w: c.w, h: c.h });
          ctx.fillStyle = adv ?? fill.color;
          if (c.w > 0 && c.h > 0) ctx.fillRect(c.x, c.y, c.w, c.h);
          if (effectiveLabels(chart, s)) {
            pending.push({ s, i, v, catTotal, bx, by, bw: barW, bh, stacked: true });
          }
        }
      }
    } else {
      for (let i = 0; i < categoryCount; i++) {
        const slotLeft = xAt(i) - groupGap / 2;
        for (let bi = 0; bi < bars.length; bi++) {
          const s = bars[bi]!;
          const v = s.values[i] ?? 0;
          const fill = resolveBarFill(s, i);
          const y1 = yFor(v);
          const y0 = yFor(Math.max(minV, Math.min(maxV, 0)));
          const bx = slotLeft + slot.firstBarLeftOffset + bi * slot.barShift;
          const by = Math.min(y0, y1);
          const bh = Math.abs(y1 - y0);
          if (fill.skip) continue;
          const c = clampFill(bx, by, barW, bh);
          const adv = advancedPointFill(ctx, s, i, { x: c.x, y: c.y, w: c.w, h: c.h });
          ctx.fillStyle = adv ?? fill.color;
          if (c.w > 0 && c.h > 0) ctx.fillRect(c.x, c.y, c.w, c.h);
          if (effectiveLabels(chart, s)) {
            pending.push({ s, i, v, catTotal: 0, bx, by, bw: barW, bh, stacked: false });
          }
        }
      }
    }

    for (const p of pending) {
      const baseDl = effectiveLabels(chart, p.s)!;
      const po = pointLabel(baseDl, p.i);
      if (po === null) continue;
      const dl = po?.dl ?? baseDl;
      const text = po?.text ?? buildLabelText(dl, chart, p.s, p.i, p.v, p.catTotal);
      if (!text) continue;
      if (p.stacked) {
        drawLabel(ctx, text, p.bx + p.bw / 2, p.by + p.bh / 2);
      } else {
        const pos = dl.position ?? "outEnd";
        const PAD = 3;
        let ly = p.by + p.bh / 2;
        if (pos === "outEnd") ly = p.v >= 0 ? p.by - PAD : p.by + p.bh + PAD;
        else if (pos === "inEnd") ly = p.v >= 0 ? p.by + PAD : p.by + p.bh - PAD;
        else if (pos === "inBase") ly = p.v >= 0 ? p.by + p.bh - PAD : p.by + PAD;
        const baseline: CanvasTextBaseline =
          pos === "outEnd"
            ? p.v >= 0
              ? "bottom"
              : "top"
            : pos === "inEnd"
              ? p.v >= 0
                ? "top"
                : "bottom"
              : pos === "inBase"
                ? p.v >= 0
                  ? "bottom"
                  : "top"
                : "middle";
        drawLabel(ctx, text, p.bx + p.bw / 2, ly, "center", baseline);
      }
    }
  }

  drawBarsForSide(primarySeries, "primary");
  drawBarsForSide(secondarySeries, "secondary");

  function drawLinesAreasForSide(sideSeries: ChartSeries[], side: "primary" | "secondary"): void {
    const yFor = side === "secondary" ? ySec : yPrim;

    type LineLabel = { s: ChartSeries; kind: "line" | "area"; i: number; v: number };
    const pending: LineLabel[] = [];

    const hasPoint = (s: ChartSeries, i: number): boolean => {
      if (i >= s.values.length) return false;
      const v = s.values[i];
      return v != null && Number.isFinite(v);
    };
    for (const s of sideSeries) {
      const k = seriesKind(s);
      if (k === "line") {
        ctx.strokeStyle = s.color ?? "#4472C4";
        ctx.lineWidth = seriesLineWidth(s, 2);
        ctx.setLineDash(seriesLineDash(s));
        ctx.beginPath();
        let penDown = false;
        for (let i = 0; i < categoryCount; i++) {
          if (!hasPoint(s, i)) {
            penDown = false;
            continue;
          }
          const x = xAt(i);
          const y = yFor(s.values[i]!);
          if (!penDown) {
            ctx.moveTo(x, y);
            penDown = true;
          } else {
            ctx.lineTo(x, y);
          }
        }
        ctx.stroke();
        ctx.setLineDash([]);

        if (s.markerSymbol !== "none") {
          ctx.fillStyle = s.color ?? "#4472C4";
          for (let i = 0; i < categoryCount; i++) {
            if (!hasPoint(s, i)) continue;
            ctx.beginPath();
            ctx.arc(xAt(i), yFor(s.values[i]!), 3, 0, Math.PI * 2);
            ctx.fill();
          }
        }
        if (effectiveLabels(chart, s)) {
          for (let i = 0; i < categoryCount; i++) {
            if (!hasPoint(s, i)) continue;
            pending.push({ s, kind: "line", i, v: s.values[i]! });
          }
        }
      } else if (k === "area") {
        const baseline = yFor(side === "secondary" ? sMin : pMin);
        ctx.fillStyle = withAlpha(s.color ?? "#4472C4", 0.55);
        ctx.beginPath();
        for (let i = 0; i < categoryCount; i++) {
          const x = xAt(i);
          const y = yFor(s.values[i] ?? 0);
          if (i === 0) ctx.moveTo(x, y);
          else ctx.lineTo(x, y);
        }
        for (let i = categoryCount - 1; i >= 0; i--) ctx.lineTo(xAt(i), baseline);
        ctx.closePath();
        ctx.fill();
        if (effectiveLabels(chart, s)) {
          for (let i = 0; i < categoryCount; i++) {
            pending.push({ s, kind: "area", i, v: s.values[i] ?? 0 });
          }
        }
      }
    }
    ctx.lineWidth = 1;

    const sideFmt =
      side === "secondary" ? (chart.valueFormatSecondary ?? chart.valueFormat) : chart.valueFormat;
    for (const p of pending) {
      const dlRaw = effectiveLabels(chart, p.s)!;
      const po = pointLabel(dlRaw, p.i);
      if (po === null) continue;
      const dlMerged = po?.dl ?? dlRaw;
      const dl = dlMerged.numFmt ? dlMerged : { ...dlMerged, numFmt: sideFmt };
      const text = po?.text ?? buildLabelText(dl, chart, p.s, p.i, p.v, 0);
      if (!text) continue;
      const x = xAt(p.i);
      const y = yFor(p.v);
      if (p.kind === "area") {
        drawLabel(ctx, text, x, y - 4, "center", "bottom");
        continue;
      }
      const pos = dl.position ?? "t";
      const PAD = 5;
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
  drawLinesAreasForSide(primarySeries, "primary");
  drawLinesAreasForSide(secondarySeries, "secondary");

  paintZeroBaseline(ctx, inner, pMin, pMax);
}
