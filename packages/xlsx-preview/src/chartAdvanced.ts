import type { Chart, ChartSeries } from "./types.js";
import type { Rect } from "./chart.js";
import { activeThemeColor } from "./color.js";
import {
  buildLabelText,
  drawAxisFrame,
  drawCategoryAxis,
  drawLabel,
  drawPlaceholderPlot,
  effectiveLabels,
  formatAxisValue,
  formatGeneral,
  niceTicks,
  paintZeroBaseline,
  pointLabel,
  resolveAxisRange,
  valueRange,
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

// ---------- radar ----------

/**
 * Polar/spider chart. ECMA-376 §21.2.2.155 / §21.2.2.176.
 *
 * Layout: one spoke per category, evenly spaced around a center.
 * Spoke 0 points up (angle = -π/2 in canvas coords) and they advance
 * clockwise — matches Excel desktop and the radar fixtures in the
 * AGS corpus. Each series traces a closed polygon whose vertex on
 * spoke `i` sits at distance `(v[i] - minV) / (maxV - minV) * R` from
 * the center.
 *
 * Gridlines are concentric *polygons* (straight segments between
 * spokes at each tick level), not circles — matches Excel. The
 * radar variants honor `chart.radarStyle`:
 *   - `standard`: stroked polygon only.
 *   - `marker`:   stroked polygon + circle markers (Excel UI default).
 *   - `filled`:   semi-transparent filled polygon + stroked outline.
 */
export function drawRadarChart(ctx: CanvasRenderingContext2D, chart: Chart, rect: Rect): void {
  const series = chart.series.filter((s) => s.values.length > 0);
  if (series.length === 0) {
    drawPlaceholderPlot(ctx, chart, rect);
    return;
  }
  const cats = chart.categories ?? [];
  const categoryCount = Math.max(...series.map((s) => s.values.length), cats.length);
  if (categoryCount < 3) {
    // Radar needs >=3 spokes to make sense as a polygon. Two or
    // fewer categories collapse to a line/point — fall back to
    // the placeholder rather than emit something misleading.
    drawPlaceholderPlot(ctx, chart, rect);
    return;
  }

  // Padding for category labels around the perimeter. We measure
  // every label so the longest one drives the inset on its side;
  // simple cardinal-only inset is good enough for v0.
  ctx.font = `${AXIS_FONT_SIZE}px -apple-system, "Helvetica Neue", Arial, sans-serif`;
  let maxLabelW = 0;
  for (let i = 0; i < categoryCount; i++) {
    const t = cats[i] ?? `${i + 1}`;
    maxLabelW = Math.max(maxLabelW, ctx.measureText(t).width);
  }
  const labelPad = 8;
  const inset = Math.min(rect.w, rect.h) * 0.08 + Math.max(maxLabelW / 2, AXIS_FONT_SIZE);
  const cx = rect.x + rect.w / 2;
  const cy = rect.y + rect.h / 2;
  const R = Math.max(20, Math.min(rect.w, rect.h) / 2 - inset);

  // Data range. Radar doesn't zero-clamp — mirror line-chart
  // behavior so a band of values like 50..90 doesn't waste 80% of
  // the radius on empty axis. `<c:scaling>` bounds still win.
  const rows = series.map((s) => Array.from({ length: categoryCount }, (_, i) => s.values[i] ?? 0));
  let { minV, maxV } = valueRange(rows);
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

  // Angle for spoke i (0 = up, clockwise).
  const angleFor = (i: number) => -Math.PI / 2 + (i / categoryCount) * Math.PI * 2;
  const radiusFor = (v: number) => {
    if (!Number.isFinite(v)) return 0;
    const span = maxV - minV;
    if (span <= 0) return 0;
    return Math.max(0, ((v - minV) / span) * R);
  };

  // 1. Spokes (radial axes).
  ctx.strokeStyle = "#e5e7eb";
  ctx.lineWidth = 1;
  for (let i = 0; i < categoryCount; i++) {
    const a = angleFor(i);
    ctx.beginPath();
    ctx.moveTo(cx, cy);
    ctx.lineTo(cx + Math.cos(a) * R, cy + Math.sin(a) * R);
    ctx.stroke();
  }

  // 2. Concentric gridline polygons, one per tick. Skip the
  // center tick (it's a point). Outermost tick traces the perimeter.
  ctx.strokeStyle = "#e5e7eb";
  for (const t of ticks) {
    const r = radiusFor(t);
    if (r <= 0.5) continue;
    ctx.beginPath();
    for (let i = 0; i < categoryCount; i++) {
      const a = angleFor(i);
      const x = cx + Math.cos(a) * r;
      const y = cy + Math.sin(a) * r;
      if (i === 0) ctx.moveTo(x, y);
      else ctx.lineTo(x, y);
    }
    ctx.closePath();
    ctx.stroke();
  }

  // 3. Category labels just outside each spoke endpoint.
  ctx.fillStyle = AXIS_LABEL_COLOR;
  ctx.font = `${AXIS_FONT_SIZE}px -apple-system, "Helvetica Neue", Arial, sans-serif`;
  for (let i = 0; i < categoryCount; i++) {
    const a = angleFor(i);
    const lx = cx + Math.cos(a) * (R + labelPad);
    const ly = cy + Math.sin(a) * (R + labelPad);
    // Alignment based on which side of the center the spoke ends on.
    // Tolerance accounts for spokes that are nearly vertical/horizontal.
    const TOL = 0.05;
    let align: CanvasTextAlign = "center";
    if (Math.cos(a) > TOL) align = "left";
    else if (Math.cos(a) < -TOL) align = "right";
    let baseline: CanvasTextBaseline = "middle";
    if (Math.sin(a) < -TOL) baseline = "bottom";
    else if (Math.sin(a) > TOL) baseline = "top";
    ctx.textAlign = align;
    ctx.textBaseline = baseline;
    ctx.fillText(cats[i] ?? `${i + 1}`, lx, ly);
  }

  // 4. Tick labels along the top spoke (angle = -π/2). Skip the
  // bottom tick (== minV) since its position == center for the
  // common non-zero-clamped case and labels would overlap.
  ctx.fillStyle = AXIS_LABEL_COLOR;
  ctx.textAlign = "right";
  ctx.textBaseline = "middle";
  for (let ti = 0; ti < ticks.length; ti++) {
    if (ti === 0) continue;
    const t = ticks[ti]!;
    const r = radiusFor(t);
    if (r <= 0.5) continue;
    const text = formatAxisValue(t, chart.valueFormat, chart.dispUnits);
    ctx.fillText(text, cx - 3, cy - r);
  }

  // 5. Series polygons. `filled` style gets a semi-transparent fill,
  // `marker` (Excel UI default) and `standard` are stroke-only.
  const filled = chart.radarStyle === "filled";
  const showMarkers = chart.radarStyle !== "standard";
  for (let si = 0; si < series.length; si++) {
    const s = series[si]!;
    const color = s.color ?? "#4472C4";
    const data = rows[si]!;
    // Per-point gap handling: if any vertex is non-finite we draw
    // an open polyline instead of a closed polygon — a gap in the
    // radar series. Rare but matches Excel's `dispBlanksAs=gap`.
    const allFinite = data.every((v) => Number.isFinite(v));

    ctx.beginPath();
    let started = false;
    for (let i = 0; i < categoryCount; i++) {
      const v = data[i] ?? 0;
      if (!Number.isFinite(v)) {
        started = false;
        continue;
      }
      const a = angleFor(i);
      const r = radiusFor(v);
      const x = cx + Math.cos(a) * r;
      const y = cy + Math.sin(a) * r;
      if (!started) {
        ctx.moveTo(x, y);
        started = true;
      } else {
        ctx.lineTo(x, y);
      }
    }
    if (allFinite) ctx.closePath();

    if (filled) {
      ctx.fillStyle = withAlpha(color, 0.45);
      ctx.fill();
    }
    ctx.strokeStyle = color;
    ctx.lineWidth = filled ? 1.25 : 2;
    ctx.stroke();

    // Markers. `radarStyle="standard"` suppresses them; explicit
    // per-series `<c:marker><c:symbol val="none"/>` also wins.
    if (showMarkers && s.markerSymbol !== "none") {
      ctx.fillStyle = color;
      for (let i = 0; i < categoryCount; i++) {
        const v = data[i] ?? 0;
        if (!Number.isFinite(v)) continue;
        const a = angleFor(i);
        const r = radiusFor(v);
        ctx.beginPath();
        ctx.arc(cx + Math.cos(a) * r, cy + Math.sin(a) * r, 3, 0, Math.PI * 2);
        ctx.fill();
      }
    }

    // Data labels (default position `t` — above the marker, away
    // from the center).
    const dl = effectiveLabels(chart, s);
    if (dl) {
      const PAD = 6;
      for (let i = 0; i < categoryCount; i++) {
        const v = s.values[i];
        if (v == null || !Number.isFinite(v)) continue;
        const po = pointLabel(dl, i);
        if (po === null) continue;
        const edl = po?.dl ?? dl;
        const text = po?.text ?? buildLabelText(edl, chart, s, i, v, 0);
        if (!text) continue;
        const a = angleFor(i);
        const r = radiusFor(v);
        // Push the label radially outward from the marker.
        const lx = cx + Math.cos(a) * (r + PAD);
        const ly = cy + Math.sin(a) * (r + PAD);
        const TOL = 0.05;
        let align: CanvasTextAlign = "center";
        if (Math.cos(a) > TOL) align = "left";
        else if (Math.cos(a) < -TOL) align = "right";
        // Above the center → label above marker; below → below.
        let baseline: CanvasTextBaseline = "middle";
        if (Math.sin(a) < -TOL) baseline = "bottom";
        else if (Math.sin(a) > TOL) baseline = "top";
        drawLabel(ctx, text, lx, ly, align, baseline);
      }
    }
  }
  // Silence unused-import lint when drawAxisFrame/niceTicks/formatGeneral/
  // paintZeroBaseline are unused here. (kept for parity w/ other painters)
  void drawAxisFrame;
  void niceTicks;
  void formatGeneral;
  void paintZeroBaseline;
}

// ---------- stock ----------

/**
 * Stock chart painter (ECMA-376 §21.2.2.207). Series count implies
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
  void drawCategoryAxis;
  void niceTicks;
}

export { drawComboChart } from "./chartCombo.js";
