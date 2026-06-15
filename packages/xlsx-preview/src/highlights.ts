import type { DependencyReference } from "./api-schema/DependencyReference.js";
import type { Grid } from "./grid.js";
import type { HighlightRange } from "./renderTypes.js";

export function referencesToHighlights(
  refs: DependencyReference[],
  activeSheetName: string,
  palette: string[],
): HighlightRange[] {
  if (palette.length === 0) return [];
  const out: HighlightRange[] = [];
  for (const ref of refs) {
    if (ref.sheet !== activeSheetName) continue;
    out.push({
      r1: ref.startRow,
      c1: ref.startColumn,
      r2: ref.endRow,
      c2: ref.endColumn,
      color: palette[out.length % palette.length]!,
    });
  }
  return out;
}

export interface HighlightRect {
  x: number;
  y: number;
  w: number;
  h: number;
  color: string;
}

function clamp(v: number, lo: number, hi: number): number {
  return v < lo ? lo : v > hi ? hi : v;
}

export function buildHighlightRects(g: Grid, highlights: HighlightRange[]): HighlightRect[] {
  const rects: HighlightRect[] = [];
  for (const h of highlights) {
    const r1 = clamp(Math.min(h.r1, h.r2), 1, g.maxRow);
    const r2 = clamp(Math.max(h.r1, h.r2), 1, g.maxRow);
    const c1 = clamp(Math.min(h.c1, h.c2), 1, g.maxCol);
    const c2 = clamp(Math.max(h.c1, h.c2), 1, g.maxCol);
    const x1 = g.colX[c1] ?? 0;
    const x2 = g.colX[c2 + 1] ?? x1;
    const y1 = g.rowY[r1] ?? 0;
    const y2 = g.rowY[r2 + 1] ?? y1;
    if (x2 <= x1 || y2 <= y1) continue;
    rects.push({ x: x1, y: y1, w: x2 - x1, h: y2 - y1, color: h.color });
  }
  return rects;
}

function fillFromHex(hex: string, alpha: number): string {
  const r = parseInt(hex.slice(1, 3), 16);
  const g = parseInt(hex.slice(3, 5), 16);
  const b = parseInt(hex.slice(5, 7), 16);
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

export function drawHighlights(
  ctx: CanvasRenderingContext2D,
  g: Grid,
  highlights: HighlightRange[],
): void {
  const rects = buildHighlightRects(g, highlights);
  if (rects.length === 0) return;
  ctx.save();
  ctx.setLineDash([]);
  ctx.lineWidth = 2;
  for (const rect of rects) {
    ctx.fillStyle = fillFromHex(rect.color, 0.1);
    ctx.fillRect(rect.x, rect.y, rect.w, rect.h);
    ctx.strokeStyle = rect.color;
    ctx.strokeRect(rect.x + 1, rect.y + 1, rect.w - 2, rect.h - 2);
  }
  ctx.restore();
}
