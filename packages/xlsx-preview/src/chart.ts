import type { Chart, ChartSeries } from "./types.js";
import type { ChartManualLayout } from "./schema/ChartManualLayout.js";
import type { ChartStyleBorder } from "./schema/ChartStyleBorder.js";
import { drawAreaChart } from "./chartArea.js";
import { drawBar3DChart, drawSurfaceChart } from "./chart3d.js";
import { drawTrendlines } from "./chartTrendline.js";
import { drawErrorBars } from "./chartErrorBars.js";
import {
  buildLabelText,
  buildStackedRows,
  catAxisRotation,
  valAxisRotation,
  rotatedLabelBandHeight,
  rotatedLabelBandWidth,
  drawRotatedLabel,
  categoryAxisExtraHeight,
  computeBarSlotMetrics,
  drawAxisFrame,
  drawCategoryAxis,
  drawCategoryAxisExtraRowsCentered,
  drawLabel,
  drawLegend,
  drawStyleBox,
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
  applyChartFontScale,
  AXIS_FONT_SIZE,
  LEGEND_FONT_SIZE,
  TITLE_FONT_SIZE,
  AXIS_TITLE_FONT_SIZE,
  resolveTitleFont,
  resolveAxisLabelFont,
} from "./chartUtils.js";

import {
  drawBubbleChart,
  drawChartEx,
  drawComboChart,
  drawOfPieChart,
  drawPieChart,
  drawRadarChart,
  drawStockChart,
  drawScatterChart,
  pieSliceColor,
  resolveBarFill,
  waterfallLegendEntries,
} from "./chartAdvanced.js";
import { advancedPointFill } from "./chartFills.js";

const TITLE_PAD = 8;
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

function drawTitleBox(
  ctx: CanvasRenderingContext2D,
  text: string,
  x: number,
  y: number,
  fontSize: number,
  align: CanvasTextAlign,
  fill?: string,
  border?: ChartStyleBorder,
  baseline: "top" | "middle" = "top",
): void {
  if ((!fill || fill === "none") && !border) return;
  const tw = ctx.measureText(text).width;
  const padX = Math.max(2, fontSize * 0.3);
  const padY = Math.max(2, fontSize * 0.25);
  let left = x;
  if (align === "center") left = x - tw / 2;
  else if (align === "right" || align === "end") left = x - tw;
  const top = baseline === "middle" ? y - fontSize / 2 : y;
  drawStyleBox(
    ctx,
    { x: left - padX, y: top - padY, w: tw + 2 * padX, h: fontSize + 2 * padY },
    fill,
    border,
  );
}

function resolveManualRect(chartRect: Rect, auto: Rect, ml: ChartManualLayout): Rect {
  const r: Rect = { ...auto };
  if (ml.x != null) {
    const off = ml.x * chartRect.w;
    r.x = ml.xMode === "factor" ? auto.x + off : chartRect.x + off;
  }
  if (ml.y != null) {
    const off = ml.y * chartRect.h;
    r.y = ml.yMode === "factor" ? auto.y + off : chartRect.y + off;
  }
  if (ml.w != null) r.w = ml.w * chartRect.w;
  if (ml.h != null) r.h = ml.h * chartRect.h;
  r.w = Math.max(8, Math.min(r.w, chartRect.x + chartRect.w - r.x));
  r.h = Math.max(8, Math.min(r.h, chartRect.y + chartRect.h - r.y));
  r.x = Math.max(chartRect.x, Math.min(r.x, chartRect.x + chartRect.w - r.w));
  r.y = Math.max(chartRect.y, Math.min(r.y, chartRect.y + chartRect.h - r.h));
  return r;
}

export function drawChart(ctx: CanvasRenderingContext2D, chart: Chart, rect: Rect): void {
  applyChartFontScale(rect);
  ctx.fillStyle = "#ffffff";
  ctx.fillRect(rect.x, rect.y, rect.w, rect.h);
  ctx.strokeStyle = "#d4d4d8";
  ctx.lineWidth = 1;
  ctx.strokeRect(rect.x + 0.5, rect.y + 0.5, rect.w - 1, rect.h - 1);

  let cursorY = rect.y + TITLE_PAD;
  if (chart.title) {
    const tf = resolveTitleFont(chart.titleFont, TITLE_FONT_SIZE);
    ctx.font = tf.css;
    ctx.textBaseline = "top";
    if (chart.titleLayout) {
      const tl = chart.titleLayout;
      let tx = rect.x + rect.w / 2;
      let ty = rect.y + TITLE_PAD;
      if (tl.x != null) {
        const off = tl.x * rect.w;
        tx = tl.xMode === "factor" ? rect.x + rect.w / 2 + off : rect.x + off;
        ctx.textAlign = "left";
      } else {
        ctx.textAlign = "center";
      }
      if (tl.y != null) {
        const off = tl.y * rect.h;
        ty = tl.yMode === "factor" ? rect.y + TITLE_PAD + off : rect.y + off;
      }
      tx = Math.max(rect.x + 2, Math.min(tx, rect.x + rect.w - 2));
      ty = Math.max(rect.y + 2, Math.min(ty, rect.y + rect.h - tf.size));
      drawTitleBox(
        ctx,
        chart.title,
        tx,
        ty,
        tf.size,
        ctx.textAlign,
        chart.titleFill,
        chart.titleBorder,
      );
      ctx.fillStyle = tf.color ?? TITLE_COLOR;
      ctx.fillText(chart.title, tx, ty);
    } else {
      ctx.textAlign = "center";
      drawTitleBox(
        ctx,
        chart.title,
        rect.x + rect.w / 2,
        cursorY,
        tf.size,
        "center",
        chart.titleFill,
        chart.titleBorder,
      );
      ctx.fillStyle = tf.color ?? TITLE_COLOR;
      ctx.fillText(chart.title, rect.x + rect.w / 2, cursorY);
      cursorY += tf.size + TITLE_PAD;
    }
  }

  const cats = chart.categories ?? [];
  const legendEntries: ChartSeries[] =
    (chart.type === "pie" || chart.type === "doughnut" || chart.type === "ofpie") &&
    chart.series.length > 0
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

  if (legendPos !== null && chart.legendLayout) {
    legendRect = resolveManualRect(rect, legendRect, chart.legendLayout);
  }

  let plotInner = true;
  if (chart.plotAreaLayout) {
    Object.assign(plotRect, resolveManualRect(rect, plotRect, chart.plotAreaLayout));
    plotInner = (chart.plotAreaLayout.layoutTarget ?? "inner") === "inner";
  }

  const AXIS_TITLE_PAD = 6;
  const AXIS_TITLE_BAND = AXIS_TITLE_FONT_SIZE + AXIS_TITLE_PAD;
  const xTitle = chart.xAxisTitle;
  const yTitle = chart.yAxisTitle;
  const yTitle2 = chart.yAxisTitleSecondary;
  const xTitleTf = resolveTitleFont(chart.xAxisTitleFont, AXIS_TITLE_FONT_SIZE);
  const yTitleTf = resolveTitleFont(chart.yAxisTitleFont, AXIS_TITLE_FONT_SIZE);
  const xTitleBand = xTitleTf.size + AXIS_TITLE_PAD;
  const yTitleBand = yTitleTf.size + AXIS_TITLE_PAD;
  let xTitleRect: Rect | null = null;
  let yTitleRect: Rect | null = null;
  let yTitle2Rect: Rect | null = null;
  if (xTitle && plotInner) {
    xTitleRect = {
      x: plotRect.x,
      y: plotRect.y + plotRect.h - xTitleBand,
      w: plotRect.w,
      h: xTitleBand,
    };
    plotRect.h -= xTitleBand;
  }
  if (yTitle && plotInner) {
    yTitleRect = {
      x: plotRect.x,
      y: plotRect.y,
      w: yTitleBand,
      h: plotRect.h,
    };
    plotRect.x += yTitleBand;
    plotRect.w -= yTitleBand;
  }
  if (yTitle2 && plotInner && (chart.secondaryAxis || chart.type === "combo")) {
    yTitle2Rect = {
      x: plotRect.x + plotRect.w - AXIS_TITLE_BAND,
      y: plotRect.y,
      w: AXIS_TITLE_BAND,
      h: plotRect.h,
    };
    plotRect.w -= AXIS_TITLE_BAND;
  }

  const DISP_UNITS_FONT_SIZE = AXIS_FONT_SIZE;
  const DISP_UNITS_BAND = DISP_UNITS_FONT_SIZE + 4;
  const duLabel = chart.dispUnits != null ? chart.dispUnitsLabel : undefined;
  const duLabel2 =
    chart.dispUnitsSecondary != null && (chart.secondaryAxis || chart.type === "combo")
      ? chart.dispUnitsLabelSecondary
      : undefined;
  let duBandRect: Rect | null = null;
  if ((duLabel || duLabel2) && plotInner) {
    duBandRect = {
      x: plotRect.x,
      y: plotRect.y,
      w: plotRect.w,
      h: DISP_UNITS_BAND,
    };
    plotRect.y += DISP_UNITS_BAND;
    plotRect.h -= DISP_UNITS_BAND;
  }

  const dataTableEligible =
    chart.dataTable != null &&
    chart.series.length > 0 &&
    chart.type !== "combo" &&
    !chart.secondaryAxis &&
    (chart.type === "column" || chart.type === "line" || chart.type === "area") &&
    chart.grouping !== "stacked" &&
    chart.grouping !== "percentstacked";
  let dataTableRect: Rect | null = null;
  if (dataTableEligible) {
    const bandH = dataTableBandHeight(chart);
    if (plotRect.h - bandH > 40) {
      dataTableRect = {
        x: plotRect.x,
        y: plotRect.y + plotRect.h - bandH,
        w: plotRect.w,
        h: bandH,
      };
      plotRect.h -= bandH;
    }
  }

  if (plotRect.w <= 20 || plotRect.h <= 20) return;

  drawStyleBox(ctx, plotRect, chart.plotAreaFill, chart.plotAreaBorder);

  if (chart.type === "surface") {
    drawSurfaceChart(ctx, chart, plotRect);
  } else if (chart.is3d && (chart.type === "column" || chart.type === "bar")) {
    drawBar3DChart(ctx, chart, plotRect);
  } else if (chart.type === "combo" || chart.secondaryAxis) {
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
      case "ofpie":
        drawOfPieChart(ctx, chart, plotRect);
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

  if (dataTableRect) {
    drawDataTable(ctx, chart, dataTableRect);
  }

  if (legendPos !== null) {
    drawLegend(ctx, legendEntries, legendRect, legendVertical ? "vertical" : "horizontal", chart);
  }

  if (xTitleRect && xTitle) {
    ctx.font = xTitleTf.css;
    ctx.textAlign = "center";
    ctx.textBaseline = "middle";
    const cx = xTitleRect.x + xTitleRect.w / 2;
    const cy = xTitleRect.y + xTitleRect.h / 2;
    drawTitleBox(
      ctx,
      xTitle,
      cx,
      cy,
      xTitleTf.size,
      "center",
      chart.xAxisTitleFill,
      chart.xAxisTitleBorder,
      "middle",
    );
    ctx.fillStyle = xTitleTf.color ?? TITLE_COLOR;
    ctx.fillText(xTitle, cx, cy);
  }
  if (yTitleRect && yTitle) {
    ctx.save();
    ctx.font = yTitleTf.css;
    ctx.textAlign = "center";
    ctx.textBaseline = "middle";
    ctx.translate(yTitleRect.x + yTitleRect.w / 2, yTitleRect.y + yTitleRect.h / 2);
    ctx.rotate(-Math.PI / 2);
    drawTitleBox(
      ctx,
      yTitle,
      0,
      0,
      yTitleTf.size,
      "center",
      chart.yAxisTitleFill,
      chart.yAxisTitleBorder,
      "middle",
    );
    ctx.fillStyle = yTitleTf.color ?? TITLE_COLOR;
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

const DATA_TABLE_ROW_H = AXIS_FONT_SIZE + 8;

function dataTableBandHeight(chart: Chart): number {
  const rows = chart.series.length + 1;
  return rows * DATA_TABLE_ROW_H;
}

function drawDataTable(ctx: CanvasRenderingContext2D, chart: Chart, rect: Rect): void {
  const dt = chart.dataTable;
  if (!dt) return;
  const series = chart.series;
  const categoryCount = Math.max(
    ...series.map((s) => s.values.length),
    (chart.categories ?? []).length,
  );
  if (categoryCount === 0) return;

  ctx.font = `${AXIS_FONT_SIZE}px -apple-system, "Helvetica Neue", Arial, sans-serif`;
  const swatchW = dt.showKeys ? 12 : 0;
  const swatchPad = dt.showKeys ? 4 : 0;
  const namePad = 6;
  const nameW = Math.max(
    0,
    ...series.map((s) => ctx.measureText(s.name || "").width),
    ...(chart.categories ?? []).map(() => 0),
  );
  const headerW = Math.min(
    rect.w * 0.4,
    Math.max(48, swatchW + swatchPad + nameW + namePad * 2 + 4),
  );
  const gridX = rect.x + headerW;
  const colW = (rect.w - headerW) / categoryCount;
  const rowH = rect.h / (series.length + 1);

  ctx.textBaseline = "middle";

  ctx.fillStyle = AXIS_LABEL_COLOR;
  ctx.textAlign = "center";
  for (let c = 0; c < categoryCount; c++) {
    const label = (chart.categories ?? [])[c] ?? `${c + 1}`;
    ctx.fillText(label, gridX + colW * (c + 0.5), rect.y + rowH * 0.5, colW - 4);
  }

  for (let r = 0; r < series.length; r++) {
    const s = series[r]!;
    const cy = rect.y + rowH * (r + 1.5);
    let tx = rect.x + namePad;
    if (dt.showKeys) {
      ctx.fillStyle = s.color ?? "#4472C4";
      ctx.fillRect(tx, cy - swatchW / 2, swatchW, swatchW);
      tx += swatchW + swatchPad;
    }
    ctx.fillStyle = AXIS_LABEL_COLOR;
    ctx.textAlign = "left";
    ctx.fillText(s.name || `Series ${r + 1}`, tx, cy, headerW - (tx - rect.x) - namePad);
    ctx.textAlign = "center";
    for (let c = 0; c < categoryCount; c++) {
      const v = s.values[c];
      if (v == null || !Number.isFinite(v)) continue;
      const text = formatAxisValue(v, chart.valueFormat, chart.dispUnits);
      ctx.fillText(text, gridX + colW * (c + 0.5), cy, colW - 4);
    }
  }

  ctx.strokeStyle = "#bfbfbf";
  ctx.lineWidth = 1;
  if (dt.showHorzBorder) {
    for (let r = 1; r <= series.length; r++) {
      const y = Math.round(rect.y + rowH * r) + 0.5;
      ctx.beginPath();
      ctx.moveTo(rect.x, y);
      ctx.lineTo(rect.x + rect.w, y);
      ctx.stroke();
    }
  }
  if (dt.showVertBorder) {
    for (let c = 0; c <= categoryCount; c++) {
      const x = Math.round(gridX + colW * c) + 0.5;
      ctx.beginPath();
      ctx.moveTo(x, rect.y);
      ctx.lineTo(x, rect.y + rect.h);
      ctx.stroke();
    }
    const hx = Math.round(rect.x) + 0.5;
    ctx.beginPath();
    ctx.moveTo(hx, rect.y);
    ctx.lineTo(hx, rect.y + rect.h);
    ctx.stroke();
  }
  if (dt.showOutline) {
    ctx.strokeRect(
      Math.round(rect.x) + 0.5,
      Math.round(rect.y) + 0.5,
      Math.round(rect.w) - 1,
      Math.round(rect.h) - 1,
    );
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
      AXIS_TICK_COUNT,
      chart.majorUnit,
    );
    minV = _bcRange.minV;
    maxV = _bcRange.maxV;
    ticks = _bcRange.ticks;
  }

  const valTf = resolveAxisLabelFont(chart.valAxisLabelFont);
  const catTf = resolveAxisLabelFont(chart.catAxisLabelFont);
  ctx.font = valTf.css;
  const labelStrings = ticks.map((t) =>
    percent ? `${Math.round(t)}%` : formatAxisValue(t, chart.valueFormat, chart.dispUnits),
  );
  const catRot = catAxisRotation(chart);
  const valRot = valAxisRotation(chart);
  const catLabels = Array.from(
    { length: categoryCount },
    (_, i) => (chart.categories ?? [])[i] ?? `${i + 1}`,
  );
  const yAxisW =
    !horizontal && valRot !== 0
      ? rotatedLabelBandWidth(ctx, labelStrings, valRot) + 8
      : Math.max(...labelStrings.map((s) => ctx.measureText(s).width)) + 8;
  const xAxisH =
    AXIS_FONT_SIZE +
    8 +
    (horizontal
      ? valRot !== 0
        ? rotatedLabelBandHeight(ctx, labelStrings, valRot)
        : 0
      : categoryAxisExtraHeight(chart) +
        (catRot !== 0 ? rotatedLabelBandHeight(ctx, catLabels, catRot) : 0));

  const innerRect: Rect = horizontal
    ? { x: rect.x + yAxisW, y: rect.y, w: rect.w - yAxisW, h: rect.h - xAxisH }
    : { x: rect.x + yAxisW, y: rect.y, w: rect.w - yAxisW, h: rect.h - xAxisH };

  const showGridlines = chart.showMajorGridlines !== false;
  ctx.fillStyle = valTf.color!;
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
      if (valRot !== 0) {
        drawRotatedLabel(ctx, labelStrings[ti]!, x, innerRect.y + innerRect.h + 6, valRot, "value");
      } else {
        ctx.fillText(labelStrings[ti]!, x, innerRect.y + innerRect.h + xAxisH / 2);
      }
    } else {
      const y = innerRect.y + (1 - frac) * innerRect.h;
      if (showGridlines && !isZeroLine) {
        ctx.beginPath();
        ctx.moveTo(innerRect.x, Math.round(y) + 0.5);
        ctx.lineTo(innerRect.x + innerRect.w, Math.round(y) + 0.5);
        ctx.stroke();
      }
      if (valRot !== 0) {
        drawRotatedLabel(ctx, labelStrings[ti]!, innerRect.x - 4, y, valRot, "value");
      } else {
        ctx.fillText(labelStrings[ti]!, innerRect.x - 4, y);
      }
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

  ctx.font = catTf.css;
  ctx.fillStyle = catTf.color!;
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
    } else if (catRot !== 0) {
      drawRotatedLabel(ctx, label, center, innerRect.y + innerRect.h + 4, catRot, "category");
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
        const c = clampFill(bx, by, bw, bh);
        const adv = advancedPointFill(ctx, s, i, { x: c.x, y: c.y, w: c.w, h: c.h });
        ctx.fillStyle = adv ?? fill.color;
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
        const c = clampFill(bx, by, bw, bh);
        const adv = advancedPointFill(ctx, s, i, { x: c.x, y: c.y, w: c.w, h: c.h });
        ctx.fillStyle = adv ?? fill.color;
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

  if (!horizontal && series.some((s) => s.errorBars)) {
    ctx.save();
    ctx.beginPath();
    ctx.rect(innerRect.x, innerRect.y, innerRect.w, innerRect.h);
    ctx.clip();
    const yPix = (v: number) => innerRect.y + (1 - (v - minV) / (maxV - minV)) * innerRect.h;
    for (let si = 0; si < series.length; si++) {
      const s = series[si]!;
      if (!s.errorBars) continue;
      const xsIdx = Array.from({ length: categoryCount }, (_, i) => i);
      const ys = Array.from({ length: categoryCount }, (_, i) => s.values[i] ?? 0);
      const xCenter = (i: number) =>
        innerRect.x + i * groupGap + slot.firstBarLeftOffset + si * slot.barShift + barSize / 2;
      drawErrorBars(ctx, s, xsIdx, ys, xCenter, yPix);
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
    if (!s.lineNone) {
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
    }

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

    if (s.errorBars) {
      const xsIdx = Array.from({ length: categoryCount }, (_, i) => i);
      drawErrorBars(ctx, s, xsIdx, data, (x) => inner.x + x * xStep, yFor);
    }
  }
  ctx.restore();
  ctx.lineWidth = 1;

  paintZeroBaseline(ctx, inner, minV, maxV);
}
