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

const LAT_MIN = -58;
const LAT_MAX = 84;
const LON_MIN = -180;
const LON_MAX = 180;

let NAME_INDEX: Map<string, World110mFeature> | null = null;

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
  dprk: "north korea",
  russia: "russia",
  "russian federation": "russia",
  czechia: "czech republic",
  "czech republic": "czech republic",
  "ivory coast": "ivory coast",
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

interface Projection {
  fwd: (lon: number, lat: number) => [number, number];
}

function makeEquirectangular(rect: Rect): Projection {
  const w = rect.w;
  const h = rect.h;
  const sx = w / (LON_MAX - LON_MIN);
  const sy = h / (LAT_MAX - LAT_MIN);

  const s = Math.min(sx, sy);
  const worldW = (LON_MAX - LON_MIN) * s;
  const worldH = (LAT_MAX - LAT_MIN) * s;
  const ox = rect.x + (w - worldW) / 2;
  const oy = rect.y + (h - worldH) / 2;
  return {
    fwd: (lon, lat) => [ox + (lon - LON_MIN) * s, oy + (LAT_MAX - lat) * s],
  };
}

function hexToRgb(hex: string): [number, number, number] {
  const m = /^#?([0-9a-f]{6})$/i.exec(hex.trim());
  if (!m) return [68, 114, 196];
  const n = parseInt(m[1]!, 16);
  return [(n >> 16) & 0xff, (n >> 8) & 0xff, n & 0xff];
}

function rgbToHex(r: number, g: number, b: number): string {
  const c = (n: number) =>
    Math.max(0, Math.min(255, Math.round(n)))
      .toString(16)
      .padStart(2, "0");
  return `#${c(r)}${c(g)}${c(b)}`;
}

function lerpColor(a: [number, number, number], b: [number, number, number], t: number): string {
  return rgbToHex(a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t);
}

interface ColorScale {
  color: (v: number) => string;
  min: number;
  max: number;

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

  let stops: { t: number; rgb: [number, number, number]; hex: string }[];
  if (authored.min && authored.max && authored.mid) {
    stops = [stopAt(0, authored.min), stopAt(0.5, authored.mid), stopAt(1, authored.max)];
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

function stopAt(t: number, hex: string): { t: number; rgb: [number, number, number]; hex: string } {
  const rgb = hexToRgb(hex);
  return { t, rgb, hex };
}

function lerpStops(stops: { t: number; rgb: [number, number, number] }[], t: number): string {
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

function tracePolygon(ctx: CanvasRenderingContext2D, rings: number[][][], proj: Projection): void {
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

function traceFeature(ctx: CanvasRenderingContext2D, f: World110mFeature, proj: Projection): void {
  const g = f.g;
  if (g.type === "Polygon") {
    tracePolygon(ctx, g.coordinates, proj);
  } else {
    for (const poly of g.coordinates) tracePolygon(ctx, poly, proj);
  }
}

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

  const showLegend = !!chart.legendPos && chart.legendPos !== "n";
  const LEGEND_W = 64;
  const LEGEND_PAD = 8;
  const mapRect: Rect = showLegend
    ? { ...rect, w: Math.max(40, rect.w - LEGEND_W - LEGEND_PAD) }
    : rect;

  const proj = makeEquirectangular(mapRect);

  const matched = new Map<World110mFeature, number>();
  for (let i = 0; i < cats.length; i++) {
    const label = cats[i];
    const v = values[i];
    if (!label || v == null || !Number.isFinite(v)) continue;
    const f = lookupCountry(label);
    if (f) matched.set(f, v);
  }

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

  const barH = Math.min(rect.h * 0.7, 240);
  const barY = rect.y + (rect.h - barH) / 2;

  const grad = ctx.createLinearGradient(0, barY + barH, 0, barY);
  for (const stop of scale.stops) grad.addColorStop(stop.t, stop.hex);
  ctx.fillStyle = grad;
  ctx.fillRect(barX, barY, barW, barH);
  ctx.strokeStyle = "#d4d4d8";
  ctx.lineWidth = 1;
  ctx.strokeRect(barX + 0.5, barY + 0.5, barW - 1, barH - 1);

  ctx.font = `${LABEL_FONT_SIZE}px -apple-system, "Helvetica Neue", Arial, sans-serif`;
  ctx.fillStyle = LABEL_COLOR;
  ctx.textAlign = "left";
  const fmt = (v: number) =>
    chart.valueFormat ? formatAxisValue(v, chart.valueFormat) : formatGeneral(v);
  ctx.textBaseline = "top";
  ctx.fillText(fmt(scale.max), barX + barW + 4, barY);
  ctx.textBaseline = "bottom";
  ctx.fillText(fmt(scale.min), barX + barW + 4, barY + barH);

  void legendW;
}
