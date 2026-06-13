import type { Chart, ChartSeries } from "./types.js";
import { drawAreaChart } from "./chartArea.js";
import { drawTrendlines } from "./chartTrendline.js";
import {
  buildLabelText,
  buildStackedRows,
  categoryAxisExtraHeight,
  computeBarSlotMetrics,
  drawAxisFrame,
  drawCategoryAxis,
  drawCategoryAxisExtraRowsCentered,
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
  seriesLineWidth,
  seriesLineDash,
} from "./chartUtils.js";

import {
  drawBubbleChart,
  drawChartEx,
  drawComboChart,
  drawPieChart,
  drawRadarChart,
  drawStockChart,
  drawScatterChart,
  pieSliceColor,
  resolveBarFill,
  waterfallLegendEntries,
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
  ctx.fillStyle = "#ffffff";
  ctx.fillRect(rect.x, rect.y, rect.w, rect.h);
  ctx.strokeStyle = "#d4d4d8";
  ctx.lineWidth = 1;
  ctx.strokeRect(rect.x + 0.5, rect.y + 0.5, rect.w - 1, rect.h - 1);

  let cursorY = rect.y + TITLE_PAD;
  if (chart.title) {
    ctx.fillStyle = TITLE_COLOR;
    ctx.font = `${TITLE_FONT_SIZE}px -apple-system, "Helvetica Neue", Arial, sans-serif`;
    ctx.textAlign = "center";
    ctx.textBaseline = "top";
    ctx.fillText(chart.title, rect.x + rect.w / 2, cursorY);
    cursorY += TITLE_FONT_SIZE + TITLE_PAD;
  }

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
      : chart.type === "chartex" && chart.cxLayout === "waterfall"
        ? waterfallLegendEntries(chart)
        : chart.type === "chartex" &&
            (chart.cxLayout === "funnel" ||
              chart.cxLayout === "treemap" ||
              chart.cxLayout === "sunburst" ||
              chart.cxLayout === "regionMap")
          ? []
          : chart.series;

  const legendPos =
    (chart.series.length > 0 || legendEntries.length > 0) && chart.legendPos
      ? chart.legendPos
      : null;
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
      case "radar":
        drawRadarChart(ctx, chart, plotRect);
        break;
      case "stock":
        drawStockChart(ctx, chart, plotRect);
        break;
      case "chartex":
        drawChartEx(ctx, chart, plotRect);
        break;
      default:
        drawPlaceholderPlot(ctx, chart, plotRect);
    }
  }

  if (legendPos !== null) {
    drawLegend(ctx, legendEntries, legendRect, legendVertical ? "vertical" : "horizontal", chart);
  }

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

    ctx.translate(yTitle2Rect.x + yTitle2Rect.w / 2, yTitle2Rect.y + yTitle2Rect.h / 2);
    ctx.rotate(Math.PI / 2);
    ctx.fillText(yTitle2, 0, 0);
    ctx.restore();
  }

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

function drawBarColumnChart(ctx: CanvasRenderingContext2D, chart: Chart, rect: Rect): void {
  const horizontal = chart.type === "bar";
  const stacked = chart.grouping === "stacked" || chart.grouping === "percentstacked";
  const percent = chart.grouping === "percentstacked";

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

  let ticks: number[];
  if (percent) {
    minV = 0;
    maxV = 100;
    ticks = [0, 25, 50, 75, 100];
  } else {
    const _bcRange = resolveAxisRange(
      minV,
      maxV,
      chart.valueMin,
      chart.valueMax,
      true,
      AXIS_TICK_COUNT,
      chart.majorUnit,
    );
    minV = _bcRange.minV;
    maxV = _bcRange.maxV;
    ticks = _bcRange.ticks;
  }

  ctx.font = `${AXIS_FONT_SIZE}px -apple-system, "Helvetica Neue", Arial, sans-serif`;
  const labelStrings = ticks.map((t) =>
    percent ? `${Math.round(t)}%` : formatAxisValue(t, chart.valueFormat, chart.dispUnits),
  );
  const yAxisW = Math.max(...labelStrings.map((s) => ctx.measureText(s).width)) + 8;
  const xAxisH = AXIS_FONT_SIZE + 8 + (horizontal ? 0 : categoryAxisExtraHeight(chart));

  const innerRect: Rect = horizontal
    ? { x: rect.x + yAxisW, y: rect.y, w: rect.w - yAxisW, h: rect.h - xAxisH }
    : { x: rect.x + yAxisW, y: rect.y, w: rect.w - yAxisW, h: rect.h - xAxisH };

  const showGridlines = chart.showMajorGridlines !== false;
  ctx.fillStyle = AXIS_LABEL_COLOR;
  ctx.strokeStyle = GRIDLINE_COLOR;
  ctx.lineWidth = 1;
  ctx.textAlign = horizontal ? "center" : "right";
  ctx.textBaseline = "middle";
  for (let ti = 0; ti < ticks.length; ti++) {
    const t = ticks[ti]!;
    const frac = (t - minV) / (maxV - minV);

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

  const groupGap = horizontal ? innerRect.h / categoryCount : innerRect.w / categoryCount;
  const slot = computeBarSlotMetrics(
    groupGap,
    series.length,
    stacked,
    chart.barGapWidth,
    chart.barOverlap,
  );
  const barSize = slot.barW;

  const zMetrics = zeroAxisMetrics(innerRect, minV, maxV);
  const zeroY = zMetrics.zeroY;
  const zeroX = zMetrics.zeroX;

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
  if (!horizontal) {
    drawCategoryAxisExtraRowsCentered(
      ctx,
      chart,
      innerRect,
      categoryCount,
      (i) => innerRect.x + (i + 0.5) * groupGap,
    );
  }
  ctx.textAlign = "left";

  const plotTop = innerRect.y;
  const plotBot = innerRect.y + innerRect.h;
  const plotLeft = innerRect.x;
  const plotRight = innerRect.x + innerRect.w;
  const clampFill = (bx: number, by: number, bw: number, bh: number) => {
    const x1 = Math.max(plotLeft, bx);
    const x2 = Math.min(plotRight, bx + bw);
    const y1 = Math.max(plotTop, by);
    const y2 = Math.min(plotBot, by + bh);
    return { x: x1, y: y1, w: Math.max(0, x2 - x1), h: Math.max(0, y2 - y1) };
  };

  if (stacked) {
    for (let i = 0; i < categoryCount; i++) {
      const groupCenter = horizontal
        ? innerRect.y + (i + 0.5) * groupGap
        : innerRect.x + (i + 0.5) * groupGap;
      let pos = 0,
        neg = 0;

      let catTotal = 0;
      for (const s of series) catTotal += Math.max(0, s.values[i] ?? 0);
      const scale = percent && catTotal > 0 ? 100 / catTotal : 1;
      for (const s of series) {
        const raw = s.values[i] ?? 0;
        const v = raw * scale;
        const start = v >= 0 ? pos : neg;
        const end = v >= 0 ? pos + v : neg + v;

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
        if (fill.skip) continue;
        ctx.fillStyle = fill.color;
        const c = clampFill(bx, by, bw, bh);
        if (c.w > 0 && c.h > 0) ctx.fillRect(c.x, c.y, c.w, c.h);

        const dl = effectiveLabels(chart, s);
        if (dl) {
          const po = pointLabel(dl, i);
          if (po !== null) {
            const edl = po?.dl ?? dl;
            const text = po?.text ?? buildLabelText(edl, chart, s, i, raw, catTotal);
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
        if (fill.skip) continue;
        ctx.fillStyle = fill.color;
        const c = clampFill(bx, by, bw, bh);
        if (c.w > 0 && c.h > 0) ctx.fillRect(c.x, c.y, c.w, c.h);
        const dl = effectiveLabels(chart, s);
        if (dl) {
          const po = pointLabel(dl, i);
          if (po === null) continue;
          const edl = po?.dl ?? dl;
          const text = po?.text ?? buildLabelText(edl, chart, s, i, v, 0);

          const pos = edl.position ?? "outEnd";
          let lx = bx + bw / 2,
            ly = by + bh / 2;
          const PAD = 3;
          if (horizontal) {
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

  if (!horizontal && series.some((s) => (s.trendlines?.length ?? 0) > 0)) {
    ctx.save();
    ctx.beginPath();
    ctx.rect(innerRect.x, innerRect.y, innerRect.w, innerRect.h);
    ctx.clip();
    const yPix = (v: number) => innerRect.y + (1 - (v - minV) / (maxV - minV)) * innerRect.h;
    const xPix = (x: number) => innerRect.x + (x + 0.5) * groupGap;
    for (const s of series) {
      if ((s.trendlines?.length ?? 0) === 0) continue;
      const xsIdx = Array.from({ length: categoryCount }, (_, i) => i);
      const ys = Array.from({ length: categoryCount }, (_, i) => s.values[i] ?? 0);
      drawTrendlines(ctx, s, xsIdx, ys, xPix, yPix);
    }
    ctx.restore();
  }

  if (zMetrics.straddlesZero) {
    paintZeroBaseline(ctx, innerRect, minV, maxV);
  } else {
    ctx.strokeStyle = "#9ca3af";
    ctx.beginPath();
    ctx.moveTo(innerRect.x, Math.round(zeroY) + 0.5);
    ctx.lineTo(innerRect.x + innerRect.w, Math.round(zeroY) + 0.5);
    ctx.stroke();
  }

  ctx.strokeStyle = "#9ca3af";
  ctx.lineWidth = 1;
  ctx.beginPath();
  ctx.moveTo(Math.round(innerRect.x) + 0.5, innerRect.y);
  ctx.lineTo(Math.round(innerRect.x) + 0.5, innerRect.y + innerRect.h);
  ctx.stroke();
}

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

  const stackedSeries: number[][] = stacked
    ? buildStackedRows(series, categoryCount, percent)
    : series.map((s) => Array.from({ length: categoryCount }, (_, i) => s.values[i] ?? 0));

  let { minV, maxV } = valueRange(stackedSeries);

  const _lRange = resolveAxisRange(
    minV,
    maxV,
    chart.valueMin,
    chart.valueMax,
    false,
    AXIS_TICK_COUNT,
    chart.majorUnit,
  );
  minV = _lRange.minV;
  maxV = _lRange.maxV;
  const ticks = _lRange.ticks;

  const inner = drawAxisFrame(ctx, chart, rect, ticks, minV, maxV, false, percent);

  drawCategoryAxis(ctx, chart, inner, categoryCount, false);

  const xStep = inner.w / Math.max(1, categoryCount - 1);
  const yFor = (v: number) => inner.y + (1 - (v - minV) / (maxV - minV)) * inner.h;

  const hasPointL = (s: ChartSeries, i: number): boolean => {
    if (stacked) return true;
    if (i >= s.values.length) return false;
    const v = s.values[i];
    return v != null && Number.isFinite(v);
  };

  ctx.save();
  ctx.beginPath();
  ctx.rect(inner.x, inner.y, inner.w, inner.h);
  ctx.clip();

  for (let si = 0; si < series.length; si++) {
    const s = series[si]!;
    const data = stackedSeries[si]!;
    ctx.strokeStyle = s.color ?? "#4472C4";
    ctx.lineWidth = seriesLineWidth(s, 2);
    ctx.setLineDash(seriesLineDash(s));
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
    ctx.setLineDash([]);

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

    if ((s.trendlines?.length ?? 0) > 0) {
      const xsIdx = Array.from({ length: categoryCount }, (_, i) => i);
      drawTrendlines(ctx, s, xsIdx, data, (x) => inner.x + x * xStep, yFor);
    }
  }
  ctx.restore();
  ctx.lineWidth = 1;

  paintZeroBaseline(ctx, inner, minV, maxV);
}
