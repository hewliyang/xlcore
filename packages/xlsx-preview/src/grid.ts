import type { Drawing, Sheet } from "./types.js";
import { iterRows } from "./columnar.js";

export const HEADER_H = 22;
export const HEADER_W = 44;
// Width per outline level when an outline gutter strip is shown next to
// the row/col header strips. Excel's gutter is ~12px wide per level; we
// match that. Includes 4px outer padding on each side of the bracket
// column so the +/- button glyph isn't flush against the edges.
export const OUTLINE_GUTTER_STEP = 12;
export const OUTLINE_GUTTER_PAD = 4;

export interface Grid {
  colX: number[];
  colW: number[];
  rowY: number[];
  rowH: number[];
  totalW: number;
  totalH: number;
  maxCol: number;
  maxRow: number;
  /// Width of the row-grouping gutter strip painted to the left of the
  /// row-number column. 0 when the sheet has no row groupings.
  rowGutterW: number;
  /// Height of the column-grouping gutter strip painted above the
  /// column-letter row. 0 when the sheet has no column groupings.
  colGutterH: number;
  /// Left edge of column A on canvas = HEADER_W + rowGutterW.
  originX: number;
  /// Top edge of row 1 on canvas = HEADER_H + colGutterH.
  originY: number;
  /// Deepest row-outline level seen on the sheet (0 when none).
  rowOutlineDepth: number;
  /// Deepest col-outline level seen on the sheet (0 when none).
  colOutlineDepth: number;
}

const SHEET_MAX_COL = 16384;
const SHEET_MAX_ROW = 1048576;

export function buildGrid(
  sheet: Sheet,
  colOverrides?: Map<number, number>,
  rowOverrides?: Map<number, number>,
  requiredFarX?: number,
  requiredFarY?: number,
): Grid {
  let minCols = Math.max(sheet.maxCol, 1);
  let minRows = Math.max(sheet.maxRow, 1);
  const viewportOnly = requiredFarX !== undefined || requiredFarY !== undefined;
  if (viewportOnly) {
    minCols = 1;
    minRows = 1;
  }
  if (sheet.drawings) {
    for (const d of sheet.drawings) {
      minCols = Math.max(minCols, d.anchor.toCol + 2);
      minRows = Math.max(minRows, d.anchor.toRow + 2);
    }
  }
  let maxCol = Math.min(minCols + 2, SHEET_MAX_COL);
  let maxRow = Math.min(minRows + 5, SHEET_MAX_ROW);

  const colSpecW = new Map<number, number>();
  for (const c of sheet.cols) {
    const w = c.hidden ? 0 : c.widthPx;
    for (let i = c.min; i <= c.max; i++) colSpecW.set(i, w);
  }
  if (colOverrides) for (const [c, w] of colOverrides) colSpecW.set(c, Math.max(0, w));
  const widthOf = (c: number) => colSpecW.get(c) ?? sheet.defaultColWidthPx;

  const rowSpecH = new Map<number, number>();
  iterRows(sheet, (row) => {
    if (row.hidden) rowSpecH.set(row.index, 0);
    else if (row.heightPx !== undefined) rowSpecH.set(row.index, row.heightPx);
  });
  if (rowOverrides) for (const [r, h] of rowOverrides) rowSpecH.set(r, Math.max(0, h));
  const heightOf = (r: number) => rowSpecH.get(r) ?? sheet.defaultRowHeightPx;

  // Outline gutter widths. We compute these up front from sheet meta
  // and then shift the grid origin so every downstream coordinate
  // (panes, geometry, hit-testing) lands on the right pixel without
  // each call site having to know the gutter exists.
  let rowOutlineDepth = 0;
  if (sheet.decodedRowMeta && sheet.decodedRowMeta.outlineLevel.length > 0) {
    for (let i = 0; i < sheet.decodedRowMeta.outlineLevel.length; i++) {
      const v = sheet.decodedRowMeta.outlineLevel[i] ?? 0;
      if (v > rowOutlineDepth) rowOutlineDepth = v;
    }
  }
  let colOutlineDepth = 0;
  for (const c of sheet.cols) {
    const v = c.outlineLevel ?? 0;
    if (v > colOutlineDepth) colOutlineDepth = v;
  }
  // Either axis having groups also reserves the perpendicular gutter so
  // the level-numeral buttons at the corner have somewhere to live.
  // Excel renders the corner box (with 1/2/.../N buttons) only when at
  // least one axis is grouped; we follow suit by sizing each gutter
  // strip from its own axis depth and showing both numerals in the
  // shared corner when either is set.
  // We reserve `depth + 1` tracks: one per bracket level plus a final
  // track at the inner edge for the level-(N+1) corner numeral (which
  // Excel uses for "expand all"). Outline depth 1 → 2 tracks → 28px.
  const rowGutterW =
    rowOutlineDepth > 0 ? OUTLINE_GUTTER_PAD * 2 + (rowOutlineDepth + 1) * OUTLINE_GUTTER_STEP : 0;
  const colGutterH =
    colOutlineDepth > 0 ? OUTLINE_GUTTER_PAD * 2 + (colOutlineDepth + 1) * OUTLINE_GUTTER_STEP : 0;
  const originX = HEADER_W + rowGutterW;
  const originY = HEADER_H + colGutterH;

  const colW: number[] = [0];
  const colX: number[] = [0, originX];
  for (let c = 1; c <= maxCol; c++) {
    const w = widthOf(c);
    colW[c] = w;
    colX[c + 1] = (colX[c] ?? originX) + w;
  }
  while (
    requiredFarX !== undefined &&
    maxCol < SHEET_MAX_COL &&
    (colX[maxCol + 1] ?? originX) < requiredFarX
  ) {
    maxCol++;
    const w = widthOf(maxCol);
    colW[maxCol] = w;
    colX[maxCol + 1] = (colX[maxCol] ?? originX) + w;
  }

  const rowH: number[] = [0];
  const rowY: number[] = [0, originY];
  for (let r = 1; r <= maxRow; r++) {
    const h = heightOf(r);
    rowH[r] = h;
    rowY[r + 1] = (rowY[r] ?? originY) + h;
  }
  while (
    requiredFarY !== undefined &&
    maxRow < SHEET_MAX_ROW &&
    (rowY[maxRow + 1] ?? originY) < requiredFarY
  ) {
    maxRow++;
    const h = heightOf(maxRow);
    rowH[maxRow] = h;
    rowY[maxRow + 1] = (rowY[maxRow] ?? originY) + h;
  }

  return {
    colX,
    colW,
    rowY,
    rowH,
    totalW: colX[maxCol + 1] ?? originX,
    totalH: rowY[maxRow + 1] ?? originY,
    maxCol,
    maxRow,
    rowGutterW,
    colGutterH,
    originX,
    originY,
    rowOutlineDepth,
    colOutlineDepth,
  };
}

export function colLabel(n: number): string {
  let s = "";
  while (n > 0) {
    const r = (n - 1) % 26;
    s = String.fromCharCode(65 + r) + s;
    n = Math.floor((n - 1) / 26);
  }
  return s;
}

const PX_PER_EMU = 1 / 9525;

export function anchorToRect(
  d: Drawing,
  g: Grid,
): { x: number; y: number; w: number; h: number } | null {
  const a = d.anchor;
  const fromX = colEdge(g, a.fromCol + 1) + a.fromColOffEmu * PX_PER_EMU;
  const fromY = rowEdge(g, a.fromRow + 1) + a.fromRowOffEmu * PX_PER_EMU;
  // `oneCellAnchor` (and any anchor that ships an explicit extent)
  // wins over the resolved `to_*` cell — the producer measured the
  // size in EMU and we render to that exact pixel size.
  const toX =
    a.extEmuCx != null && a.extEmuCx > 0
      ? fromX + a.extEmuCx * PX_PER_EMU
      : colEdge(g, a.toCol + 1) + a.toColOffEmu * PX_PER_EMU;
  const toY =
    a.extEmuCy != null && a.extEmuCy > 0
      ? fromY + a.extEmuCy * PX_PER_EMU
      : rowEdge(g, a.toRow + 1) + a.toRowOffEmu * PX_PER_EMU;
  const w = toX - fromX;
  const h = toY - fromY;
  if (w <= 1 || h <= 1) return null;
  return { x: fromX, y: fromY, w, h };
}

function colEdge(g: Grid, c: number): number {
  if (c >= 1 && c < g.colX.length) return g.colX[c] ?? g.originX;
  const lastIdx = g.colX.length - 1;
  const last = g.colX[lastIdx] ?? g.originX;
  const prev = g.colX[lastIdx - 1] ?? g.originX;
  const w = Math.max(40, last - prev);
  return last + (c - lastIdx) * w;
}

function rowEdge(g: Grid, r: number): number {
  if (r >= 1 && r < g.rowY.length) return g.rowY[r] ?? g.originY;
  const lastIdx = g.rowY.length - 1;
  const last = g.rowY[lastIdx] ?? g.originY;
  const prev = g.rowY[lastIdx - 1] ?? g.originY;
  const h = Math.max(20, last - prev);
  return last + (r - lastIdx) * h;
}
