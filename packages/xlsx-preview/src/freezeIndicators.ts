import type { Grid } from "./grid.js";
import { frozenDims } from "./panes.js";
import type { Sheet } from "./types.js";

export function drawFreezeIndicators(
  ctx: CanvasRenderingContext2D,
  sheet: Sheet,
  g: Grid,
  canvasW: number,
  canvasH: number,
): void {
  if (!sheet.freeze) return;

  const { pcw, prh } = frozenDims(sheet, g);
  ctx.save();
  ctx.strokeStyle = "#9ca3af";
  ctx.lineWidth = 1;
  ctx.beginPath();
  if (sheet.freeze.leftCol > 1) {
    const x = Math.round(g.originX + pcw) + 0.5;
    ctx.moveTo(x, 0);
    ctx.lineTo(x, canvasH);
  }
  if (sheet.freeze.topRow > 1) {
    const y = Math.round(g.originY + prh) + 0.5;
    ctx.moveTo(0, y);
    ctx.lineTo(canvasW, y);
  }
  ctx.stroke();
  ctx.restore();
}
