import type { ShapeParagraph } from "./types.js";

const DEFAULT_FONT_PT = 11;
const PT_PER_PX = 0.75;

export type BulletGlyph = {
  text: string;
  font: string;
  color: string;
  width: number;
};

export class NumberingState {
  private state = new Map<number, { kind: string; type: string; n: number }>();
  next(level: number, type: string, startAt: number): number {
    const prev = this.state.get(level);
    if (prev && prev.kind === "autoNum" && prev.type === type) {
      prev.n += 1;
      return prev.n;
    }
    this.state.set(level, { kind: "autoNum", type, n: startAt });
    return startAt;
  }
  noteOther(level: number, kind: string) {
    const prev = this.state.get(level);
    if (!prev || prev.kind !== kind) this.state.delete(level);
  }
}

export function paragraphRefFontPt(p: ShapeParagraph): number {
  for (const r of p.runs ?? []) {
    if (r.size && r.size > 0) return r.size;
  }
  return DEFAULT_FONT_PT;
}

export function computeLineHeight(
  p: ShapeParagraph,
  refPt: number,
  fontScale: number,
  lineScale: number,
): number {
  const naturalPx = ((refPt * fontScale) / PT_PER_PX) * 1.2 * lineScale;
  if (p.lineSpacingPts != null) {
    return Math.max(1, Math.ceil(p.lineSpacingPts / 100 / PT_PER_PX));
  }
  if (p.lineSpacingPct != null) {
    return Math.max(1, Math.ceil(naturalPx * (p.lineSpacingPct / 100000)));
  }
  return Math.ceil(naturalPx);
}

export function computeParaSpacing(
  pct: number | undefined,
  pts: number | undefined,
  refPt: number,
  fontScale: number,
): number {
  if (pts != null) return Math.max(0, pts / 100 / PT_PER_PX);
  if (pct != null) {
    const naturalPx = ((refPt * fontScale) / PT_PER_PX) * 1.2;
    return Math.max(0, naturalPx * (pct / 100000));
  }
  return 0;
}

export function buildBulletGlyph(
  ctx: CanvasRenderingContext2D,
  p: ShapeParagraph,
  refPt: number,
  fontScale: number,
  numbering: NumberingState,
): BulletGlyph | null {
  const b = p.bullet;
  const level = p.level ?? 0;
  if (!b || b.kind === "none") {
    numbering.noteOther(level, "none");
    return null;
  }
  let bulletPt: number;
  if (b.sizePts != null) bulletPt = b.sizePts / 100;
  else if (b.sizePct != null) bulletPt = refPt * (b.sizePct / 100000);
  else bulletPt = refPt;
  let text: string;
  if (b.kind === "char" && b.char) {
    text = b.char;
  } else if (b.kind === "autoNum" && b.autoNumType) {
    const start = Math.max(1, b.autoNumStartAt ?? 1);
    text = autoNumGlyph(b.autoNumType, numbering.next(level, b.autoNumType, start));
  } else {
    numbering.noteOther(level, b.kind);
    text = "\u2022";
  }
  if (!text) return null;
  const family = b.font
    ? `"${b.font}", -apple-system, "Helvetica Neue", Arial, sans-serif`
    : '-apple-system, "Helvetica Neue", Arial, sans-serif';
  const px = (bulletPt * fontScale) / PT_PER_PX;
  const font = `400 ${px}px ${family}`;
  const explicit = b.color?.rgb ? `#${b.color.rgb.slice(-6)}` : null;
  const fallbackRun = (p.runs ?? []).find((r) => r.color?.rgb);
  const finalColor =
    explicit ?? (fallbackRun?.color?.rgb ? `#${fallbackRun.color.rgb.slice(-6)}` : "#000000");
  ctx.save();
  ctx.font = font;
  const width = ctx.measureText(`${text} `).width;
  ctx.restore();
  return { text, font, color: finalColor, width };
}

export function drawBullet(
  ctx: CanvasRenderingContext2D,
  b: BulletGlyph,
  x: number,
  y: number,
  lineHeight: number,
): void {
  ctx.save();
  ctx.font = b.font;
  ctx.fillStyle = b.color;
  ctx.textBaseline = "alphabetic";
  ctx.textAlign = "left";
  ctx.fillText(b.text, x, y + lineHeight * 0.82);
  ctx.restore();
}

function autoNumGlyph(type: string, n: number): string {
  const arabic = String(n);
  const alphaLc = toAlpha(n).toLowerCase();
  const alphaUc = toAlpha(n).toUpperCase();
  const romanLc = toRoman(n).toLowerCase();
  const romanUc = toRoman(n).toUpperCase();
  switch (type) {
    case "arabicPeriod": return `${arabic}.`;
    case "arabicParenR": return `${arabic})`;
    case "arabicParenBoth": return `(${arabic})`;
    case "arabicPlain": return arabic;
    case "alphaLcPeriod": return `${alphaLc}.`;
    case "alphaLcParenR": return `${alphaLc})`;
    case "alphaLcParenBoth": return `(${alphaLc})`;
    case "alphaUcPeriod": return `${alphaUc}.`;
    case "alphaUcParenR": return `${alphaUc})`;
    case "alphaUcParenBoth": return `(${alphaUc})`;
    case "romanLcPeriod": return `${romanLc}.`;
    case "romanLcParenR": return `${romanLc})`;
    case "romanLcParenBoth": return `(${romanLc})`;
    case "romanUcPeriod": return `${romanUc}.`;
    case "romanUcParenR": return `${romanUc})`;
    case "romanUcParenBoth": return `(${romanUc})`;
    default: return `${arabic}.`;
  }
}

function toAlpha(n: number): string {
  let s = "";
  let m = Math.max(1, n);
  while (m > 0) {
    const rem = (m - 1) % 26;
    s = String.fromCharCode(65 + rem) + s;
    m = Math.floor((m - 1) / 26);
  }
  return s || "A";
}

function toRoman(n: number): string {
  if (n <= 0 || n >= 4000) return String(n);
  const map: [number, string][] = [
    [1000, "M"], [900, "CM"], [500, "D"], [400, "CD"],
    [100, "C"], [90, "XC"], [50, "L"], [40, "XL"],
    [10, "X"], [9, "IX"], [5, "V"], [4, "IV"], [1, "I"],
  ];
  let out = "";
  let v = n;
  for (const [k, s] of map) {
    while (v >= k) {
      out += s;
      v -= k;
    }
  }
  return out;
}
