import type { Sheet } from "./types.js";
import { HEADER_H, HEADER_W } from "./grid.js";
import type { Grid } from "./grid.js";
import { GRID_COLOR } from "./renderConstants.js";
import type { Visible } from "./renderTypes.js";

// Draw default faint grid lines across the data area.
export function drawGridLines(
  ctx: CanvasRenderingContext2D,
  sheet: Sheet,
  g: Grid,
  vis: Visible,
): void {
  if (!sheet.showGridLines) return;
  const top = g.rowY[vis.firstRow] ?? HEADER_H;
  const bot = g.rowY[vis.lastRow + 1] ?? g.totalH;
  const left = g.colX[vis.firstCol] ?? HEADER_W;
  const right = g.colX[vis.lastCol + 1] ?? g.totalW;
  ctx.strokeStyle = GRID_COLOR;
  ctx.lineWidth = 1;
  ctx.beginPath();
  for (let c = vis.firstCol; c <= vis.lastCol + 1; c++) {
    const x = Math.round(g.colX[c] ?? 0) + 0.5;
    ctx.moveTo(x, top);
    ctx.lineTo(x, bot);
  }
  for (let r = vis.firstRow; r <= vis.lastRow + 1; r++) {
    const y = Math.round(g.rowY[r] ?? 0) + 0.5;
    ctx.moveTo(left, y);
    ctx.lineTo(right, y);
  }
  ctx.stroke();
}

export interface CellRect {
  x: number;
  y: number;
  w: number;
  h: number;
}

export function cellRect(g: Grid, r: number, c: number): CellRect {
  return { x: g.colX[c] ?? 0, y: g.rowY[r] ?? 0, w: g.colW[c] ?? 0, h: g.rowH[r] ?? 0 };
}

export function mergedRect(
  g: Grid,
  m: { r1: number; c1: number; r2: number; c2: number },
): CellRect {
  const x = g.colX[m.c1] ?? 0;
  const y = g.rowY[m.r1] ?? 0;
  return {
    x,
    y,
    w: (g.colX[m.c2 + 1] ?? x) - x,
    h: (g.rowY[m.r2 + 1] ?? y) - y,
  };
}

export function buildMergeMaps(sheet: Sheet): {
  covered: Set<string>;
  topLeftOf: Map<string, { r1: number; c1: number; r2: number; c2: number }>;
} {
  const covered = new Set<string>();
  const topLeftOf = new Map<string, { r1: number; c1: number; r2: number; c2: number }>();
  for (const m of sheet.merges) {
    for (let r = m.r1; r <= m.r2; r++) {
      for (let c = m.c1; c <= m.c2; c++) {
        const k = `${r}:${c}`;
        topLeftOf.set(k, m);
        if (!(r === m.r1 && c === m.c1)) covered.add(k);
      }
    }
  }
  return { covered, topLeftOf };
}

export function rectFor(
  sheet: Sheet,
  g: Grid,
  r: number,
  c: number,
  topLeftOf: Map<string, { r1: number; c1: number; r2: number; c2: number }>,
): CellRect {
  const m = topLeftOf.get(`${r}:${c}`);
  return m ? mergedRect(g, m) : cellRect(g, r, c);
}

// Find the cell at (r,c) — O(log rowCount + log cellsInRow) via the
// columnar `rowPtr` index. Returns a freshly-materialized POJO from
// the sheet's typed-array storage; callers that ask for the same cell
// in a hot loop should cache the result.
export { findCell } from "./columnar.js";
