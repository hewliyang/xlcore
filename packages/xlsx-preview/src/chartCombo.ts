import type { Chart, ChartSeries } from "./types.js";
import type { Rect } from "./chart.js";
import { resolveBarFill } from "./chartAdvanced.js";
import {
  buildLabelText,
  computeBarSlotMetrics,
  drawLabel,
  drawPlaceholderPlot,
  effectiveLabels,
  formatAxisValue,
  isZeroTickInside,
  paintZeroBaseline,
  pointLabel,
  resolveAxisRange,
  withAlpha,
} from "./chartUtils.js";

const AXIS_FONT_SIZE = 10;
const AXIS_LABEL_COLOR = "#52525b";
const AXIS_TICK_COUNT = 5;
const GRIDLINE_COLOR = "#e5e7eb";

// ---------- combo (dual y-axis, mixed chart types) ----------
//
// Handles two scenarios:
//   1. Combo charts: `<c:barChart>` + `<c:lineChart>` (etc.) in one
//      plotArea. Each series carries `chartType` from the extractor.
//   2. Single-type charts with a secondary axis (e.g. two lines on
//      different scales). `chart.secondaryAxis` is true; series carry
//      `axisGroup = primary|secondary` but no per-series `chartType`.
//
// Layout: left gutter for primary y-axis labels, right gutter for
// secondary y-axis labels, shared category x-axis along the bottom.
// All bars/columns are drawn at category centers; line/area points
// also land on category centers so they overlay the bars correctly
// (this matches Excel's combo layout).
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

  // Per-series effective chart type: per-series override > chart-level
  // type > "column". For non-combo dual-axis (e.g. Chart_25 two-lines),
  // chart.type is already "line"/"column" and we inherit it.
  const seriesKind = (s: ChartSeries): string =>
    s.chartType ?? (chart.type === "combo" ? "column" : chart.type);

  // Compute y range for a side. Stacked grouping only applies to
  // bar/column series within that side; line/area series always use
  // their own raw values for the range.
  function rangeFor(side: ChartSeries[]): { minV: number; maxV: number } {
    // Seed with +/-Infinity so the data min/max actually wins for
    // entirely-positive (or entirely-negative) data. Starting at 0
    // would silently zero-clamp here — hiding the bug behind
    // resolveAxisRange's user-scaling logic and forcing combo charts
    // with a manual `<c:max>` (e.g. data 1098–1220, max=1220) to
    // render as a 0–1220 axis with all bars looking identical.
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
    // No data at all on this side (shouldn't happen after the
    // categoryCount > 0 guard, but be defensive).
    if (!Number.isFinite(minV)) minV = 0;
    if (!Number.isFinite(maxV)) maxV = 1;
    // Note: we deliberately do NOT zero-clamp here. The caller passes
    // both the raw data range and any explicit `<c:scaling>` bounds
    // to `resolveAxisRange`, which handles zero-clamping vs
    // user-scaled mode in one place. Clamping here would erase the
    // data minimum (e.g. 1098) and force the axis to start at 0 even
    // when the workbook pinned a manual max.
    if (minV === maxV) maxV = minV + 1;
    return { minV, maxV };
  }

  const primaryRange = rangeFor(primarySeries);
  const secondaryRange = secondarySeries.length > 0 ? rangeFor(secondarySeries) : null;

  // Honor explicit `<c:scaling>` min/max from the workbook. Combo
  // charts almost always set at least one bound on the secondary
  // axis (e.g. Charts(1) Chart 19 sets `<c:max val="1220"/>` on
  // primary and `<c:max val="14"/>` on secondary). Without this the
  // chart's data variation gets compressed to indistinguishable
  // bars/lines.
  // Zero-clamp must be computed per-axis: a value axis draws from
  // zero iff some series *bound to that axis* is a bar/column/area
  // type. Previously a single `hasBars` over the union leaked the
  // primary side's bars onto the secondary axis, compressing
  // line-only secondary data sitting far above zero into the top
  // sliver of the plot. See parity-charts.md Bug #11.
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
    /*zeroClamp=*/ primaryHasBars,
    AXIS_TICK_COUNT,
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
        /*zeroClamp=*/ secondaryHasBars,
        AXIS_TICK_COUNT,
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
  const xAxisH = AXIS_FONT_SIZE + 8;

  const inner: Rect = {
    x: rect.x + leftGutter,
    y: rect.y,
    w: rect.w - leftGutter - rightGutter,
    h: rect.h - xAxisH,
  };

  // Gridlines + primary axis labels (left). Combo paths only ever
  // paint primary gridlines (the secondary side's would clash on
  // dual-axis charts), so we gate on the primary toggle alone.
  // Per parity-charts.md Bug #12.
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
    // Bug #13 step 1: skip the lighter gridline at t==0 on a
    // straddles-zero primary axis; the heavier baseline will overlay
    // after fills.
    const isZeroLine = isZeroTickInside(t, pMin, pMax);
    if (showPrimaryGridlines && !isZeroLine) {
      ctx.beginPath();
      ctx.moveTo(inner.x, Math.round(y) + 0.5);
      ctx.lineTo(inner.x + inner.w, Math.round(y) + 0.5);
      ctx.stroke();
    }
    ctx.fillText(pLabels[ti]!, inner.x - 4, y);
  }
  // Secondary axis labels (right) — no extra gridlines (would clash
  // with the primary ones); just tick marks + text.
  if (secondaryTicks) {
    ctx.textAlign = "left";
    for (let ti = 0; ti < secondaryTicks.length; ti++) {
      const t = secondaryTicks[ti]!;
      const frac = (t - sMin) / (sMax - sMin);
      const y = inner.y + (1 - frac) * inner.h;
      ctx.fillText(sLabels[ti]!, inner.x + inner.w + 4, y);
    }
  }
  // Axis baselines.
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

  // Category x-axis labels — centered (matches bar layout).
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
    // Fallback to raw string formatting; we already format axis ticks
    // via formatAxisValue, but categoriesFormat is for cat labels.
    return raw;
  };
  for (let i = 0; i < categoryCount; i++) {
    ctx.fillText(catLabel(i), inner.x + (i + 0.5) * groupGap, inner.y + inner.h + 4);
  }

  // Helpers: x position for category i (center of its group), y for a value
  // on the primary or secondary scale.
  const xAt = (i: number) => inner.x + (i + 0.5) * groupGap;
  const yPrim = (v: number) => inner.y + (1 - (v - pMin) / (pMax - pMin)) * inner.h;
  const ySec = (v: number) =>
    secondaryTicks ? inner.y + (1 - (v - sMin) / (sMax - sMin)) * inner.h : yPrim(v);

  // Group bar/column series by side so we can compute clustered or
  // stacked bar layout per-side. Within a side, the bars share the
  // category-group slot (i + 0.5) * groupGap.
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

    // Collect (series, categoryIdx, value, bbox) deferred-label tuples
    // so labels paint after every bar fill on this side — labels are
    // halo'd text and should sit on top of any sibling-series bar that
    // would otherwise occlude them. Per parity-charts.md Bug #8: the
    // combo path previously skipped dLbls entirely.
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

    // Excel clips bar fills to the plot area; honor the same here so
    // out-of-range stacked totals (or single values larger than a
    // user-pinned `<c:max>`) don't paint past the top gridline.
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
        // Per-category total for `showPercent` (positive contributions
        // only, matching the single-axis stacked-bar path).
        let catTotal = 0;
        for (const s of bars) catTotal += Math.max(0, s.values[i] ?? 0);
        for (const s of bars) {
          const v = s.values[i] ?? 0;
          const start = v >= 0 ? pos : neg;
          const end = v >= 0 ? pos + v : neg + v;
          // Always advance the stack accumulator so transparent dPts
          // still occupy their slot — see resolveBarFill.
          if (v >= 0) pos += v;
          else neg += v;
          const fill = resolveBarFill(s, i);
          const yA = yFor(start);
          const yB = yFor(end);
          const bx = xAt(i) - barW / 2;
          const by = Math.min(yA, yB);
          const bh = Math.abs(yB - yA);
          if (fill.skip) continue;
          ctx.fillStyle = fill.color;
          const c = clampFill(bx, by, barW, bh);
          if (c.w > 0 && c.h > 0) ctx.fillRect(c.x, c.y, c.w, c.h);
          if (effectiveLabels(chart, s)) {
            pending.push({ s, i, v, catTotal, bx, by, bw: barW, bh, stacked: true });
          }
        }
      }
    } else {
      for (let i = 0; i < categoryCount; i++) {
        // xAt(i) returns the slot center; offset back to slot's left edge.
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
          ctx.fillStyle = fill.color;
          const c = clampFill(bx, by, barW, bh);
          if (c.w > 0 && c.h > 0) ctx.fillRect(c.x, c.y, c.w, c.h);
          if (effectiveLabels(chart, s)) {
            pending.push({ s, i, v, catTotal: 0, bx, by, bw: barW, bh, stacked: false });
          }
        }
      }
    }

    // Paint deferred labels on top of all bars on this side. Mirrors
    // the single-axis vertical bar/column label logic in
    // `drawBarColumnChart` (combo is always vertical — `chart.type ===
    // "bar"` would route to `drawBarColumnChart` instead).
    for (const p of pending) {
      const baseDl = effectiveLabels(chart, p.s)!;
      const po = pointLabel(baseDl, p.i);
      if (po === null) continue; // per-point suppression
      const dl = po?.dl ?? baseDl;
      const text = po?.text ?? buildLabelText(dl, chart, p.s, p.i, p.v, p.catTotal);
      if (!text) continue;
      if (p.stacked) {
        // Default position for stacked: `ctr` (in-bar center).
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

  // Lines / areas: drawn on top of bars so series overlap is readable.
  function drawLinesAreasForSide(sideSeries: ChartSeries[], side: "primary" | "secondary"): void {
    const yFor = side === "secondary" ? ySec : yPrim;
    // Defer label painting so labels sit above every line/area on this
    // side. Mirrors the per-series approach in `drawLineChart` /
    // `drawAreaChart` but stripped to the combo subset (categories are
    // equispaced, no stacked grouping inside the combo path — Excel
    // routes pure stacked-line/area charts to the single-axis painters).
    type LineLabel = { s: ChartSeries; kind: "line" | "area"; i: number; v: number };
    const pending: LineLabel[] = [];
    // Excel default `<c:dispBlanksAs val="gap"/>` (ECMA-376 §21.2.2.34):
    // missing values break the line / leave a gap in area fills rather
    // than collapsing to zero. Without this guard a line series shorter
    // than `categoryCount` plots a phantom segment crashing to y(0) —
    // visible on `Charts_Chart_17` where `No. of projects` has a single
    // value [18.0] on a secondary axis ranging [18, 19], dropping the
    // line off the bottom of the plot.
    const hasPoint = (s: ChartSeries, i: number): boolean => {
      if (i >= s.values.length) return false;
      const v = s.values[i];
      return v != null && Number.isFinite(v);
    };
    for (const s of sideSeries) {
      const k = seriesKind(s);
      if (k === "line") {
        ctx.strokeStyle = s.color ?? "#4472C4";
        ctx.lineWidth = 2;
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
        // Markers — only at real data points. Suppressed by
        // explicit `<c:marker><c:symbol val="none"/>` on the series.
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

    // Paint labels on top. Line series honor `<c:dLbls><c:dLblPos>`
    // (default `t` = top); area series default to a small offset above
    // the top edge.
    //
    // Number format precedence: per-series dLbls.numFmt > the value
    // format of *the axis this series is bound to* > chart-level
    // primary format. Without the side-aware step, a line series on
    // the secondary axis (e.g. Chart 19's `Guards per contract`, on
    // the right axis with `<c:numFmt formatCode="0.0"/>`) inherits
    // the primary axis's `0` format and prints integers (`12, 11, 10`)
    // instead of the spec-correct `12.1, 10.7, 10.2`.
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
  // Bug #13 step 1: heavier zero baseline on the primary axis when
  // it straddles zero. (We don't paint a separate baseline for the
  // secondary axis since its scale isn't visualized as a gridline
  // family — the secondary side only gets tick labels, not gridlines.)
  paintZeroBaseline(ctx, inner, pMin, pMax);
}
