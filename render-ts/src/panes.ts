import type { Sheet } from "./types.js";

import type { Grid } from "./grid.js";
import type { Pane, Viewport, Visible } from "./renderTypes.js";

// ---------- panes ----------

// How wide the pinned-column strip is, and how tall the pinned-row strip
// is, in logical CSS px. 0 when that axis is unfrozen.
function frozenExtent(
  sheet: Sheet,
  g: Grid,
): { splitX: number; splitY: number; pcw: number; prh: number } {
  const fz = sheet.freeze;
  const splitX = fz && fz.leftCol > 1 ? fz.leftCol : 1;
  const splitY = fz && fz.topRow > 1 ? fz.topRow : 1;
  const pcw = splitX > 1 ? (g.colX[splitX] ?? g.originX) - g.originX : 0;
  const prh = splitY > 1 ? (g.rowY[splitY] ?? g.originY) - g.originY : 0;
  return { splitX, splitY, pcw, prh };
}

export function splitPanes(
  sheet: Sheet,
  g: Grid,
  vp: Viewport | null,
  canvasW: number,
  canvasH: number,
): Pane[] {
  const { splitX, splitY, pcw, prh } = frozenExtent(sheet, g);
  const hasH = splitX > 1;
  const hasV = splitY > 1;
  const vpx = vp ? vp.x : 0;
  const vpy = vp ? vp.y : 0;

  const panes: Pane[] = [];

  // BR (always present) — covers everything past both splits.
  {
    const cx = g.originX + pcw;
    const cy = g.originY + prh;
    const cw = Math.max(0, canvasW - cx);
    const ch = Math.max(0, canvasH - cy);
    const tx = -vpx;
    const ty = -vpy;
    const vis = paneVisible(g, cx, cy, cw, ch, tx, ty);
    if (hasH) vis.firstCol = Math.max(vis.firstCol, splitX);
    if (hasV) vis.firstRow = Math.max(vis.firstRow, splitY);
    panes.push({ cx, cy, cw, ch, tx, ty, vis, kind: "br" });
  }
  if (hasV) {
    // TR — pinned rows, scrolling cols.
    const cx = g.originX + pcw;
    const cy = g.originY;
    const cw = Math.max(0, canvasW - cx);
    const ch = prh;
    const tx = -vpx;
    const ty = 0;
    const vis = paneVisible(g, cx, cy, cw, ch, tx, ty);
    if (hasH) vis.firstCol = Math.max(vis.firstCol, splitX);
    vis.firstRow = 1;
    vis.lastRow = Math.min(vis.lastRow, splitY - 1);
    panes.push({ cx, cy, cw, ch, tx, ty, vis, kind: "tr" });
  }
  if (hasH) {
    // BL — pinned cols, scrolling rows.
    const cx = g.originX;
    const cy = g.originY + prh;
    const cw = pcw;
    const ch = Math.max(0, canvasH - cy);
    const tx = 0;
    const ty = -vpy;
    const vis = paneVisible(g, cx, cy, cw, ch, tx, ty);
    vis.firstCol = 1;
    vis.lastCol = Math.min(vis.lastCol, splitX - 1);
    if (hasV) vis.firstRow = Math.max(vis.firstRow, splitY);
    panes.push({ cx, cy, cw, ch, tx, ty, vis, kind: "bl" });
  }
  if (hasH && hasV) {
    // TL — fully pinned corner.
    const cx = g.originX;
    const cy = g.originY;
    const cw = pcw;
    const ch = prh;
    const vis: Visible = {
      firstCol: 1,
      lastCol: splitX - 1,
      firstRow: 1,
      lastRow: splitY - 1,
    };
    panes.push({ cx, cy, cw, ch, tx: 0, ty: 0, vis, kind: "tl" });
  }
  return panes;
}

function paneVisible(
  g: Grid,
  cx: number,
  cy: number,
  cw: number,
  ch: number,
  tx: number,
  ty: number,
): Visible {
  // Inverse of the canvas transform: absX = cx - tx (since canvasX = absX + tx).
  const ax1 = cx - tx;
  const ay1 = cy - ty;
  return visibleRange(g, ax1, ay1, ax1 + cw, ay1 + ch);
}

/// Public for interact.ts — given a canvas-local point, returns the pane
/// whose clip rect contains it (so callers can convert to absolute logical
/// coords). Returns null if the point lies in a header strip or outside any
/// pane.
export function paneAtPoint(
  sheet: Sheet,
  g: Grid,
  vp: Viewport | null,
  canvasW: number,
  canvasH: number,
  cx: number,
  cy: number,
): { tx: number; ty: number; kind: "tl" | "tr" | "bl" | "br" } | null {
  const panes = splitPanes(sheet, g, vp, canvasW, canvasH);
  for (const p of panes) {
    if (cx >= p.cx && cx < p.cx + p.cw && cy >= p.cy && cy < p.cy + p.ch) {
      return { tx: p.tx, ty: p.ty, kind: p.kind };
    }
  }
  return null;
}

/// Public for interact.ts — exposes the frozen-strip widths so the resize
/// hit-tester can know which segment of the header (pinned vs scrolling) a
/// pointer is in.
export function frozenDims(
  sheet: Sheet,
  g: Grid,
): { splitX: number; splitY: number; pcw: number; prh: number } {
  return frozenExtent(sheet, g);
}

// Find the first/last column and row whose extent intersects the rect
// [x1,y1] - [x2,y2]. Uses a linear scan; grid sizes are small (<= a few
// thousand virtual cols/rows in practice).
function visibleRange(
  g: Grid,
  x1: number,
  y1: number,
  x2: number,
  y2: number,
): { firstCol: number; lastCol: number; firstRow: number; lastRow: number } {
  let firstCol = 1,
    lastCol = g.maxCol;
  for (let c = 1; c <= g.maxCol; c++) {
    const right = g.colX[c + 1] ?? g.colX[c] ?? 0;
    if (right > x1) {
      firstCol = c;
      break;
    }
  }
  for (let c = firstCol; c <= g.maxCol; c++) {
    const left = g.colX[c] ?? 0;
    if (left >= x2) {
      lastCol = c - 1;
      break;
    }
    lastCol = c;
  }
  let firstRow = 1,
    lastRow = g.maxRow;
  for (let r = 1; r <= g.maxRow; r++) {
    const bot = g.rowY[r + 1] ?? g.rowY[r] ?? 0;
    if (bot > y1) {
      firstRow = r;
      break;
    }
  }
  for (let r = firstRow; r <= g.maxRow; r++) {
    const top = g.rowY[r] ?? 0;
    if (top >= y2) {
      lastRow = r - 1;
      break;
    }
    lastRow = r;
  }
  return { firstCol, lastCol, firstRow, lastRow };
}
