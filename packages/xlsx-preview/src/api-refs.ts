import type { ChartAnchor } from "./api-schema/index.js";

const SAFE_SHEET_NAME = /^[A-Za-z_][A-Za-z0-9_]*$/;

export function quoteSheetName(name: string): string {
  if (SAFE_SHEET_NAME.test(name)) return name;
  return `'${name.replace(/'/g, "''")}'`;
}

export function qualify(sheet: string, ref: string): string {
  if (hasSheetPrefix(ref)) return ref;
  return `${quoteSheetName(sheet)}!${ref}`;
}

export function hasSheetPrefix(ref: string): boolean {
  let quoted = false;
  for (let i = 0; i < ref.length; i++) {
    const ch = ref[i];
    if (ch === "'") {
      if (quoted && ref[i + 1] === "'") {
        i++;
        continue;
      }
      quoted = !quoted;
    } else if (ch === "!" && !quoted) {
      return true;
    }
  }
  return false;
}

export function colLetter(col1: number): string {
  if (!Number.isInteger(col1) || col1 < 1) {
    throw new RangeError(`column must be a positive 1-based integer, got ${col1}`);
  }
  let n = col1;
  let out = "";
  while (n > 0) {
    n--;
    out = String.fromCharCode(65 + (n % 26)) + out;
    n = Math.floor(n / 26);
  }
  return out;
}

export function cellA1(row1: number, col1: number): string {
  if (!Number.isInteger(row1) || row1 < 1) {
    throw new RangeError(`row must be a positive 1-based integer, got ${row1}`);
  }
  return `${colLetter(col1)}${row1}`;
}

export function rangeA1(row1: number, col1: number, rowCount: number, colCount: number): string {
  if (!Number.isInteger(rowCount) || rowCount < 1) {
    throw new RangeError(`rowCount must be >= 1, got ${rowCount}`);
  }
  if (!Number.isInteger(colCount) || colCount < 1) {
    throw new RangeError(`colCount must be >= 1, got ${colCount}`);
  }
  const start = cellA1(row1, col1);
  if (rowCount === 1 && colCount === 1) return start;
  const end = cellA1(row1 + rowCount - 1, col1 + colCount - 1);
  return `${start}:${end}`;
}

export interface AnchorA1 {
  fromColumn: number;
  fromRow: number;
  toColumn: number;
  toRow: number;
}

export function anchorA1(range: string): AnchorA1 {
  const bare = range.includes("!") ? range.slice(range.lastIndexOf("!") + 1) : range;
  const parts = bare.split(":");
  if (parts.length !== 2) {
    throw new RangeError(`anchorA1: expected a two-cell range like "A1:E15", got "${range}"`);
  }
  const a = CELL_RE.exec(parts[0] ?? "");
  const b = CELL_RE.exec(parts[1] ?? "");
  if (!a || !b) {
    throw new RangeError(`anchorA1: expected a two-cell range like "A1:E15", got "${range}"`);
  }
  const c1 = colNum(a[1] ?? "") - 1;
  const c2 = colNum(b[1] ?? "") - 1;
  const r1 = parseInt(a[2] ?? "0", 10) - 1;
  const r2 = parseInt(b[2] ?? "0", 10) - 1;
  return {
    fromColumn: Math.min(c1, c2),
    fromRow: Math.min(r1, r2),
    toColumn: Math.max(c1, c2) + 1,
    toRow: Math.max(r1, r2) + 1,
  };
}

const EMU_PER_PIXEL = 9525;
const DEFAULT_COL_WIDTH_PX = 64;
const DEFAULT_ROW_HEIGHT_PX = 20;

export interface AbsoluteAnchorOptions {
  /** Uniform column width in px (default 64, Excel's default at 100% zoom). */
  colWidthPx?: number;
  /** Uniform row height in px (default 20). */
  rowHeightPx?: number;
}

/**
 * Convert an absolute pixel rect (sheet content space, A1's top-left = 0,0) into a
 * two-cell {@link ChartAnchor} with EMU offsets — the px → (col, row, offsetEMU)
 * derivation everyone otherwise hand-rolls against the default 64×20 px grid.
 *
 * Assumes a **uniform** grid: every column is `colWidthPx` wide and every row is
 * `rowHeightPx` tall (pass overrides for sheets with non-default but uniform
 * sizing). Offsets are always strictly inside their cell, so the result never
 * trips the engine's "offset exceeds the referenced cell" warning. For sheets
 * with per-column/row sizes, derive the anchor from the real layout instead.
 */
export function absoluteAnchor(
  x: number,
  y: number,
  w: number,
  h: number,
  options: AbsoluteAnchorOptions = {},
): ChartAnchor {
  if (!Number.isFinite(x) || x < 0) {
    throw new RangeError(`absoluteAnchor: x must be a non-negative number, got ${x}`);
  }
  if (!Number.isFinite(y) || y < 0) {
    throw new RangeError(`absoluteAnchor: y must be a non-negative number, got ${y}`);
  }
  if (!Number.isFinite(w) || w <= 0) {
    throw new RangeError(`absoluteAnchor: w must be a positive number, got ${w}`);
  }
  if (!Number.isFinite(h) || h <= 0) {
    throw new RangeError(`absoluteAnchor: h must be a positive number, got ${h}`);
  }
  const colW = options.colWidthPx ?? DEFAULT_COL_WIDTH_PX;
  const rowH = options.rowHeightPx ?? DEFAULT_ROW_HEIGHT_PX;
  if (!Number.isFinite(colW) || colW <= 0) {
    throw new RangeError(`absoluteAnchor: colWidthPx must be a positive number, got ${colW}`);
  }
  if (!Number.isFinite(rowH) || rowH <= 0) {
    throw new RangeError(`absoluteAnchor: rowHeightPx must be a positive number, got ${rowH}`);
  }
  const split = (px: number, size: number): { cell: number; offsetEmu: bigint } => {
    const cell = Math.floor(px / size);
    const offsetPx = px - cell * size;
    return { cell, offsetEmu: BigInt(Math.round(offsetPx * EMU_PER_PIXEL)) };
  };
  const fromCol = split(x, colW);
  const fromRow = split(y, rowH);
  const toCol = split(x + w, colW);
  const toRow = split(y + h, rowH);
  return {
    fromColumn: fromCol.cell,
    fromRow: fromRow.cell,
    toColumn: toCol.cell,
    toRow: toRow.cell,
    fromColumnOffsetEmu: fromCol.offsetEmu,
    fromRowOffsetEmu: fromRow.offsetEmu,
    toColumnOffsetEmu: toCol.offsetEmu,
    toRowOffsetEmu: toRow.offsetEmu,
  };
}

export type CellAddress = string | { row: number; column: number };
export type RangeAddress =
  | string
  | { row: number; column: number; rowCount?: number; columnCount?: number };

export function resolveCell(addr: CellAddress): string {
  if (typeof addr === "string") return addr;
  return cellA1(addr.row, addr.column);
}

export function resolveRange(addr: RangeAddress): string {
  if (typeof addr === "string") return addr;
  return rangeA1(addr.row, addr.column, addr.rowCount ?? 1, addr.columnCount ?? 1);
}

export interface SheetRef {
  current: string;
}

export interface RangeDims {
  rows: number | null;
  cols: number | null;
}

const CELL_RE = /^\$?([A-Za-z]+)\$?(\d+)$/;
const COL_RE = /^\$?([A-Za-z]+)$/;
const ROW_RE = /^\$?(\d+)$/;

function colNum(letters: string): number {
  let n = 0;
  const up = letters.toUpperCase();
  for (let i = 0; i < up.length; i++) {
    n = n * 26 + (up.charCodeAt(i) - 64);
  }
  return n;
}

export function rangeDims(ref: string): RangeDims | null {
  const bare = ref.includes("!") ? ref.slice(ref.lastIndexOf("!") + 1) : ref;
  const parts = bare.split(":");
  if (parts.length === 1) {
    const only = parts[0] ?? "";
    const m = CELL_RE.exec(only);
    if (!m) return null;
    return { rows: 1, cols: 1 };
  }
  if (parts.length !== 2) return null;
  const a = parts[0] ?? "";
  const b = parts[1] ?? "";
  const cellA = CELL_RE.exec(a);
  const cellB = CELL_RE.exec(b);
  if (cellA && cellB) {
    const r1 = parseInt(cellA[2] ?? "0", 10);
    const r2 = parseInt(cellB[2] ?? "0", 10);
    const c1 = colNum(cellA[1] ?? "");
    const c2 = colNum(cellB[1] ?? "");
    return { rows: Math.abs(r2 - r1) + 1, cols: Math.abs(c2 - c1) + 1 };
  }
  const colA = COL_RE.exec(a);
  const colB = COL_RE.exec(b);
  if (colA && colB) {
    return { rows: null, cols: Math.abs(colNum(colB[1] ?? "") - colNum(colA[1] ?? "")) + 1 };
  }
  const rowA = ROW_RE.exec(a);
  const rowB = ROW_RE.exec(b);
  if (rowA && rowB) {
    return {
      rows: Math.abs(parseInt(rowB[1] ?? "0", 10) - parseInt(rowA[1] ?? "0", 10)) + 1,
      cols: null,
    };
  }
  return null;
}

export function validateMatrixShape<T>(api: string, ref: string, matrix: T[][]): void {
  if (!Array.isArray(matrix)) {
    throw new TypeError(`${api}: expected 2-D array, got ${typeof matrix}`);
  }
  if (matrix.length === 0) {
    throw new RangeError(`${api}: matrix must have at least one row`);
  }
  const firstRow = matrix[0];
  if (!Array.isArray(firstRow)) {
    throw new TypeError(`${api}: expected 2-D array (row 0 is not an array)`);
  }
  const cols = firstRow.length;
  if (cols === 0) {
    throw new RangeError(`${api}: matrix rows must have at least one column`);
  }
  for (let r = 1; r < matrix.length; r++) {
    const row = matrix[r];
    if (!Array.isArray(row)) {
      throw new TypeError(`${api}: row ${r} is not an array`);
    }
    if (row.length !== cols) {
      throw new RangeError(
        `${api}: jagged matrix — row 0 has ${cols} cols, row ${r} has ${row.length}`,
      );
    }
  }
  const dims = rangeDims(ref);
  if (!dims) return;
  if (dims.rows !== null && dims.rows !== matrix.length) {
    throw new RangeError(`${api}: range ${ref} expects ${dims.rows} row(s), got ${matrix.length}`);
  }
  if (dims.cols !== null && dims.cols !== cols) {
    throw new RangeError(`${api}: range ${ref} expects ${dims.cols} column(s), got ${cols}`);
  }
}
