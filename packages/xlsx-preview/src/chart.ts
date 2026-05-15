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
// Number formatting reuses the same subset as the cell renderer, so axis labels match cell formats.
import type { Chart, ChartSeries } from "./types.js";
import {
  buildLabelText,
  buildStackedRows,
  computeBarSlotMetrics,
  drawAxisFrame,
  drawCategoryAxis,
  drawLabel,
  drawLegend,
  drawPlaceholderPlot,
  measureVerticalLegendWidth,
  effectiveLabels,
  pointLabel,
  formatAxisValue,
  isZeroTickInside,
  paintZeroBaseline,
  zeroAxisMetrics,
  resolveAxisRange,
  valueRange,
  withAlpha,
} from "./chartUtils.js";

import {
  drawBubbleChart,
  drawComboChart,
  drawPieChart,
  drawScatterChart,
  pieSliceColor,
  resolveBarFill,
} from "./chartAdvanced.js";

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

  // Build the legend entry list up front so we can measure the side
  // strip width when positioning at `l` / `r` / `tr`. Pie/doughnut
  // legends are slice-keyed (one entry per category); everything else
  // is series-keyed.
  // `categories` / `pointColors` get `skip_serializing_if = Vec::is_empty`
  // on the wire, so the renderer must treat them as optional even though
  // the TS type calls them required arrays.
  const cats = chart.categories ?? [];
  const legendEntries: ChartSeries[] =
    (chart.type === "pie" || chart.type === "doughnut") && chart.series.length > 0
      ? (() => {
          const s = chart.series[0]!;
          const pointColors = s.pointColors ?? [];
          const sliceCount = Math.max(s.values.length, cats.length);
          return Array.from({ length: sliceCount }, (_, i) => ({
            ...s,
            name: cats[i] ?? `${i + 1}`,
            color: pieSliceColor(i, pointColors),
          }));
        })()
      : chart.series;

  // ECMA-376 legend positions: t/b/l/r/tr. The extractor surfaces
  // `legendPos = undefined` when the source XML has no `<c:legend>`
  // element (Excel: "no legend") and a concrete position string
  // when the element is present (defaulting to `"r"` per Excel
  // when `<c:legendPos>` itself is absent). We treat absent as
  // "don't paint" to match Excel desktop / hsx — see
  // parity-charts.md Bug #17. `tr` ("top-right overlay") is
  // coerced to `r` below since we don't overlay legends today.
  const legendPos = chart.series.length > 0 && chart.legendPos ? chart.legendPos : null;
  const legendVertical = legendPos === "l" || legendPos === "r" || legendPos === "tr";
  let legendW = 0;
  let legendH = 0;
  if (legendPos !== null) {
    if (legendVertical) {
      legendW = measureVerticalLegendWidth(ctx, legendEntries);
    } else {
      legendH = LEGEND_FONT_SIZE + 14;
    }
  }

  let legendRect: Rect = { x: 0, y: 0, w: 0, h: 0 };
  const plotRect: Rect = {
    x: rect.x + PLOT_PAD_LEFT,
    y: cursorY,
    w: rect.w - PLOT_PAD_LEFT - PLOT_PAD_RIGHT,
    h: rect.y + rect.h - cursorY - 4,
  };
  switch (legendPos) {
    case "t":
      legendRect = { x: rect.x, y: cursorY, w: rect.w, h: legendH };
      plotRect.y = cursorY + legendH;
      plotRect.h = rect.y + rect.h - plotRect.y - 4;
      break;
    case "l":
      legendRect = { x: rect.x + 4, y: cursorY, w: legendW, h: plotRect.h };
      plotRect.x = rect.x + 4 + legendW + 8;
      plotRect.w = rect.x + rect.w - plotRect.x - PLOT_PAD_RIGHT;
      break;
    case "r":
    case "tr":
      legendRect = {
        x: rect.x + rect.w - legendW - 8,
        y: cursorY,
        w: legendW,
        h: plotRect.h,
      };
      plotRect.w = legendRect.x - plotRect.x - 8;
      break;
    case "b":
    default:
      legendRect = {
        x: rect.x,
        y: rect.y + rect.h - legendH,
        w: rect.w,
        h: legendH,
      };
      plotRect.h = rect.y + rect.h - cursorY - legendH - 4;
      break;
  }
  // Axis-title bands. ECMA-376 §21.2.2.213 — every axis carries an
  // optional `<c:title>`. We reserve a fixed strip (font size + 6px
  // padding) on each occupied edge, then paint inside it. The strips
  // sit *inside* the chart frame but *outside* the plot area so the
  // axis tick labels still have room. We don't reserve when the
  // corresponding side hosts no title — keeps unaffected charts
  // pixel-stable.
  const AXIS_TITLE_FONT_SIZE = 11;
  const AXIS_TITLE_PAD = 6;
  const AXIS_TITLE_BAND = AXIS_TITLE_FONT_SIZE + AXIS_TITLE_PAD;
  const xTitle = chart.xAxisTitle;
  const yTitle = chart.yAxisTitle;
  const yTitle2 = chart.yAxisTitleSecondary;
  let xTitleRect: Rect | null = null;
  let yTitleRect: Rect | null = null;
  let yTitle2Rect: Rect | null = null;
  if (xTitle) {
    xTitleRect = {
      x: plotRect.x,
      y: plotRect.y + plotRect.h - AXIS_TITLE_BAND,
      w: plotRect.w,
      h: AXIS_TITLE_BAND,
    };
    plotRect.h -= AXIS_TITLE_BAND;
  }
  if (yTitle) {
    yTitleRect = {
      x: plotRect.x,
      y: plotRect.y,
      w: AXIS_TITLE_BAND,
      h: plotRect.h,
    };
    plotRect.x += AXIS_TITLE_BAND;
    plotRect.w -= AXIS_TITLE_BAND;
  }
  if (yTitle2 && (chart.secondaryAxis || chart.type === "combo")) {
    yTitle2Rect = {
      x: plotRect.x + plotRect.w - AXIS_TITLE_BAND,
      y: plotRect.y,
      w: AXIS_TITLE_BAND,
      h: plotRect.h,
    };
    plotRect.w -= AXIS_TITLE_BAND;
  }

  // `<c:dispUnitsLbl>` caption band. ECMA-376 §21.2.2.46: when an axis
  // is authored with `<c:dispUnits>` (e.g. `builtInUnit=thousands`)
  // and a sibling `<c:dispUnitsLbl>` (e.g. `"S$ mn"`), the caption
  // paints near the axis to call out the scale factor applied to the
  // tick labels. Excel's default rotation is along the axis (-5400000
  // EMU = -90°) but the placement is also commonly horizontal at the
  // top of the axis depending on theme. We paint horizontal,
  // left-aligned to the y-axis on the primary side and right-aligned
  // on the secondary side, in a narrow reserved band right above the
  // plot area (i.e. between the chart title and the topmost tick).
  const DISP_UNITS_FONT_SIZE = 10;
  const DISP_UNITS_BAND = DISP_UNITS_FONT_SIZE + 4;
  const duLabel = chart.dispUnits != null ? chart.dispUnitsLabel : undefined;
  const duLabel2 =
    chart.dispUnitsSecondary != null && (chart.secondaryAxis || chart.type === "combo")
      ? chart.dispUnitsLabelSecondary
      : undefined;
  let duBandRect: Rect | null = null;
  if (duLabel || duLabel2) {
    duBandRect = {
      x: plotRect.x,
      y: plotRect.y,
      w: plotRect.w,
      h: DISP_UNITS_BAND,
    };
    plotRect.y += DISP_UNITS_BAND;
    plotRect.h -= DISP_UNITS_BAND;
  }

  if (plotRect.w <= 20 || plotRect.h <= 20) return;

  // Combo charts (`<c:barChart>` + `<c:lineChart>` in one plotArea) or
  // any chart with a secondary axis (right-hand y-axis) route through
  // the dual-scale path so both series groups land on the same plot
  // with their own y-scale. Per-series `chartType` lets us mix
  // column/bar/line/area within a single chart.
  if (chart.type === "combo" || chart.secondaryAxis) {
    drawComboChart(ctx, chart, plotRect);
  } else {
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
      case "bubble":
        drawBubbleChart(ctx, chart, plotRect);
        break;
      default:
        drawPlaceholderPlot(ctx, chart, plotRect);
    }
  }

  if (legendPos !== null) {
    drawLegend(ctx, legendEntries, legendRect, legendVertical ? "vertical" : "horizontal", chart);
  }

  // Paint axis titles on top — done last so they sit above any
  // overflow from the plot painters. Y-axis titles rotate -90°
  // (Excel convention: text reads bottom-to-top on the left edge,
  // top-to-bottom on the right edge).
  if (xTitleRect && xTitle) {
    ctx.fillStyle = TITLE_COLOR;
    ctx.font = `${AXIS_TITLE_FONT_SIZE}px -apple-system, "Helvetica Neue", Arial, sans-serif`;
    ctx.textAlign = "center";
    ctx.textBaseline = "middle";
    ctx.fillText(xTitle, xTitleRect.x + xTitleRect.w / 2, xTitleRect.y + xTitleRect.h / 2);
  }
  if (yTitleRect && yTitle) {
    ctx.save();
    ctx.fillStyle = TITLE_COLOR;
    ctx.font = `${AXIS_TITLE_FONT_SIZE}px -apple-system, "Helvetica Neue", Arial, sans-serif`;
    ctx.textAlign = "center";
    ctx.textBaseline = "middle";
    ctx.translate(yTitleRect.x + yTitleRect.w / 2, yTitleRect.y + yTitleRect.h / 2);
    ctx.rotate(-Math.PI / 2);
    ctx.fillText(yTitle, 0, 0);
    ctx.restore();
  }
  if (yTitle2Rect && yTitle2) {
    ctx.save();
    ctx.fillStyle = TITLE_COLOR;
    ctx.font = `${AXIS_TITLE_FONT_SIZE}px -apple-system, "Helvetica Neue", Arial, sans-serif`;
    ctx.textAlign = "center";
    ctx.textBaseline = "middle";
    // Right-side rotation: text reads top-to-bottom, matching Excel.
    ctx.translate(yTitle2Rect.x + yTitle2Rect.w / 2, yTitle2Rect.y + yTitle2Rect.h / 2);
    ctx.rotate(Math.PI / 2);
    ctx.fillText(yTitle2, 0, 0);
    ctx.restore();
  }
  // `<c:dispUnitsLbl>` caption(s). Painted last so they sit above any
  // gridline / fill bleed. Left caption hugs the left edge of the
  // plot (right above the y-axis), right caption hugs the right edge.
  if (duBandRect && (duLabel || duLabel2)) {
    ctx.save();
    ctx.fillStyle = AXIS_LABEL_COLOR;
    ctx.font = `${DISP_UNITS_FONT_SIZE}px -apple-system, "Helvetica Neue", Arial, sans-serif`;
    ctx.textBaseline = "middle";
    if (duLabel) {
      ctx.textAlign = "left";
      ctx.fillText(duLabel, duBandRect.x + 2, duBandRect.y + duBandRect.h / 2);
    }
    if (duLabel2) {
      ctx.textAlign = "right";
      ctx.fillText(duLabel2, duBandRect.x + duBandRect.w - 2, duBandRect.y + duBandRect.h / 2);
    }
    ctx.restore();
  }
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
  const categoryCount = Math.max(
    ...series.map((s) => s.values.length),
    (chart.categories ?? []).length,
  );
  if (categoryCount === 0) return;

  // Compute value range. Seed with +/-Infinity so the data extremes
  // win for entirely-positive data; the subsequent `resolveAxisRange`
  // call applies the zero-clamp when no `<c:scaling>` override is
  // present.
  let minV = Number.POSITIVE_INFINITY,
    maxV = Number.NEGATIVE_INFINITY;
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
  if (!Number.isFinite(minV)) minV = 0;
  if (!Number.isFinite(maxV)) maxV = 1;
  // Resolve the axis range, honoring any explicit `<c:scaling><c:min>`
  // / `<c:max>` from the workbook. Bars/columns zero-clamp by default;
  // an explicit min or max flips the axis into user-scaled mode
  // (matches Excel).
  const _bcRange = resolveAxisRange(
    minV,
    maxV,
    chart.valueMin,
    chart.valueMax,
    /*zeroClamp=*/ true,
    AXIS_TICK_COUNT,
  );
  minV = _bcRange.minV;
  maxV = _bcRange.maxV;
  const ticks = _bcRange.ticks;

  // Measure the value-axis label width so we can carve out a y-axis gutter.
  ctx.font = `${AXIS_FONT_SIZE}px -apple-system, "Helvetica Neue", Arial, sans-serif`;
  const labelStrings = ticks.map((t) => formatAxisValue(t, chart.valueFormat, chart.dispUnits));
  const yAxisW = Math.max(...labelStrings.map((s) => ctx.measureText(s).width)) + 8;
  const xAxisH = AXIS_FONT_SIZE + 8;

  const innerRect: Rect = horizontal
    ? { x: rect.x + yAxisW, y: rect.y, w: rect.w - yAxisW, h: rect.h - xAxisH }
    : { x: rect.x + yAxisW, y: rect.y, w: rect.w - yAxisW, h: rect.h - xAxisH };

  // Gridlines + value-axis labels. Gridline pass honors
  // `chart.showMajorGridlines` per parity-charts.md Bug #12; tick
  // labels always paint (Excel keeps labels even when gridlines
  // are hidden via "Line Color: No Line").
  const showGridlines = chart.showMajorGridlines !== false;
  ctx.fillStyle = AXIS_LABEL_COLOR;
  ctx.strokeStyle = GRIDLINE_COLOR;
  ctx.lineWidth = 1;
  ctx.textAlign = horizontal ? "center" : "right";
  ctx.textBaseline = "middle";
  for (let ti = 0; ti < ticks.length; ti++) {
    const t = ticks[ti]!;
    const frac = (t - minV) / (maxV - minV);
    // Bug #13 step 1: skip the lighter gridline at t==0 when the axis
    // straddles zero — we'll overlay the heavier baseline after fills.
    const isZeroLine = isZeroTickInside(t, minV, maxV);
    if (horizontal) {
      const x = innerRect.x + frac * innerRect.w;
      if (showGridlines && !isZeroLine) {
        ctx.beginPath();
        ctx.moveTo(Math.round(x) + 0.5, innerRect.y);
        ctx.lineTo(Math.round(x) + 0.5, innerRect.y + innerRect.h);
        ctx.stroke();
      }
      ctx.fillText(labelStrings[ti]!, x, innerRect.y + innerRect.h + xAxisH / 2);
    } else {
      const y = innerRect.y + (1 - frac) * innerRect.h;
      if (showGridlines && !isZeroLine) {
        ctx.beginPath();
        ctx.moveTo(innerRect.x, Math.round(y) + 0.5);
        ctx.lineTo(innerRect.x + innerRect.w, Math.round(y) + 0.5);
        ctx.stroke();
      }
      ctx.fillText(labelStrings[ti]!, innerRect.x - 4, y);
    }
  }

  // Bars. Slot geometry per ECMA-376 §21.2.2.75 / .108 — see
  // computeBarSlotMetrics. `gapWidth` defaults to 150 (Excel spec),
  // `overlap` defaults to 100 for stacked / 0 for clustered.
  const groupGap = horizontal ? innerRect.h / categoryCount : innerRect.w / categoryCount;
  const slot = computeBarSlotMetrics(
    groupGap,
    series.length,
    stacked,
    chart.barGapWidth,
    chart.barOverlap,
  );
  const barSize = slot.barW;

  // Precompute zero baseline (parity-charts.md Bug #13 step 3:
  // shared `zeroAxisMetrics` so the bar geometry, the gridline-skip
  // pass, and the post-fill heavier baseline all consult one source
  // of truth).
  const zMetrics = zeroAxisMetrics(innerRect, minV, maxV);
  const zeroY = zMetrics.zeroY;
  const zeroX = zMetrics.zeroX;

  // Category labels along axis.
  ctx.fillStyle = AXIS_LABEL_COLOR;
  ctx.textAlign = "center";
  ctx.textBaseline = horizontal ? "middle" : "top";
  for (let i = 0; i < categoryCount; i++) {
    const center = horizontal
      ? innerRect.y + (i + 0.5) * groupGap
      : innerRect.x + (i + 0.5) * groupGap;
    const label = (chart.categories ?? [])[i] ?? `${i + 1}`;
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
        // Always advance the stack accumulator — transparent dPts
        // (resolveBarFill skip=true) still occupy their slot so
        // subsequent series float above the prior contributions.
        if (v >= 0) pos += v;
        else neg += v;
        const fill = resolveBarFill(s, i);
        const sFrac = (start - minV) / (maxV - minV);
        const eFrac = (end - minV) / (maxV - minV);
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
        if (fill.skip) continue; // transparent dPt — no fill, no label
        ctx.fillStyle = fill.color;
        ctx.fillRect(bx, by, bw, bh);
        // Stacked label: position default `ctr` (in-bar center).
        const dl = effectiveLabels(chart, s);
        if (dl) {
          const po = pointLabel(dl, i);
          if (po !== null) {
            const edl = po?.dl ?? dl;
            const text = po?.text ?? buildLabelText(edl, chart, s, i, v, catTotal);
            drawLabel(ctx, text, bx + bw / 2, by + bh / 2);
          }
        }
      }
    }
  } else {
    for (let i = 0; i < categoryCount; i++) {
      for (let si = 0; si < series.length; si++) {
        const s = series[si]!;
        const v = s.values[i] ?? 0;
        const frac = (v - minV) / (maxV - minV);
        const fill = resolveBarFill(s, i);
        let bx = 0,
          by = 0,
          bw = 0,
          bh = 0;
        if (horizontal) {
          const groupTop = innerRect.y + i * groupGap + slot.firstBarLeftOffset;
          const top = groupTop + si * slot.barShift;
          const x1 = zeroX;
          const x2 = innerRect.x + frac * innerRect.w;
          bx = Math.min(x1, x2);
          by = top;
          bw = Math.abs(x2 - x1);
          bh = barSize;
        } else {
          const groupLeft = innerRect.x + i * groupGap + slot.firstBarLeftOffset;
          const left = groupLeft + si * slot.barShift;
          const yTop = innerRect.y + (1 - frac) * innerRect.h;
          const yBot = zeroY;
          bx = left;
          by = Math.min(yTop, yBot);
          bw = barSize;
          bh = Math.abs(yBot - yTop);
        }
        if (fill.skip) continue; // transparent dPt — no fill, no label
        ctx.fillStyle = fill.color;
        ctx.fillRect(bx, by, bw, bh);
        const dl = effectiveLabels(chart, s);
        if (dl) {
          const po = pointLabel(dl, i);
          if (po === null) continue; // suppressed via per-point delete
          const edl = po?.dl ?? dl;
          const text = po?.text ?? buildLabelText(edl, chart, s, i, v, /*catTotal=*/ 0);
          // Default position: outEnd. `inEnd`/`ctr`/`inBase` honored.
          const pos = edl.position ?? "outEnd";
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

  // Axis baselines. The horizontal stroke sits at the zero baseline
  // (== bottom of inner rect when the axis is entirely non-negative;
  // somewhere inside when the axis straddles zero). For the
  // straddles-zero case we upgrade it to the heavier `paintZeroBaseline`
  // stroke per parity-charts.md Bug #13 step 1, drawn *after* bar fills
  // so it reads as a conceptual divider. When the axis doesn't straddle
  // zero we keep the original light `#9ca3af` frame stroke since it's
  // serving as the x-axis frame, not as a zero marker.
  if (zMetrics.straddlesZero) {
    paintZeroBaseline(ctx, innerRect, minV, maxV);
  } else {
    ctx.strokeStyle = "#9ca3af";
    ctx.beginPath();
    ctx.moveTo(innerRect.x, Math.round(zeroY) + 0.5);
    ctx.lineTo(innerRect.x + innerRect.w, Math.round(zeroY) + 0.5);
    ctx.stroke();
  }
  // Left y-axis frame edge (unchanged).
  ctx.strokeStyle = "#9ca3af";
  ctx.lineWidth = 1;
  ctx.beginPath();
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
  const categoryCount = Math.max(
    ...series.map((s) => s.values.length),
    (chart.categories ?? []).length,
  );
  if (categoryCount === 0) return;

  const stacked = chart.grouping === "stacked" || chart.grouping === "percentstacked";
  const percent = chart.grouping === "percentstacked";

  // Cumulative per-category stacks (only used for stacked / percentStacked).
  const stackedSeries: number[][] = stacked
    ? buildStackedRows(series, categoryCount, percent)
    : series.map((s) => Array.from({ length: categoryCount }, (_, i) => s.values[i] ?? 0));

  let { minV, maxV } = valueRange(stackedSeries);
  // Line charts don't zero-clamp by default (Excel auto-scales to
  // data range), but explicit `<c:scaling>` bounds still override.
  const _lRange = resolveAxisRange(
    minV,
    maxV,
    chart.valueMin,
    chart.valueMax,
    /*zeroClamp=*/ false,
    AXIS_TICK_COUNT,
  );
  minV = _lRange.minV;
  maxV = _lRange.maxV;
  const ticks = _lRange.ticks;

  const inner = drawAxisFrame(ctx, chart, rect, ticks, minV, maxV, /*horizontal=*/ false, percent);

  // Category x-axis labels.
  drawCategoryAxis(ctx, chart, inner, categoryCount, /*horizontal=*/ false);

  const xStep = inner.w / Math.max(1, categoryCount - 1);
  const yFor = (v: number) => inner.y + (1 - (v - minV) / (maxV - minV)) * inner.h;

  // Excel default `<c:dispBlanksAs val="gap"/>` (ECMA-376 §21.2.2.34):
  // missing points break the line. Stacked rows from `buildStackedRows`
  // already fill gaps with 0 by construction (stacking semantics), so
  // this guard only fires for the unstacked path.
  const hasPointL = (s: ChartSeries, i: number): boolean => {
    if (stacked) return true;
    if (i >= s.values.length) return false;
    const v = s.values[i];
    return v != null && Number.isFinite(v);
  };

  for (let si = 0; si < series.length; si++) {
    const s = series[si]!;
    const data = stackedSeries[si]!;
    ctx.strokeStyle = s.color ?? "#4472C4";
    ctx.lineWidth = 2;
    ctx.beginPath();
    let penDown = false;
    for (let i = 0; i < categoryCount; i++) {
      if (!hasPointL(s, i)) {
        penDown = false;
        continue;
      }
      const x = inner.x + i * xStep;
      const y = yFor(data[i] ?? 0);
      if (!penDown) {
        ctx.moveTo(x, y);
        penDown = true;
      } else {
        ctx.lineTo(x, y);
      }
    }
    ctx.stroke();
    // Markers (small circles) — only at real data points. Skipped
    // when `<c:marker><c:symbol val="none"/>` was authored on the
    // series (e.g. chart32.xml's Technology line).
    if (s.markerSymbol !== "none") {
      ctx.fillStyle = s.color ?? "#4472C4";
      for (let i = 0; i < categoryCount; i++) {
        if (!hasPointL(s, i)) continue;
        const x = inner.x + i * xStep;
        const y = yFor(data[i] ?? 0);
        ctx.beginPath();
        ctx.arc(x, y, 3, 0, Math.PI * 2);
        ctx.fill();
      }
    }
    // Data labels (default position `t` above the marker).
    const dl = effectiveLabels(chart, s);
    if (dl) {
      const PAD = 5;
      for (let i = 0; i < categoryCount; i++) {
        if (!hasPointL(s, i)) continue;
        const po = pointLabel(dl, i);
        if (po === null) continue;
        const edl = po?.dl ?? dl;
        const pos = edl.position ?? "t";
        const v = s.values[i] ?? 0;
        const text = po?.text ?? buildLabelText(edl, chart, s, i, v, 0);
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
  // Bug #13 step 1: heavier zero baseline when the axis straddles zero.
  paintZeroBaseline(ctx, inner, minV, maxV);
}

// ---------- area ----------

function drawAreaChart(ctx: CanvasRenderingContext2D, chart: Chart, rect: Rect): void {
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
  );
  minV = _aRange.minV;
  maxV = _aRange.maxV;
  const ticks = _aRange.ticks;

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
  ctx.lineWidth = 1;
  // Bug #13 step 1: heavier zero baseline when the axis straddles zero.
  paintZeroBaseline(ctx, inner, minV, maxV);
}
