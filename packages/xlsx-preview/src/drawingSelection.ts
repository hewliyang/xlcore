import { SELECTION_STROKE } from "./renderConstants.js";

export interface DrawingRect {
  x: number;
  y: number;
  w: number;
  h: number;
}

export const HANDLE_SIZE = 7;

export function drawingHandles(rect: DrawingRect): DrawingRect[] {
  const { x, y, w, h } = rect;
  const cx = x + w / 2;
  const cy = y + h / 2;
  const pts: Array<[number, number]> = [
    [x, y],
    [cx, y],
    [x + w, y],
    [x + w, cy],
    [x + w, y + h],
    [cx, y + h],
    [x, y + h],
    [x, cy],
  ];
  const s = HANDLE_SIZE;
  return pts.map(([px, py]) => ({ x: px - s / 2, y: py - s / 2, w: s, h: s }));
}

export function drawingHandleAtPoint(rect: DrawingRect, x: number, y: number, tol = 2): number | null {
  const handles = drawingHandles(rect);
  for (let i = 0; i < handles.length; i++) {
    const h = handles[i];
    if (!h) continue;
    if (x >= h.x - tol && x <= h.x + h.w + tol && y >= h.y - tol && y <= h.y + h.h + tol) return i;
  }
  return null;
}

const HANDLE_CURSORS = [
  "nwse-resize",
  "ns-resize",
  "nesw-resize",
  "ew-resize",
  "nwse-resize",
  "ns-resize",
  "nesw-resize",
  "ew-resize",
];

export function drawingHandleCursor(index: number): string {
  return HANDLE_CURSORS[index] ?? "default";
}

export function drawDrawingSelection(ctx: CanvasRenderingContext2D, rect: DrawingRect): void {
  ctx.save();
  ctx.strokeStyle = SELECTION_STROKE;
  ctx.lineWidth = 1;
  ctx.strokeRect(rect.x + 0.5, rect.y + 0.5, rect.w - 1, rect.h - 1);
  ctx.fillStyle = "#ffffff";
  for (const h of drawingHandles(rect)) {
    ctx.fillRect(h.x, h.y, h.w, h.h);
    ctx.strokeRect(h.x + 0.5, h.y + 0.5, h.w - 1, h.h - 1);
  }
  ctx.restore();
}
