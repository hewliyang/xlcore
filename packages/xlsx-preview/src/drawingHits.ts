import { anchorToRect, type Grid } from "./grid.js";
import type { Drawing, DrawingHyperlink, Hyperlink, Sheet } from "./types.js";

export function drawingHyperlinkAt(
  sheet: Sheet,
  grid: Grid,
  x: number,
  y: number,
): Hyperlink | undefined {
  for (const d of sheet.drawings ?? []) {
    const link = d.hyperlink;
    if (!link) continue;
    const rect = anchorToRect(d, grid);
    if (!rect) continue;
    if (x < rect.x || y < rect.y || x >= rect.x + rect.w || y >= rect.y + rect.h) continue;
    return drawingHyperlinkToCellHyperlink(link);
  }
  return undefined;
}

export function drawingHyperlinkToCellHyperlink(link: DrawingHyperlink): Hyperlink {
  return {
    range: { r1: 0, c1: 0, r2: 0, c2: 0 },
    target: link.target ?? undefined,
    location: link.location ?? undefined,
    tooltip: link.tooltip ?? undefined,
    display: link.display ?? undefined,
  };
}

export function drawingAtPoint(
  sheet: Sheet,
  grid: Grid,
  x: number,
  y: number,
): Drawing | undefined {
  for (const d of sheet.drawings ?? []) {
    const rect = anchorToRect(d, grid);
    if (!rect) continue;
    if (x >= rect.x && y >= rect.y && x < rect.x + rect.w && y < rect.y + rect.h) {
      return d;
    }
  }
  return undefined;
}
