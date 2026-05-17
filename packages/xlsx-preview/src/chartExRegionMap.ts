// chartEx (`cx:`) regionMap painter — Excel's "Filled Map" chart.
//
// Excel authors choose a Bing-backed geographic projection for region
// maps and ship the resolved polygons inside the workbook as opaque
// `<cx:binary>` geoCache blobs (gzipped Bing-proprietary polygon
// streams that our extractor doesn't decode). To paint the chart we
// bring our own world geometry: Natural Earth 110m admin_0 countries,
// slimmed and 2-decimal-rounded into
// `packages/xlsx-preview/src/world110m.ts`.
//
// Behaviour:
//   - Equirectangular projection of the visible plot rect, lat
//     clipped to roughly Mercator-friendly bounds so Antarctica
//     doesn't dominate the bottom strip.
//   - Country name normalisation maps each `<cx:strDim type="cat">`
//     label to a single Natural Earth feature. Two-letter ISO codes
//     are checked too so abbreviated workbooks (e.g. "US", "GB")
//     still resolve.
//   - Two-stop linear color scale: minimum → near-white; maximum →
//     theme accent1. Workbooks that author a `<cx:valueColors>` 2-
//     or 3-stop palette will use Excel defaults until we wire that
//     through the extractor; the current heuristic matches Excel's
//     out-of-the-box "Sequential" presentation.
//   - Unmatched countries paint a soft neutral gray so the map still
//     reads as a world frame.
//   - Gradient legend bar on the right edge of the plot rect.
//
// To regenerate the geometry table after a Natural Earth release:
//   curl -sL https://raw.githubusercontent.com/nvkelso/natural-earth-vector/master/geojson/ne_110m_admin_0_countries.geojson -o /tmp/ne.json
//   python3 - <<'PY'
//   import json
//   d = json.load(open('/tmp/ne.json'))
//   def r(c):
//       if isinstance(c, list):
//           if c and isinstance(c[0], (int, float)):
//               return [round(c[0],2), round(c[1],2)]
//           return [r(x) for x in c]
//       return c
//   feats=[{'n':f['properties'].get('NAME'),
//           'nl':f['properties'].get('NAME_LONG'),
//           'a2':f['properties'].get('ISO_A2'),
//           'a3':f['properties'].get('ISO_A3'),
//           'g':{'type':f['geometry']['type'],
//                'coordinates':r(f['geometry']['coordinates'])}}
//          for f in d['features'] if f.get('geometry')]
//   PY

import type { Chart } from "./types.js";
import type { Rect } from "./chart.js";
import { activeThemeColor } from "./color.js";
import { drawPlaceholderPlot, formatAxisValue, formatGeneral } from "./chartUtils.js";
import { WORLD_110M_FEATURES, type World110mFeature } from "./world110m.js";

const LABEL_FONT_SIZE = 10;
const LABEL_COLOR = "#52525b";
const UNMATCHED_FILL = "#e5e7eb";
const COUNTRY_STROKE = "#ffffff";
const COUNTRY_STROKE_WIDTH = 0.5;

// Lat clamp: Natural Earth ranges full -90..90, but populated areas
// only stretch to ~83°N (northern Greenland) and Antarctica below
// -60° is mostly unused. Clamp to [-58, 84] so the bulk of land mass
// fills the rect instead of leaving big polar voids.
const LAT_MIN = -58;
const LAT_MAX = 84;
const LON_MIN = -180;
const LON_MAX = 180;

// ---------- name index ----------
//
// Maps every candidate label (lower-cased) to its `World110mFeature`.
// Built lazily on first chart paint, then memoized.
let NAME_INDEX: Map<string, World110mFeature> | null = null;

// Hand-curated aliases for the common Excel/workbook labels that
// don't match any Natural Earth NAME / NAME_LONG / ISO code variant.
// Add sparingly — Natural Earth's NAME column already covers ~95%
// of mainstream workbook spellings.
const NAME_ALIASES: Record<string, string> = {
  usa: "united states of america",
  "u.s.a.": "united states of america",
  "u.s.": "united states of america",
  "united states": "united states of america",
  america: "united states of america",
  uk: "united kingdom",
  "u.k.": "united kingdom",
  "great britain": "united kingdom",
  britain: "united kingdom",
  uae: "united arab emirates",
  korea: "south korea",
  "south korea": "south korea",
  "republic of korea": "south korea",
  "north korea": "north korea",
  "dprk": "north korea",
  russia: "russia",
  "russian federation": "russia",
  czechia: "czech republic",
  "czech republic": "czech republic",
  "ivory coast": "ivory coast", // NE uses "Ivory Coast" verbatim
  "côte d'ivoire": "ivory coast",
  "viet nam": "vietnam",
  burma: "myanmar",
  laos: "laos",
  "lao pdr": "laos",
  congo: "democratic republic of the congo",
  drc: "democratic republic of the congo",
  "dr congo": "democratic republic of the congo",
  "republic of the congo": "republic of the congo",
  swaziland: "eswatini",
  cabo: "cape verde",
  "cabo verde": "cape verde",
  tanzania: "united republic of tanzania",
  syria: "syria",
};

function buildNameIndex(): Map<string, World110mFeature> {
  const idx = new Map<string, World110mFeature>();
  const put = (key: string | null | undefined, f: World110mFeature) => {
    if (!key) return;
    const k = key.trim().toLowerCase();
    if (!k || k === "-99") return;
    if (!idx.has(k)) idx.set(k, f);
  };
  for (const f of WORLD_110M_FEATURES) {
    put(f.n, f);
    put(f.nl, f);
    put(f.a2, f);
    put(f.a3, f);
  }
  // Apply aliases. Each alias points at a canonical lower-cased key
  // already in the index; if the canonical key is missing (e.g. a
  // future Natural Earth release renamed it) we skip the alias.
  for (const [alias, canon] of Object.entries(NAME_ALIASES)) {
    const target = idx.get(canon);
    if (target && !idx.has(alias)) idx.set(alias, target);
  }
  return idx;
}

function lookupCountry(label: string): World110mFeature | undefined {
  if (!NAME_INDEX) NAME_INDEX = buildNameIndex();
  const k = label.trim().toLowerCase();
  return NAME_INDEX.get(k);
}

// ---------- projection ----------

interface Projection {
  fwd: (lon: number, lat: number) => [number, number];
}

function makeEquirectangular(rect: Rect): Projection {
  const w = rect.w;
  const h = rect.h;
  const sx = w / (LON_MAX - LON_MIN);
  const sy = h / (LAT_MAX - LAT_MIN);
  // Preserve 1:1 lon/lat aspect so countries don't squash. Letterbox
  // by centring the world inside the plot rect.
  const s = Math.min(sx, sy);
  const worldW = (LON_MAX - LON_MIN) * s;
  const worldH = (LAT_MAX - LAT_MIN) * s;
  const ox = rect.x + (w - worldW) / 2;
  const oy = rect.y + (h - worldH) / 2;
  return {
    fwd: (lon, lat) => [ox + (lon - LON_MIN) * s, oy + (LAT_MAX - lat) * s],
  };
}

// ---------- color scale ----------

/** Parse `#rrggbb` to `[r, g, b]` (0..255). */
function hexToRgb(hex: string): [number, number, number] {
  const m = /^#?([0-9a-f]{6})$/i.exec(hex.trim());
  if (!m) return [68, 114, 196];
  const n = parseInt(m[1]!, 16);
  return [(n >> 16) & 0xff, (n >> 8) & 0xff, n & 0xff];
}

function rgbToHex(r: number, g: number, b: number): string {
  const c = (n: number) =>
    Math.max(0, Math.min(255, Math.round(n))).toString(16).padStart(2, "0");
  return `#${c(r)}${c(g)}${c(b)}`;
}

function lerpColor(a: [number, number, number], b: [number, number, number], t: number): string {
  return rgbToHex(a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t);
}

interface ColorScale {
  /** Map a raw data value to a CSS color (or undefined when NaN). */
  color: (v: number) => string;
  min: number;
  max: number;
  /** Color stops as `(t in [0,1], hex)` pairs in ascending t order;
   *  used to paint the gradient legend bar. */
  stops: { t: number; hex: string }[];
}

function buildColorScale(
  values: number[],
  authored: { min?: string; mid?: string; max?: string },
): ColorScale | null {
  const finite = values.filter((v) => Number.isFinite(v));
  if (finite.length === 0) return null;
  const min = Math.min(...finite);
  const max = Math.max(...finite);

  // Three palette flavors:
  //   1. authored 3-stop (min + mid + max)  → diverging
  //   2. authored 2-stop (min + max only)   → linear
  //   3. nothing authored                   → near-white → accent1
  //      (matches Excel's default "sequential" presentation).
  let stops: { t: number; rgb: [number, number, number]; hex: string }[];
  if (authored.min && authored.max && authored.mid) {
    stops = [
      stopAt(0, authored.min),
      stopAt(0.5, authored.mid),
      stopAt(1, authored.max),
    ];
  } else if (authored.min && authored.max) {
    stops = [stopAt(0, authored.min), stopAt(1, authored.max)];
  } else {
    const accentHex = activeThemeColor(4, "#4472C4");
    const accent = hexToRgb(accentHex);
    const low: [number, number, number] = [
      accent[0] + (255 - accent[0]) * 0.85,
      accent[1] + (255 - accent[1]) * 0.85,
      accent[2] + (255 - accent[2]) * 0.85,
    ];
    stops = [
      { t: 0, rgb: low, hex: rgbToHex(low[0], low[1], low[2]) },
      { t: 1, rgb: accent, hex: accentHex },
    ];
  }

  return {
    min,
    max,
    stops: stops.map((s) => ({ t: s.t, hex: s.hex })),
    color: (v) => {
      if (!Number.isFinite(v)) return UNMATCHED_FILL;
      const t = max === min ? 1 : (v - min) / (max - min);
      return lerpStops(stops, Math.max(0, Math.min(1, t)));
    },
  };
}

function stopAt(
  t: number,
  hex: string,
): { t: number; rgb: [number, number, number]; hex: string } {
  const rgb = hexToRgb(hex);
  return { t, rgb, hex };
}

/** Piecewise-linear interpolation across an arbitrary number of
 *  ascending-`t` color stops. Clamps to the endpoint stops outside
 *  `[stops[0].t, stops[n-1].t]`. */
function lerpStops(
  stops: { t: number; rgb: [number, number, number] }[],
  t: number,
): string {
  if (stops.length === 0) return UNMATCHED_FILL;
  if (t <= stops[0]!.t) return rgbToHex(...stops[0]!.rgb);
  for (let i = 1; i < stops.length; i++) {
    const a = stops[i - 1]!;
    const b = stops[i]!;
    if (t <= b.t) {
      const span = b.t - a.t;
      const localT = span === 0 ? 0 : (t - a.t) / span;
      return lerpColor(a.rgb, b.rgb, localT);
    }
  }
  return rgbToHex(...stops[stops.length - 1]!.rgb);
}

// ---------- path tracing ----------

function tracePolygon(
  ctx: CanvasRenderingContext2D,
  rings: number[][][],
  proj: Projection,
): void {
  for (const ring of rings) {
    if (ring.length < 2) continue;
    const first = ring[0]!;
    const [fx, fy] = proj.fwd(first[0]!, first[1]!);
    ctx.moveTo(fx, fy);
    for (let i = 1; i < ring.length; i++) {
      const p = ring[i]!;
      const [x, y] = proj.fwd(p[0]!, p[1]!);
      ctx.lineTo(x, y);
    }
    ctx.closePath();
  }
}

function traceFeature(
  ctx: CanvasRenderingContext2D,
  f: World110mFeature,
  proj: Projection,
): void {
  const g = f.g;
  if (g.type === "Polygon") {
    tracePolygon(ctx, g.coordinates, proj);
  } else {
    for (const poly of g.coordinates) tracePolygon(ctx, poly, proj);
  }
}

// ---------- main entry ----------

export function drawRegionMapChartEx(
  ctx: CanvasRenderingContext2D,
  chart: Chart,
  rect: Rect,
): void {
  const series = chart.series[0];
  if (!series || series.values.length === 0) {
    drawPlaceholderPlot(ctx, chart, rect);
    return;
  }

  const cats = chart.categories ?? [];
  const values = series.values;
  const scale = buildColorScale(values, {
    min: chart.cxRegionMapMinColor,
    mid: chart.cxRegionMapMidColor,
    max: chart.cxRegionMapMaxColor,
  });
  if (!scale) {
    drawPlaceholderPlot(ctx, chart, rect);
    return;
  }

  // Reserve a strip on the right for the gradient legend (only when
  // the chart actually has a `<cx:legend>` — chart.legendPos is the
  // signal). Width is fixed at 60px (swatch + label band).
  const showLegend = !!chart.legendPos && chart.legendPos !== "n";
  const LEGEND_W = 64;
  const LEGEND_PAD = 8;
  const mapRect: Rect = showLegend
    ? { ...rect, w: Math.max(40, rect.w - LEGEND_W - LEGEND_PAD) }
    : rect;

  const proj = makeEquirectangular(mapRect);

  // Build a quick lookup: matched feature -> data value. We rely on
  // object identity in the feature table for keys.
  const matched = new Map<World110mFeature, number>();
  for (let i = 0; i < cats.length; i++) {
    const label = cats[i];
    const v = values[i];
    if (!label || v == null || !Number.isFinite(v)) continue;
    const f = lookupCountry(label);
    if (f) matched.set(f, v);
  }

  // Paint base layer (unmatched countries) first, then matched on top.
  // Stroke each country with a thin white seam.
  ctx.save();
  ctx.lineJoin = "round";
  ctx.lineWidth = COUNTRY_STROKE_WIDTH;
  ctx.strokeStyle = COUNTRY_STROKE;
  for (const f of WORLD_110M_FEATURES) {
    if (matched.has(f)) continue;
    ctx.fillStyle = UNMATCHED_FILL;
    ctx.beginPath();
    traceFeature(ctx, f, proj);
    ctx.fill();
    ctx.stroke();
  }
  for (const [f, v] of matched) {
    ctx.fillStyle = scale.color(v);
    ctx.beginPath();
    traceFeature(ctx, f, proj);
    ctx.fill();
    ctx.stroke();
  }
  ctx.restore();

  // Gradient legend bar.
  if (showLegend) {
    drawGradientLegend(ctx, chart, rect, mapRect, scale, LEGEND_W, LEGEND_PAD);
  }
}

function drawGradientLegend(
  ctx: CanvasRenderingContext2D,
  chart: Chart,
  rect: Rect,
  mapRect: Rect,
  scale: ColorScale,
  legendW: number,
  pad: number,
): void {
  const barW = 14;
  const barX = mapRect.x + mapRect.w + pad;
  // Keep the legend bar within the vertical span of the map (looks
  // tidier than spanning the full rect when the map is letterboxed).
  const barH = Math.min(rect.h * 0.7, 240);
  const barY = rect.y + (rect.h - barH) / 2;

  // The bar paints bottom-to-top (low value at the bottom). Map each
  // scale stop's `t` into a gradient stop position; the gradient's 0
  // coordinate is `barY + barH` (bottom), 1 is `barY` (top), so the
  // stop position is simply the same `t`.
  const grad = ctx.createLinearGradient(0, barY + barH, 0, barY);
  for (const stop of scale.stops) grad.addColorStop(stop.t, stop.hex);
  ctx.fillStyle = grad;
  ctx.fillRect(barX, barY, barW, barH);
  ctx.strokeStyle = "#d4d4d8";
  ctx.lineWidth = 1;
  ctx.strokeRect(barX + 0.5, barY + 0.5, barW - 1, barH - 1);

  // Min / max labels at the bar's ends. Format with the chart's value
  // format when present (e.g. percent for the World Population fixture)
  // so the legend reads in the same units as the workbook data.
  ctx.font = `${LABEL_FONT_SIZE}px -apple-system, "Helvetica Neue", Arial, sans-serif`;
  ctx.fillStyle = LABEL_COLOR;
  ctx.textAlign = "left";
  const fmt = (v: number) =>
    chart.valueFormat ? formatAxisValue(v, chart.valueFormat) : formatGeneral(v);
  ctx.textBaseline = "top";
  ctx.fillText(fmt(scale.max), barX + barW + 4, barY);
  ctx.textBaseline = "bottom";
  ctx.fillText(fmt(scale.min), barX + barW + 4, barY + barH);

  // Suppress the unused-binding lint without taking out the slot
  // (`legendW` is part of the public layout contract for this fn).
  void legendW;
}
