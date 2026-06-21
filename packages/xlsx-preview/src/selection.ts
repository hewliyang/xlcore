import type { Sheet } from "./types.js";
import { clamp } from "./mathUtils.js";
import type { Grid } from "./grid.js";
import { buildMergeMaps, cellRect, mergedRect } from "./geometry.js";
import { SELECTION_FILL, SELECTION_STROKE } from "./renderConstants.js";
import type { RenderOptions } from "./renderTypes.js";

export function resolveSelection(
  opts: RenderOptions,
  g: Grid,
): { r1: number; c1: number; r2: number; c2: number } | null {
  if (opts.selection) {
    const s = opts.selection;
    return {
      r1: clamp(Math.min(s.r1, s.r2), 1, g.maxRow),
      r2: clamp(Math.max(s.r1, s.r2), 1, g.maxRow),
      c1: clamp(Math.min(s.c1, s.c2), 1, g.maxCol),
      c2: clamp(Math.max(s.c1, s.c2), 1, g.maxCol),
    };
  }
  if (opts.activeCell) {
    const a = opts.activeCell;
    if (a.r < 1 || a.c < 1) return null;
    return { r1: a.r, r2: a.r, c1: a.c, c2: a.c };
  }
  return null;
}


export function drawSelection(
  ctx: CanvasRenderingContext2D,
  sheet: Sheet,
  g: Grid,
  sel: { r1: number; c1: number; r2: number; c2: number },
  active: { r: number; c: number } | null,
): void {
  const x1 = g.colX[sel.c1] ?? 0;
  const x2 = g.colX[sel.c2 + 1] ?? x1;
  const y1 = g.rowY[sel.r1] ?? 0;
  const y2 = g.rowY[sel.r2 + 1] ?? y1;
  if (x2 <= x1 || y2 <= y1) return;

  ctx.save();
  ctx.fillStyle = SELECTION_FILL;

  const isSingle = sel.r1 === sel.r2 && sel.c1 === sel.c2;
  if (!isSingle) {
    if (
      active &&
      active.r >= sel.r1 &&
      active.r <= sel.r2 &&
      active.c >= sel.c1 &&
      active.c <= sel.c2
    ) {
      const { topLeftOf } = buildMergeMaps(sheet);
      const m = topLeftOf.get(`${active.r}:${active.c}`);
      const ar = m ? mergedRect(g, m) : cellRect(g, active.r, active.c);

      const ax1 = ar.x,
        ay1 = ar.y,
        ax2 = ar.x + ar.w,
        ay2 = ar.y + ar.h;
      if (ay1 > y1) ctx.fillRect(x1, y1, x2 - x1, ay1 - y1);
      if (ay2 < y2) ctx.fillRect(x1, ay2, x2 - x1, y2 - ay2);
      if (ax1 > x1) ctx.fillRect(x1, ay1, ax1 - x1, ay2 - ay1);
      if (ax2 < x2) ctx.fillRect(ax2, ay1, x2 - ax2, ay2 - ay1);
    } else {
      ctx.fillRect(x1, y1, x2 - x1, y2 - y1);
    }
  }

  ctx.strokeStyle = SELECTION_STROKE;
  ctx.lineWidth = 2;
  ctx.setLineDash([]);
  ctx.strokeRect(x1 + 1, y1 + 1, x2 - x1 - 2, y2 - y1 - 2);

  ctx.fillStyle = SELECTION_STROKE;
  ctx.fillRect(x2 - 4, y2 - 4, 6, 6);
  ctx.strokeStyle = "#ffffff";
  ctx.lineWidth = 1;
  ctx.strokeRect(x2 - 3.5, y2 - 3.5, 5, 5);
  ctx.restore();
}
