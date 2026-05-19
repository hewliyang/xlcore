import type { CfColorScale, Color } from "./types.js";
import { colorToCss } from "./color.js";

interface ResolvedStop {
  value: number;
  rgb: [number, number, number];
}

export function resolveColorScaleStops(cs: CfColorScale, values: number[]): ResolvedStop[] {
  const min = Math.min(...values);
  const max = Math.max(...values);
  const sorted = [...values].sort((a, b) => a - b);
  return cs.stops
    .map((s) => {
      let v: number;
      switch (s.type) {
        case "min":
          v = min;
          break;
        case "max":
          v = max;
          break;
        case "num":
          v = parseFloat(s.val ?? "0");
          break;
        case "percent": {
          const p = parseFloat(s.val ?? "0") / 100;
          v = min + (max - min) * p;
          break;
        }
        case "percentile": {
          const p = parseFloat(s.val ?? "0") / 100;
          const idx = Math.min(sorted.length - 1, Math.max(0, Math.round(p * (sorted.length - 1))));
          v = sorted[idx] ?? min;
          break;
        }

        default:
          v = min;
      }
      return { value: v, rgb: rgbTriple(s.color) };
    })
    .sort((a, b) => a.value - b.value);
}

function rgbTriple(c: Color): [number, number, number] {
  const css = colorToCss(c, "#ffffff");
  return [
    parseInt(css.slice(1, 3), 16),
    parseInt(css.slice(3, 5), 16),
    parseInt(css.slice(5, 7), 16),
  ];
}

export function interpolateStops(stops: ResolvedStop[], value: number): string | null {
  if (stops.length === 0) return null;
  const first = stops[0]!;
  const last = stops[stops.length - 1]!;
  if (value <= first.value) return rgbToCss(first.rgb);
  if (value >= last.value) return rgbToCss(last.rgb);
  for (let i = 0; i < stops.length - 1; i++) {
    const a = stops[i]!;
    const b = stops[i + 1]!;
    if (value >= a.value && value <= b.value) {
      const span = b.value - a.value;
      const t = span === 0 ? 0 : (value - a.value) / span;
      const r = Math.round(a.rgb[0] + (b.rgb[0] - a.rgb[0]) * t);
      const gg = Math.round(a.rgb[1] + (b.rgb[1] - a.rgb[1]) * t);
      const bb = Math.round(a.rgb[2] + (b.rgb[2] - a.rgb[2]) * t);
      return `rgb(${r}, ${gg}, ${bb})`;
    }
  }
  return null;
}

function rgbToCss(rgb: [number, number, number]): string {
  return `rgb(${rgb[0]}, ${rgb[1]}, ${rgb[2]})`;
}
