import type { Chart, ChartSeries, DataLabels } from "./types.js";
import type { Rect } from "./chart.js";
import type { ChartStyleBorder } from "./schema/ChartStyleBorder.js";
import type { ChartStyleFont } from "./schema/ChartStyleFont.js";

const AXIS_FONT_SIZE = 10;
const LEGEND_FONT_SIZE = 11;
const GRIDLINE_COLOR = "#e5e7eb";
const AXIS_LABEL_COLOR = "#52525b";
const LEGEND_FONT_FAMILY = `-apple-system, "Helvetica Neue", Arial, sans-serif`;

function styleDashFor(dash: string | undefined): number[] {
  switch (dash) {
    case "dot":
    case "sysDot":
      return [1, 3];
    case "dash":
      return [4, 3];
    case "lgDash":
      return [8, 3];
    case "dashDot":
      return [4, 3, 1, 3];
    case "lgDashDot":
      return [8, 3, 1, 3];
    case "sysDash":
      return [3, 1];
    default:
      return [];
  }
}

export function drawStyleBox(
  ctx: CanvasRenderingContext2D,
  rect: Rect,
  fill?: string,
  border?: ChartStyleBorder,
): void {
  if (fill && fill !== "none") {
    ctx.fillStyle = fill;
    ctx.fillRect(rect.x, rect.y, rect.w, rect.h);
  }
  if (border && (border.color || border.widthEmu != null || border.dash)) {
    ctx.save();
    ctx.strokeStyle = border.color ?? "#000000";
    ctx.lineWidth = border.widthEmu != null ? Math.max(0.5, border.widthEmu / 12700) : 1;
    ctx.setLineDash(styleDashFor(border.dash));
    ctx.strokeRect(rect.x, rect.y, rect.w, rect.h);
    ctx.restore();
  }
}

export function chartFontCss(font: ChartStyleFont | undefined, fallbackSize: number): string {
  const size = font?.sizePt ?? fallbackSize;
  const family = font?.typeface ? `"${font.typeface}", ${LEGEND_FONT_FAMILY}` : LEGEND_FONT_FAMILY;
  const style = font?.italic ? "italic " : "";
  const weight = font?.bold ? "bold " : "";
  return `${style}${weight}${size}px ${family}`;
}

const ZERO_BASELINE_COLOR = "#7a7a7a";
const ZERO_BASELINE_WIDTH = 1.5;
const ZERO_EPS = 1e-9;

export function axisStraddlesZero(minV: number, maxV: number): boolean {
  return minV < -ZERO_EPS && maxV > ZERO_EPS;
}

export interface ZeroAxisMetrics {
  straddlesZero: boolean;
  zeroFrac: number;
  zeroY: number;
  zeroX: number;
}
export function zeroAxisMetrics(inner: Rect, minV: number, maxV: number): ZeroAxisMetrics {
  const range = maxV - minV;

  const rawFrac = range > 0 ? (0 - minV) / range : 0;
  const zeroFrac = Math.max(0, Math.min(1, rawFrac));
  return {
    straddlesZero: axisStraddlesZero(minV, maxV),
    zeroFrac,
    zeroY: inner.y + (1 - zeroFrac) * inner.h,
    zeroX: inner.x + zeroFrac * inner.w,
  };
}

export function paintZeroBaseline(
  ctx: CanvasRenderingContext2D,
  inner: Rect,
  minV: number,
  maxV: number,
): void {
  const z = zeroAxisMetrics(inner, minV, maxV);
  if (!z.straddlesZero) return;
  const y = z.zeroY;
  const prevStroke = ctx.strokeStyle;
  const prevWidth = ctx.lineWidth;
  ctx.strokeStyle = ZERO_BASELINE_COLOR;
  ctx.lineWidth = ZERO_BASELINE_WIDTH;
  ctx.beginPath();

  ctx.moveTo(inner.x, Math.round(y));
  ctx.lineTo(inner.x + inner.w, Math.round(y));
  ctx.stroke();
  ctx.strokeStyle = prevStroke;
  ctx.lineWidth = prevWidth;
}

export function isZeroTickInside(t: number, minV: number, maxV: number): boolean {
  return Math.abs(t) < ZERO_EPS && axisStraddlesZero(minV, maxV);
}
const DATA_LABEL_FONT_SIZE = 9;
const DATA_LABEL_COLOR = "#1f2937";

export interface BarSlotMetrics {
  barW: number;
  firstBarLeftOffset: number;
  barShift: number;
}
export function computeBarSlotMetrics(
  slotW: number,
  seriesCount: number,
  stacked: boolean,
  gapWidthPct: number | undefined,
  overlapPct: number | undefined,
): BarSlotMetrics {
  const gw = Math.max(0, Math.min(500, gapWidthPct ?? 150));
  const ov = stacked ? 100 : Math.max(-100, Math.min(100, overlapPct ?? 0));
  const N = stacked ? 1 : Math.max(1, seriesCount);
  const shiftFactor = 1 - ov / 100;
  const denom = 1 + (N - 1) * shiftFactor + gw / 100;
  const barW = slotW / denom;
  const totalSpan = barW * (1 + (N - 1) * shiftFactor);
  const firstBarLeftOffset = (slotW - totalSpan) / 2;
  const barShift = barW * shiftFactor;
  return { barW, firstBarLeftOffset, barShift };
}

export function valueRange(rows: number[][]): { minV: number; maxV: number } {
  let minV = Number.POSITIVE_INFINITY,
    maxV = Number.NEGATIVE_INFINITY;
  for (const r of rows) {
    for (const v of r) {
      if (v > maxV) maxV = v;
      if (v < minV) minV = v;
    }
  }
  if (!Number.isFinite(minV)) minV = 0;
  if (!Number.isFinite(maxV)) maxV = 1;
  return { minV, maxV };
}

export function buildStackedRows(
  series: ChartSeries[],
  categoryCount: number,
  percent: boolean,
): number[][] {
  const tops: number[][] = series.map((_) => new Array(categoryCount).fill(0));
  for (let i = 0; i < categoryCount; i++) {
    let total = 0;
    if (percent) {
      for (const s of series) total += Math.max(0, s.values[i] ?? 0);
      if (total <= 0) total = 1;
    }
    let acc = 0;
    for (let si = 0; si < series.length; si++) {
      const raw = series[si]!.values[i] ?? 0;
      const v = percent ? (Math.max(0, raw) / total) * 100 : raw;
      acc += v;
      tops[si]![i] = acc;
    }
  }
  return tops;
}

export function categoryAxisExtraRows(chart: Chart): string[][] {
  const lv = chart.cxCategoryLevels ?? [];
  if (lv.length <= 1) return [];
  const out: string[][] = [];
  for (let i = lv.length - 2; i >= 0; i--) out.push(lv[i] ?? []);
  return out;
}

export function categoryAxisExtraHeight(chart: Chart): number {
  return categoryAxisExtraRows(chart).length * (AXIS_FONT_SIZE + 4);
}

export function catAxisRotation(chart: Chart): number {
  const r = chart.catAxisLabelRotation ?? 0;
  return Number.isFinite(r) && r !== 0 ? r : 0;
}

export function valAxisRotation(chart: Chart): number {
  const r = chart.valAxisLabelRotation ?? 0;
  return Number.isFinite(r) && r !== 0 ? r : 0;
}

export function rotatedLabelBandHeight(
  ctx: CanvasRenderingContext2D,
  labels: string[],
  rotationDeg: number,
): number {
  if (rotationDeg === 0) return 0;
  const rad = (Math.abs(rotationDeg) * Math.PI) / 180;
  const maxW = Math.max(0, ...labels.map((s) => ctx.measureText(s).width));
  return maxW * Math.sin(rad) + AXIS_FONT_SIZE * Math.cos(rad);
}

export function rotatedLabelBandWidth(
  ctx: CanvasRenderingContext2D,
  labels: string[],
  rotationDeg: number,
): number {
  if (rotationDeg === 0) return 0;
  const rad = (Math.abs(rotationDeg) * Math.PI) / 180;
  const maxW = Math.max(0, ...labels.map((s) => ctx.measureText(s).width));
  return maxW * Math.cos(rad) + AXIS_FONT_SIZE * Math.sin(rad);
}

export function drawRotatedLabel(
  ctx: CanvasRenderingContext2D,
  text: string,
  anchorX: number,
  anchorY: number,
  rotationDeg: number,
  kind: "category" | "value",
): void {
  const rad = (rotationDeg * Math.PI) / 180;
  ctx.save();
  ctx.translate(anchorX, anchorY);
  ctx.rotate(rad);
  if (kind === "category") {
    ctx.textAlign = rotationDeg < 0 ? "right" : "left";
    ctx.textBaseline = "middle";
  } else {
    ctx.textAlign = "right";
    ctx.textBaseline = "middle";
  }
  ctx.fillText(text, 0, 0);
  ctx.restore();
}

export function drawAxisFrame(
  ctx: CanvasRenderingContext2D,
  chart: Chart,
  rect: Rect,
  ticks: number[],
  minV: number,
  maxV: number,
  horizontal: boolean,
  percent: boolean,
): Rect {
  ctx.font = `${AXIS_FONT_SIZE}px -apple-system, "Helvetica Neue", Arial, sans-serif`;
  const labelStrings = ticks.map((t) =>
    percent ? `${Math.round(t)}%` : formatAxisValue(t, chart.valueFormat, chart.dispUnits),
  );
  const valRot = valAxisRotation(chart);
  const yAxisW = horizontal
    ? Math.max(...labelStrings.map((s) => ctx.measureText(s).width)) + 8
    : valRot !== 0
      ? rotatedLabelBandWidth(ctx, labelStrings, valRot) + 8
      : Math.max(...labelStrings.map((s) => ctx.measureText(s).width)) + 8;
  const catRot = catAxisRotation(chart);
  const catBand =
    !horizontal && catRot !== 0
      ? rotatedLabelBandHeight(ctx, chart.categories ?? [], catRot)
      : 0;
  const xAxisH =
    AXIS_FONT_SIZE +
    8 +
    (horizontal
      ? valRot !== 0
        ? rotatedLabelBandHeight(ctx, labelStrings, valRot)
        : 0
      : categoryAxisExtraHeight(chart) + catBand);
  const inner: Rect = horizontal
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
      const x = inner.x + frac * inner.w;
      if (showGridlines && !isZeroLine) {
        ctx.beginPath();
        ctx.moveTo(Math.round(x) + 0.5, inner.y);
        ctx.lineTo(Math.round(x) + 0.5, inner.y + inner.h);
        ctx.stroke();
      }
      if (valRot !== 0) {
        drawRotatedLabel(ctx, labelStrings[ti]!, x, inner.y + inner.h + 6, valRot, "value");
      } else {
        ctx.fillText(labelStrings[ti]!, x, inner.y + inner.h + xAxisH / 2);
      }
    } else {
      const y = inner.y + (1 - frac) * inner.h;
      if (showGridlines && !isZeroLine) {
        ctx.beginPath();
        ctx.moveTo(inner.x, Math.round(y) + 0.5);
        ctx.lineTo(inner.x + inner.w, Math.round(y) + 0.5);
        ctx.stroke();
      }
      if (valRot !== 0) {
        drawRotatedLabel(ctx, labelStrings[ti]!, inner.x - 4, y, valRot, "value");
      } else {
        ctx.fillText(labelStrings[ti]!, inner.x - 4, y);
      }
    }
  }

  ctx.strokeStyle = "#9ca3af";
  ctx.beginPath();
  ctx.moveTo(inner.x, Math.round(inner.y + inner.h) + 0.5);
  ctx.lineTo(inner.x + inner.w, Math.round(inner.y + inner.h) + 0.5);
  ctx.moveTo(Math.round(inner.x) + 0.5, inner.y);
  ctx.lineTo(Math.round(inner.x) + 0.5, inner.y + inner.h);
  ctx.stroke();
  return inner;
}

export function drawCategoryAxis(
  ctx: CanvasRenderingContext2D,
  chart: Chart,
  inner: Rect,
  categoryCount: number,
  horizontal: boolean,
): void {
  ctx.font = `${AXIS_FONT_SIZE}px -apple-system, "Helvetica Neue", Arial, sans-serif`;
  ctx.fillStyle = AXIS_LABEL_COLOR;
  ctx.textAlign = "center";
  ctx.textBaseline = horizontal ? "middle" : "top";

  const denom = Math.max(1, categoryCount - 1);

  const fmt = chart.categoriesFormat;
  const cats = chart.categories ?? [];
  const labels = Array.from({ length: categoryCount }, (_, i) => {
    const raw = cats[i] ?? `${i + 1}`;
    if (!fmt) return raw;
    const n = parseFloat(raw);
    if (!Number.isFinite(n)) return raw;
    return formatValue(n, fmt).text;
  });
  const catRot = catAxisRotation(chart);
  const minGapPx = 8;
  let lastRight = -Infinity;
  for (let i = 0; i < categoryCount; i++) {
    const label = labels[i]!;
    const w = ctx.measureText(label).width;
    if (horizontal) {
      ctx.fillText(label, inner.x - 4, inner.y + (i / denom) * inner.h);
      continue;
    }
    const cx = inner.x + (i / denom) * inner.w;
    if (catRot !== 0) {
      drawRotatedLabel(ctx, label, cx, inner.y + inner.h + 4, catRot, "category");
      continue;
    }
    const left = cx - w / 2;
    if (left < lastRight + minGapPx) continue;
    ctx.fillText(label, cx, inner.y + inner.h + 4);
    lastRight = cx + w / 2;
  }

  if (!horizontal) {
    const extras = categoryAxisExtraRows(chart);
    const rowH = AXIS_FONT_SIZE + 4;
    for (let ri = 0; ri < extras.length; ri++) {
      const row = extras[ri]!;
      const yBase = inner.y + inner.h + 4 + (ri + 1) * rowH;
      let lastRightR = -Infinity;
      for (let i = 0; i < categoryCount; i++) {
        const label = row[i] ?? "";
        if (!label) continue;
        const cx = inner.x + (i / denom) * inner.w;
        const w = ctx.measureText(label).width;
        const left = cx - w / 2;
        if (left < lastRightR + minGapPx) continue;
        ctx.fillText(label, cx, yBase);
        lastRightR = cx + w / 2;
      }
    }
  }
}

export function drawCategoryAxisExtraRowsCentered(
  ctx: CanvasRenderingContext2D,
  chart: Chart,
  inner: Rect,
  categoryCount: number,
  xCenterFor: (i: number) => number,
): void {
  const extras = categoryAxisExtraRows(chart);
  if (extras.length === 0) return;
  ctx.font = `${AXIS_FONT_SIZE}px -apple-system, "Helvetica Neue", Arial, sans-serif`;
  ctx.fillStyle = AXIS_LABEL_COLOR;
  ctx.textAlign = "center";
  ctx.textBaseline = "top";
  const rowH = AXIS_FONT_SIZE + 4;
  const minGapPx = 8;
  for (let ri = 0; ri < extras.length; ri++) {
    const row = extras[ri]!;
    const y = inner.y + inner.h + 4 + (ri + 1) * rowH;
    let lastRight = -Infinity;
    for (let i = 0; i < categoryCount; i++) {
      const label = row[i] ?? "";
      if (!label) continue;
      const cx = xCenterFor(i);
      const w = ctx.measureText(label).width;
      const left = cx - w / 2;
      if (left < lastRight + minGapPx) continue;
      ctx.fillText(label, cx, y);
      lastRight = cx + w / 2;
    }
  }
}

export function seriesLineWidth(s: ChartSeries, fallback: number): number {
  if (s.lineWidthEmu == null) return fallback;
  return Math.max(0.5, s.lineWidthEmu / 12700);
}

export function seriesLineDash(s: ChartSeries): number[] {
  switch (s.lineDash) {
    case "dot":
      return [1, 3];
    case "dash":
      return [4, 3];
    case "lgDash":
      return [8, 3];
    case "dashDot":
      return [4, 3, 1, 3];
    case "lgDashDot":
      return [8, 3, 1, 3];
    case "lgDashDotDot":
      return [8, 3, 1, 3, 1, 3];
    case "sysDash":
      return [3, 1];
    case "sysDot":
      return [1, 1];
    case "sysDashDot":
      return [3, 1, 1, 1];
    case "sysDashDotDot":
      return [3, 1, 1, 1, 1, 1];
    default:
      return [];
  }
}

export function withAlpha(color: string, alpha: number): string {
  const m = /^#([0-9a-f]{6})$/i.exec(color);
  if (!m) return color;
  const hex = m[1]!;
  const r = parseInt(hex.slice(0, 2), 16);
  const g = parseInt(hex.slice(2, 4), 16);
  const b = parseInt(hex.slice(4, 6), 16);
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

import { formatValue } from "./numfmt.js";

export type LegendSwatchKind = "swatch" | "line" | "marker" | "lineMarker" | "verticalBar";

function legendKindFor(chart: Chart | undefined, s: ChartSeries): LegendSwatchKind {
  if (!chart) return "swatch";

  const kind = s.chartType ?? chart.type;

  const noMarker = s.markerSymbol === "none";
  if (kind === "line") return noMarker ? "line" : "lineMarker";
  if (kind === "stock") {
    return noMarker ? "verticalBar" : "marker";
  }
  if (kind === "scatter") {
    const style = chart.scatterStyle;
    if (style === "line" || style === "smooth") return "line";
    if (style === "lineMarker" || style === "smoothMarker") {
      return noMarker ? "line" : "lineMarker";
    }
    return noMarker ? "swatch" : "marker";
  }
  return "swatch";
}

function paintLegendSwatch(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  w: number,
  color: string,
  kind: LegendSwatchKind,
): void {
  ctx.fillStyle = color;
  if (kind === "swatch") {
    ctx.fillRect(x, y - w / 2, w, w);
    return;
  }
  if (kind === "marker") {
    ctx.beginPath();
    ctx.arc(x + w / 2, y, 3, 0, Math.PI * 2);
    ctx.fill();
    return;
  }
  if (kind === "verticalBar") {
    const prevStroke = ctx.strokeStyle;
    const prevWidth = ctx.lineWidth;
    ctx.strokeStyle = "#262626";
    ctx.lineWidth = 1.5;
    ctx.beginPath();
    ctx.moveTo(x + w / 2, y - w / 2 + 1);
    ctx.lineTo(x + w / 2, y + w / 2 - 1);
    ctx.stroke();
    ctx.strokeStyle = prevStroke;
    ctx.lineWidth = prevWidth;
    return;
  }

  const prevStroke = ctx.strokeStyle;
  const prevWidth = ctx.lineWidth;
  ctx.strokeStyle = color;
  ctx.lineWidth = 2;
  ctx.beginPath();
  ctx.moveTo(x, y);
  ctx.lineTo(x + w, y);
  ctx.stroke();
  ctx.strokeStyle = prevStroke;
  ctx.lineWidth = prevWidth;
  if (kind === "lineMarker") {
    ctx.beginPath();
    ctx.arc(x + w / 2, y, 3, 0, Math.PI * 2);
    ctx.fill();
  }
}

export function drawLegend(
  ctx: CanvasRenderingContext2D,
  series: ChartSeries[],
  rect: Rect,
  orientation: "horizontal" | "vertical" = "horizontal",
  chart?: Chart,
): void {
  drawStyleBox(ctx, rect, chart?.legendFill, chart?.legendBorder);
  const font = chart?.legendFont;
  const labelColor = font?.color ?? AXIS_LABEL_COLOR;
  ctx.font = chartFontCss(font, LEGEND_FONT_SIZE);
  ctx.textBaseline = "middle";
  const swatchW = 10;
  if (orientation === "vertical") {
    const lineH = (font?.sizePt ?? LEGEND_FONT_SIZE) + 6;
    const totalH = series.length * lineH;
    let y = rect.y + Math.max(0, (rect.h - totalH) / 2) + lineH / 2;
    const x = rect.x;
    for (let i = 0; i < series.length; i++) {
      const s = series[i]!;
      paintLegendSwatch(ctx, x, y, swatchW, s.color ?? "#4472C4", legendKindFor(chart, s));
      ctx.fillStyle = labelColor;
      ctx.textAlign = "left";
      ctx.fillText(s.name || `Series ${i + 1}`, x + swatchW + 4, y);
      y += lineH;
    }
    return;
  }
  const itemPad = 16;

  const widths = series.map((s) => swatchW + 6 + ctx.measureText(s.name || "").width);
  const totalW = widths.reduce((a, b) => a + b, 0) + itemPad * (series.length - 1);
  let x = rect.x + (rect.w - totalW) / 2;
  const y = rect.y + rect.h / 2;
  for (let i = 0; i < series.length; i++) {
    const s = series[i]!;
    paintLegendSwatch(ctx, x, y, swatchW, s.color ?? "#4472C4", legendKindFor(chart, s));
    ctx.fillStyle = labelColor;
    ctx.textAlign = "left";
    ctx.fillText(s.name || `Series ${i + 1}`, x + swatchW + 4, y);
    x += widths[i]! + itemPad;
  }
}

export function measureVerticalLegendWidth(
  ctx: CanvasRenderingContext2D,
  series: ChartSeries[],
): number {
  if (series.length === 0) return 0;
  ctx.font = `${LEGEND_FONT_SIZE}px -apple-system, "Helvetica Neue", Arial, sans-serif`;
  const swatchW = 10;
  let maxLabel = 0;
  for (const s of series) {
    const w = ctx.measureText(s.name || "").width;
    if (w > maxLabel) maxLabel = w;
  }
  return Math.ceil(swatchW + 4 + maxLabel + 4);
}

export function drawPlaceholderPlot(ctx: CanvasRenderingContext2D, chart: Chart, rect: Rect): void {
  ctx.fillStyle = "#f4f4f5";
  ctx.fillRect(rect.x, rect.y, rect.w, rect.h);
  ctx.fillStyle = AXIS_LABEL_COLOR;
  ctx.font = `12px -apple-system, "Helvetica Neue", Arial, sans-serif`;
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  const label = `${chart.type} chart (renderer v0 stub)`;
  ctx.fillText(label, rect.x + rect.w / 2, rect.y + rect.h / 2);
}

export function effectiveLabels(chart: Chart, s: ChartSeries): DataLabels | undefined {
  return s.dataLabels ?? chart.dataLabels;
}

export function pointLabel(
  base: DataLabels,
  i: number,
): { dl: DataLabels; text?: string } | null | undefined {
  const po = base.pointOverrides?.find((p) => p.idx === i);
  if (!po) return undefined;
  if (po.delete) return null;
  const merged: DataLabels = {
    showValue: po.showValue ?? base.showValue,
    showCategory: po.showCategory ?? base.showCategory,
    showSeriesName: po.showSeriesName ?? base.showSeriesName,
    showPercent: po.showPercent ?? base.showPercent,
    position: po.position ?? base.position,
    separator: base.separator,
    numFmt: po.numFmt ?? base.numFmt,
    pointOverrides: base.pointOverrides,
  };
  return { dl: merged, text: po.text };
}

export function buildLabelText(
  dl: DataLabels,
  chart: Chart,
  series: ChartSeries,
  categoryIdx: number,
  value: number,
  categoryTotal: number,
): string {
  const sep = dl.separator ?? ", ";
  const parts: string[] = [];
  if (dl.showSeriesName && series.name) parts.push(series.name);
  if (dl.showCategory) {
    const cats = chart.categories ?? [];
    const c = cats[categoryIdx];
    if (c != null && c !== "") parts.push(c);
  }

  if (dl.showPercent && categoryTotal > 0) {
    const pct = (value / categoryTotal) * 100;
    parts.push(`${Math.round(pct)}%`);
  }
  if (dl.showValue) {
    const fmt = dl.numFmt ?? chart.valueFormat;
    parts.push(formatAxisValue(value, fmt));
  }
  return parts.join(sep);
}

export function drawLabel(
  ctx: CanvasRenderingContext2D,
  text: string,
  x: number,
  y: number,
  align: CanvasTextAlign = "center",
  baseline: CanvasTextBaseline = "middle",
): void {
  if (!text) return;
  ctx.font = `${DATA_LABEL_FONT_SIZE}px -apple-system, "Helvetica Neue", Arial, sans-serif`;
  ctx.textAlign = align;
  ctx.textBaseline = baseline;

  ctx.lineWidth = 3;
  ctx.strokeStyle = "rgba(255,255,255,0.85)";
  ctx.lineJoin = "round";
  ctx.strokeText(text, x, y);
  ctx.lineWidth = 1;
  ctx.fillStyle = DATA_LABEL_COLOR;
  ctx.fillText(text, x, y);
}

export function resolveAxisRange(
  dataMin: number,
  dataMax: number,
  forcedMin: number | undefined,
  forcedMax: number | undefined,
  zeroClamp: boolean,
  tickCount: number,
  forcedMajorUnit?: number,
): { minV: number; maxV: number; ticks: number[] } {
  let lo = forcedMin ?? dataMin;
  let hi = forcedMax ?? dataMax;
  const userScaled =
    forcedMin !== undefined || forcedMax !== undefined || forcedMajorUnit !== undefined;
  if (zeroClamp && !userScaled) {
    if (lo > 0) lo = 0;
    if (hi < 0) hi = 0;
  }
  if (lo === hi) hi = lo + 1;
  const EPS = 1e-9;

  let t: number[];
  if (
    forcedMajorUnit !== undefined &&
    Number.isFinite(forcedMajorUnit) &&
    forcedMajorUnit > 0 &&
    (hi - lo) / forcedMajorUnit < 1000
  ) {
    const step = forcedMajorUnit;

    const anchor = forcedMin !== undefined ? forcedMin : 0;

    let niceMin: number;
    if (forcedMin !== undefined) {
      niceMin = forcedMin;
    } else {
      const floorAtData = anchor + Math.floor((lo - anchor) / step + EPS) * step;
      const floorAtZero = anchor + Math.min(0, Math.floor((lo - anchor) / step + EPS)) * step;
      const tentativeMax = anchor + Math.ceil((hi - anchor) / step - EPS) * step;
      const tickCountToZero = Math.round((tentativeMax - floorAtZero) / step) + 1;
      niceMin = tickCountToZero <= 14 ? floorAtZero : floorAtData;
    }
    const niceMax = anchor + Math.ceil((hi - anchor) / step - EPS) * step;
    t = [];
    for (let v = niceMin; v <= niceMax + step / 2; v += step) {
      t.push(parseFloat(v.toPrecision(12)));
    }
    if (t.length === 0) t.push(lo, hi);
  } else {
    t = niceTicks(lo, hi, tickCount);
  }
  if (forcedMin !== undefined) {
    t = t.filter((v) => v >= forcedMin - EPS);
    if (t.length === 0 || t[0]! > forcedMin + EPS) t.unshift(forcedMin);
  }
  if (forcedMax !== undefined) {
    t = t.filter((v) => v <= forcedMax + EPS);
    if (t.length === 0 || t[t.length - 1]! < forcedMax - EPS) t.push(forcedMax);
  }
  return { minV: t[0]!, maxV: t[t.length - 1]!, ticks: t };
}

export function niceTicks(min: number, max: number, count: number): number[] {
  if (max === min) {
    max = min + 1;
  }
  const range = niceNum(max - min, false);
  const step = niceNum(range / Math.max(1, count - 1), true);
  const niceMin = Math.floor(min / step) * step;
  const niceMax = Math.ceil(max / step) * step;
  const out: number[] = [];
  for (let v = niceMin; v <= niceMax + step / 2; v += step) {
    out.push(parseFloat(v.toPrecision(12)));
  }
  return out;
}

export function niceNum(range: number, round: boolean): number {
  const exp = Math.floor(Math.log10(Math.max(1e-12, Math.abs(range))));
  const f = range / Math.pow(10, exp);
  let nf: number;
  if (round) {
    if (f < 1.5) nf = 1;
    else if (f < 3) nf = 2;
    else if (f < 7) nf = 5;
    else nf = 10;
  } else {
    if (f <= 1) nf = 1;
    else if (f <= 2) nf = 2;
    else if (f <= 5) nf = 5;
    else nf = 10;
  }
  return nf * Math.pow(10, exp);
}

export function formatAxisValue(
  v: number,
  fmt: string | undefined,
  divisor?: number | null,
): string {
  if (divisor != null && Number.isFinite(divisor) && divisor > 0) {
    v = v / divisor;
  }
  if (!fmt || fmt === "General") return formatGeneral(v);
  const stripped = fmt.replace(/\[[^\]]*\]/g, "");
  const section = stripped.split(";")[0] ?? stripped;
  const decimals = decimalsIn(section);
  if (section.includes("%")) return (v * 100).toFixed(decimals) + "%";
  if (section.includes("$")) {
    const grouped = section.includes(",") || section.includes("#,##");
    return "$" + (grouped ? withGrouping(v, decimals) : v.toFixed(decimals));
  }
  if (section.includes(",")) return withGrouping(v, decimals);
  if (section.includes("0") || section.includes("#")) return v.toFixed(decimals);
  return formatGeneral(v);
}
export function formatGeneral(v: number): string {
  if (Number.isInteger(v) && Math.abs(v) < 1e15) return v.toString();
  return parseFloat(v.toPrecision(8)).toString();
}
export function decimalsIn(fmt: string): number {
  const i = fmt.indexOf(".");
  if (i < 0) return 0;
  let n = 0;
  for (let j = i + 1; j < fmt.length; j++) {
    const ch = fmt[j];
    if (ch === "0" || ch === "#") n++;
    else break;
  }
  return n;
}
export function withGrouping(v: number, decimals: number): string {
  const neg = v < 0;
  const abs = Math.abs(v).toFixed(decimals);
  const [intPart, frac] = abs.split(".");
  const grouped = (intPart ?? "0").replace(/\B(?=(\d{3})+(?!\d))/g, ",");
  return (neg ? "-" : "") + grouped + (frac ? "." + frac : "");
}
