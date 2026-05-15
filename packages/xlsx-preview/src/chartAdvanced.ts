import type { Chart, ChartSeries } from "./types.js";
import type { Rect } from "./chart.js";
import { activeThemeColor } from "./color.js";
import {
  buildLabelText,
  drawAxisFrame,
  drawLabel,
  drawPlaceholderPlot,
  effectiveLabels,
  formatGeneral,
  niceTicks,
  paintZeroBaseline,
  pointLabel,
  withAlpha,
} from "./chartUtils.js";

const AXIS_FONT_SIZE = 10;
const AXIS_LABEL_COLOR = "#52525b";
const AXIS_TICK_COUNT = 5;

// ---------- pie / doughnut ----------

const DEFAULT_PIE_ACCENTS = ["#4472C4", "#ED7D31", "#A5A5A5", "#FFC000", "#5B9BD5", "#70AD47"];

export function pieSliceColor(index: number, pointColors: readonly (string | undefined)[]): string {
  const explicit = pointColors[index];
  // `"none"` is the schema sentinel for `<c:dPt><c:spPr><a:noFill/>`
  // (explicit transparent). Pie/doughnut treat it as "fall back to
  // palette" — a transparent slice in a pie is nonsensical, and the
  // AGS corpus only exercises it on stacked columns. The bar painter
  // below has the real branch.
  if (explicit && explicit.length > 0 && explicit !== "none") return explicit;
  const accentIndex = 4 + (index % 6);
  return activeThemeColor(accentIndex, DEFAULT_PIE_ACCENTS[index % DEFAULT_PIE_ACCENTS.length]!);
}

/**
 * Resolve the per-bar fill for a series at category `i`, honoring
 * `<c:dPt>` overrides (parity-charts.md Bug #3 follow-up). Returns:
 * - `{ skip: true }` when the source XML carries
 *   `<c:dPt><c:spPr><a:noFill/></c:spPr>` for this index — the
 *   caller must not paint geometry OR data labels for this point,
 *   but should still advance any stacked-axis offset (the
 *   transparent point still occupies its slot in the stack so the
 *   next series floats above it; that's the waterfall-bar idiom
 *   AGS Chart_Chart_2 uses).
 * - `{ skip: false, color }` otherwise, with `color` resolved as:
 *   explicit dPt hex → series color → fallback `#4472C4`.
 */
export function resolveBarFill(
  s: ChartSeries,
  i: number,
): { skip: true } | { skip: false; color: string } {
  const override = s.pointColors?.[i];
  if (override === "none") return { skip: true };
  if (override && override.length > 0) return { skip: false, color: override };
  return { skip: false, color: s.color ?? "#4472C4" };
}

export function drawPieChart(ctx: CanvasRenderingContext2D, chart: Chart, rect: Rect): void {
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
  // them as `series.pointColors[i]`); otherwise we cycle the workbook
  // theme accents (theme indexes 4..9).
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
    ctx.fillStyle = pieSliceColor(i, pointColors);
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
  // Per-slice <c:dLbl> overrides can change position / show* / numFmt /
  // literal text for individual slices (common in Excel pies that label
  // only one or two slices).
  const dl = effectiveLabels(chart, ser);
  if (dl) {
    for (const sl of slices) {
      const po = pointLabel(dl, sl.idx);
      if (po === null) continue;
      const edl = po?.dl ?? dl;
      const pos = edl.position ?? "outEnd";
      const labelR =
        pos === "outEnd" || pos === "bestFit" ? r + 12 : pos === "ctr" ? (innerR + r) / 2 : r - 12; // inEnd
      const text = po?.text ?? buildLabelText(edl, chart, ser, sl.idx, sl.v, total);
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

export function drawScatterChart(ctx: CanvasRenderingContext2D, chart: Chart, rect: Rect): void {
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
      const c = (chart.categories ?? [])[i];
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
  const drawMarkers =
    style == null || style === "marker" || style === "lineMarker" || style === "smoothMarker";

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

    // Markers + per-point labels. `<c:marker><c:symbol val="none"/>`
    // overrides the chart-level scatter style for this series.
    const noMarker = s.markerSymbol === "none";
    for (const p of pts) {
      if (drawMarkers && !noMarker) {
        ctx.beginPath();
        ctx.arc(p.x, p.y, 3.5, 0, Math.PI * 2);
        ctx.fill();
      }
      if (dl) {
        const po = pointLabel(dl, p.i);
        if (po !== null) {
          const edl = po?.dl ?? dl;
          const text = po?.text ?? buildLabelText(edl, chart, s, p.i, p.v, 0);
          if (text) drawLabel(ctx, text, p.x, p.y - 6, "center", "bottom");
        }
      }
    }
  }
  // Bug #13 step 1: heavier zero baseline when the y-axis straddles zero.
  paintZeroBaseline(ctx, inner, yMin, yMax);
}

// ---------- bubble ----------
//
// Bubble charts are scatter plots whose marker radius is driven by a
// third value (`<c:bubbleSize>`, ECMA-376 §21.2.2.30). The largest
// bubble's diameter is capped at a fraction of the plot's smaller
// dimension; everything else scales relative to that. Two sizing
// modes per `<c:sizeRepresents>` (§21.2.2.197): `area` (default,
// bubble area is proportional to the value) and `w` (bubble width
// is proportional to the value). `<c:bubbleScale val="N"/>` (0..=300,
// default 100) is a final multiplier on the cap.
//
// Negative bubble sizes are skipped (matches Excel's default; the
// `<c:showNegBubbles>` toggle for hatch-rendered negative bubbles is
// not yet honored).

export function drawBubbleChart(ctx: CanvasRenderingContext2D, chart: Chart, rect: Rect): void {
  const series = chart.series.filter((s) => s.values.length > 0);
  if (series.length === 0) {
    drawPlaceholderPlot(ctx, chart, rect);
    return;
  }

  // X data: per-series `xValues`, else 1..N indices.
  const xCache: number[][] = series.map((s) => {
    const xs = (s.xValues ?? []) as number[];
    if (xs.length > 0) return xs.slice();
    return s.values.map((_, i) => i + 1);
  });
  // Bubble-size data: per-series `bubbleSizes`. Falls back to a
  // constant 1 if the source workbook authored a bubble chart with
  // no size data (rare but well-defined — Excel renders identical-
  // sized bubbles).
  const sCache: number[][] = series.map((s) => {
    const bs = (s.bubbleSizes ?? []) as number[];
    if (bs.length > 0) return bs.slice();
    return s.values.map(() => 1);
  });

  let xMin = Infinity,
    xMax = -Infinity;
  let yMin = Infinity,
    yMax = -Infinity;
  let maxSize = -Infinity;
  for (let si = 0; si < series.length; si++) {
    const xs = xCache[si]!;
    const ys = series[si]!.values;
    const sz = sCache[si]!;
    const n = Math.min(xs.length, ys.length);
    for (let i = 0; i < n; i++) {
      const x = xs[i]!,
        y = ys[i]!;
      if (x < xMin) xMin = x;
      if (x > xMax) xMax = x;
      if (y < yMin) yMin = y;
      if (y > yMax) yMax = y;
      const s = sz[i] ?? 0;
      if (Number.isFinite(s) && s > maxSize) maxSize = s;
    }
  }
  if (!Number.isFinite(xMin) || !Number.isFinite(yMin)) {
    drawPlaceholderPlot(ctx, chart, rect);
    return;
  }
  if (xMin === xMax) xMax = xMin + 1;
  if (yMin === yMax) yMax = yMin + 1;
  if (!Number.isFinite(maxSize) || maxSize <= 0) maxSize = 1;
  const xTicks = niceTicks(xMin, xMax, AXIS_TICK_COUNT);
  const yTicks = niceTicks(yMin, yMax, AXIS_TICK_COUNT);
  xMin = xTicks[0]!;
  xMax = xTicks[xTicks.length - 1]!;
  yMin = yTicks[0]!;
  yMax = yTicks[yTicks.length - 1]!;

  const inner = drawAxisFrame(ctx, chart, rect, yTicks, yMin, yMax, /*horizontal=*/ false, false);

  // Numeric x-axis labels.
  ctx.font = `${AXIS_FONT_SIZE}px -apple-system, "Helvetica Neue", Arial, sans-serif`;
  ctx.fillStyle = AXIS_LABEL_COLOR;
  ctx.textAlign = "center";
  ctx.textBaseline = "top";
  for (const t of xTicks) {
    const frac = (t - xMin) / (xMax - xMin);
    const x = inner.x + frac * inner.w;
    ctx.fillText(formatGeneral(t), x, inner.y + inner.h + 4);
  }

  // Max bubble *diameter* (not radius) is ~22% of the plot's smaller
  // dimension on Excel desktop / hsx; multiplied by `bubbleScale`/100.
  // Working backwards from a paired observation (hsx renders the
  // largest size=85 bubble at ~50px diameter inside a ~225px plot
  // → 0.22 ratio); we land at the same cap.
  const scalePct = chart.bubbleScale ?? 100;
  const baseR = Math.min(inner.w, inner.h) * 0.11 * (scalePct / 100);
  // Minimum visible radius so a tiny non-zero bubble doesn't vanish.
  const minR = 2;
  // `area` (default): r = baseR * sqrt(size / maxSize)
  // `w`            : r = baseR *      (size / maxSize)
  const byArea = (chart.sizeRepresents ?? "area") !== "w";

  for (let si = 0; si < series.length; si++) {
    const s = series[si]!;
    const xs = xCache[si]!;
    const ys = s.values;
    const sz = sCache[si]!;
    const n = Math.min(xs.length, ys.length);
    if (n === 0) continue;
    const color = s.color ?? "#4472C4";
    const dl = effectiveLabels(chart, s);

    for (let i = 0; i < n; i++) {
      const xv = xs[i]!,
        yv = ys[i]!;
      const bs = sz[i] ?? 0;
      // Negative / zero bubbles drop (Excel default; `showNegBubbles`
      // would hatch-fill them, deferred).
      if (!Number.isFinite(bs) || bs <= 0) continue;
      const frac = bs / maxSize;
      const r = Math.max(minR, baseR * (byArea ? Math.sqrt(frac) : frac));
      const px = inner.x + ((xv - xMin) / (xMax - xMin)) * inner.w;
      const py = inner.y + (1 - (yv - yMin) / (yMax - yMin)) * inner.h;

      ctx.beginPath();
      ctx.arc(px, py, r, 0, Math.PI * 2);
      // Translucent fill so overlapping bubbles read; matches Excel's
      // default bubble appearance.
      ctx.fillStyle = withAlpha(color, 0.6);
      ctx.fill();
      ctx.strokeStyle = color;
      ctx.lineWidth = 1;
      ctx.stroke();

      if (dl) {
        const po = pointLabel(dl, i);
        if (po !== null) {
          const edl = po?.dl ?? dl;
          const text = po?.text ?? buildLabelText(edl, chart, s, i, yv, 0);
          if (text) drawLabel(ctx, text, px, py, "center", "middle");
        }
      }
    }
  }
  paintZeroBaseline(ctx, inner, yMin, yMax);
}

export { drawComboChart } from "./chartCombo.js";
