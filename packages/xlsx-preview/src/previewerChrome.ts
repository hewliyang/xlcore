import { iterRows } from "./columnar.js";
import { clamp } from "./mathUtils.js";
import { colorToCssWithTheme } from "./color.js";
import { HEADER_H, HEADER_W } from "./render.js";
import type { Selection } from "./interact.js";
import type { Sheet as WireSheet } from "./schema/Sheet.js";
import type { Sheet, WorkbookLayout } from "./types.js";

export const VIRTUAL_EXTRA_COLS = 50;
export const VIRTUAL_EXTRA_ROWS = 1000;

export interface VirtualSizeState {
  colOverrides: Map<number, number>;
  rowOverrides: Map<number, number>;
}

export function virtualSize(sheet: Sheet, state: VirtualSizeState): { w: number; h: number } {
  const dw = sheet.defaultColWidthPx || 64;
  const dh = sheet.defaultRowHeightPx || 18;
  const maxCol = Math.min(16384, Math.max(sheet.maxCol + 2, sheet.maxCol + VIRTUAL_EXTRA_COLS));
  const maxRow = Math.min(1048576, Math.max(sheet.maxRow + 5, sheet.maxRow + VIRTUAL_EXTRA_ROWS));
  let w = HEADER_W + maxCol * dw;
  let h = HEADER_H + maxRow * dh;
  const colWidths = new Map<number, number>();
  for (const c of sheet.cols) {
    for (let i = c.min; i <= c.max; i++) colWidths.set(i, c.hidden ? 0 : c.widthPx);
  }
  for (const [c, v] of state.colOverrides) colWidths.set(c, Math.max(0, v));
  for (const [c, v] of colWidths) if (c >= 1 && c <= maxCol) w += v - dw;
  const rowHeights = new Map<number, number>();
  iterRows(sheet, (row) => {
    if (row.hidden) rowHeights.set(row.index, 0);
    else if (row.heightPx !== undefined) rowHeights.set(row.index, row.heightPx);
  });
  for (const [r, v] of state.rowOverrides) rowHeights.set(r, Math.max(0, v));
  for (const [r, v] of rowHeights) if (r >= 1 && r <= maxRow) h += v - dh;
  return { w, h };
}

export function normalizeSelection(selection: Selection, maxRow: number, maxCol: number): Selection {
  const r1 = clamp(Math.floor(Math.min(selection.r1, selection.r2)), 1, maxRow);
  const r2 = clamp(Math.floor(Math.max(selection.r1, selection.r2)), 1, maxRow);
  const c1 = clamp(Math.floor(Math.min(selection.c1, selection.c2)), 1, maxCol);
  const c2 = clamp(Math.floor(Math.max(selection.c1, selection.c2)), 1, maxCol);
  return { r1, c1, r2, c2 };
}

export function makeButton(label: string): HTMLButtonElement {
  const button = document.createElement("button");
  button.textContent = label;
  button.style.cssText =
    "background:#fff;border:1px solid #d1d5db;padding:4px 10px;cursor:pointer;font:inherit;font-size:12px;border-radius:4px;";
  return button;
}

export function makeTab(label: string, sheet: WireSheet, layout: WorkbookLayout): HTMLButtonElement {
  const button = document.createElement("button");
  button.textContent = label;
  let bg = "#fff";
  let fg = "#111827";
  if (sheet.tabColor) {
    const tab = colorToCssWithTheme(sheet.tabColor, layout.theme, "#9ca3af");
    button.dataset.tabColor = tab;
    bg = tab;
    fg = contrastingTextColor(tab);
  }
  button.style.cssText = `flex:none;background:${bg};color:${fg};border:1px solid #d1d5db;border-bottom:none;padding:6px 14px;cursor:pointer;font:inherit;font-size:12px;white-space:nowrap;`;
  return button;
}

export function contrastingTextColor(css: string): string {
  if (css.length !== 7 || css[0] !== "#") return "#111827";
  const r = parseInt(css.slice(1, 3), 16);
  const g = parseInt(css.slice(3, 5), 16);
  const b = parseInt(css.slice(5, 7), 16);
  const luma = (r * 299 + g * 587 + b * 114) / 1000;
  return luma > 140 ? "#111827" : "#ffffff";
}
