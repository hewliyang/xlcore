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

