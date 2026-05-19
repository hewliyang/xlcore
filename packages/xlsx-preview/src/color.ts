import type { Color, WorkbookLayout } from "./types.js";

const INDEXED_PALETTE: Record<number, string> = {
  0: "#000000",
  1: "#ffffff",
  2: "#ff0000",
  3: "#00ff00",
  4: "#0000ff",
  5: "#ffff00",
  6: "#ff00ff",
  7: "#00ffff",
  8: "#000000",
  9: "#ffffff",
  10: "#ff0000",
  11: "#00ff00",
  12: "#0000ff",
  13: "#ffff00",
  14: "#ff00ff",
  15: "#00ffff",
  16: "#800000",
  17: "#008000",
  18: "#000080",
  19: "#808000",
  20: "#800080",
  21: "#008080",
  22: "#c0c0c0",
  23: "#808080",
  24: "#9999ff",
  25: "#993366",
  26: "#ffffcc",
  27: "#ccffff",
  28: "#660066",
  29: "#ff8080",
  30: "#0066cc",
  31: "#ccccff",
  32: "#000080",
  33: "#ff00ff",
  34: "#ffff00",
  35: "#00ffff",
  36: "#800080",
  37: "#800000",
  38: "#008080",
  39: "#0000ff",
  40: "#00ccff",
  41: "#ccffff",
  42: "#ccffcc",
  43: "#ffff99",
  44: "#99ccff",
  45: "#ff99cc",
  46: "#cc99ff",
  47: "#ffcc99",
  48: "#3366ff",
  49: "#33cccc",
  50: "#99cc00",
  51: "#ffcc00",
  52: "#ff9900",
  53: "#ff6600",
  54: "#666699",
  55: "#969696",
  56: "#003366",
  57: "#339966",
  58: "#003300",
  59: "#333300",
  60: "#993300",
  61: "#993366",
  62: "#333399",
  63: "#333333",
  64: "#000000",
  65: "#ffffff",
};

const DEFAULT_THEME_PALETTE: Record<number, string> = {
  0: "#ffffff",
  1: "#000000",
  2: "#e7e6e6",
  3: "#44546a",
  4: "#4472c4",
  5: "#ed7d31",
  6: "#a5a5a5",
  7: "#ffc000",
  8: "#5b9bd5",
  9: "#70ad47",
  10: "#0563c1",
  11: "#954f72",
};

let activeThemePalette: Record<number, string> = DEFAULT_THEME_PALETTE;

function paletteForTheme(theme: WorkbookLayout["theme"]): Record<number, string> {
  if (!theme || !theme.colors || theme.colors.length === 0) return DEFAULT_THEME_PALETTE;
  const map: Record<number, string> = { ...DEFAULT_THEME_PALETTE };
  theme.colors.forEach((hex, i) => {
    if (hex && /^[0-9a-fA-F]{6}$/.test(hex)) map[i] = "#" + hex.toLowerCase();
  });
  return map;
}

export function setActiveTheme(theme: WorkbookLayout["theme"]): void {
  activeThemePalette = paletteForTheme(theme);
}

export function activeThemeColor(index: number, fallback: string): string {
  return activeThemePalette[index] ?? fallback;
}

function parseRgbString(rgb: string): string | null {
  if (rgb.length === 8) return "#" + rgb.slice(2);
  if (rgb.length === 6) return "#" + rgb;
  return null;
}

export function applyTint(hex: string, tint: number): string {
  const r = parseInt(hex.slice(1, 3), 16) / 255;
  const g = parseInt(hex.slice(3, 5), 16) / 255;
  const b = parseInt(hex.slice(5, 7), 16) / 255;

  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const l = (max + min) / 2;
  let h = 0;
  let s = 0;
  if (max !== min) {
    const d = max - min;
    s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
    switch (max) {
      case r:
        h = (g - b) / d + (g < b ? 6 : 0);
        break;
      case g:
        h = (b - r) / d + 2;
        break;
      default:
        h = (r - g) / d + 4;
    }
    h /= 6;
  }

  let l2 = tint < 0 ? l * (1 + tint) : l * (1 - tint) + tint;
  if (l2 < 0) l2 = 0;
  if (l2 > 1) l2 = 1;

  let r2: number, g2: number, b2: number;
  if (s === 0) {
    r2 = g2 = b2 = l2;
  } else {
    const q = l2 < 0.5 ? l2 * (1 + s) : l2 + s - l2 * s;
    const p = 2 * l2 - q;
    const hue2rgb = (t: number): number => {
      if (t < 0) t += 1;
      if (t > 1) t -= 1;
      if (t < 1 / 6) return p + (q - p) * 6 * t;
      if (t < 1 / 2) return q;
      if (t < 2 / 3) return p + (q - p) * (2 / 3 - t) * 6;
      return p;
    };
    r2 = hue2rgb(h + 1 / 3);
    g2 = hue2rgb(h);
    b2 = hue2rgb(h - 1 / 3);
  }

  const toHex = (v: number) =>
    Math.round(v * 255)
      .toString(16)
      .padStart(2, "0");
  return "#" + toHex(r2) + toHex(g2) + toHex(b2);
}

function colorToCssUsingPalette(
  c: Color | undefined,
  fallback: string,
  palette: Record<number, string>,
): string {
  if (!c) return fallback;
  let base: string | null = null;
  if (c.rgb) base = parseRgbString(c.rgb);
  else if (c.theme !== undefined) base = palette[c.theme] ?? null;
  else if (c.indexed !== undefined) base = INDEXED_PALETTE[c.indexed] ?? null;
  if (!base) return fallback;
  if (c.tint && c.tint !== 0) return applyTint(base, c.tint);
  return base;
}

export function colorToCss(c: Color | undefined, fallback = "#000000"): string {
  return colorToCssUsingPalette(c, fallback, activeThemePalette);
}

export function colorToCssWithTheme(
  c: Color | undefined,
  theme: WorkbookLayout["theme"],
  fallback = "#000000",
): string {
  return colorToCssUsingPalette(c, fallback, paletteForTheme(theme));
}
