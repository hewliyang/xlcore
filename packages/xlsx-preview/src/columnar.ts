import type { Sheet as WireSheet } from "./schema/Sheet.js";
import type { Cell, TextRun, WorkbookLayout } from "./types.js";
import type { Sheet } from "./types.js";

export interface DecodedCells {
  count: number;
  r: Uint32Array;
  c: Uint32Array;
  kind: Uint8Array;
  valueIdx: Int32Array;
  formulaIdx: Int32Array;
  styleIdx: Int32Array;
  runsIdx: Int32Array;

  rowPtr: Uint32Array;
}

export interface DecodedRowMeta {
  count: number;
  index: Uint32Array;
  heightPx: Float32Array;
  styleIdx: Int32Array;
  hidden: Uint8Array;

  outlineLevel: Uint8Array;

  byIndex: Map<number, number>;
}

const KIND_NAMES: readonly string[] = ["n", "s", "inline", "b", "e", "str", "f"];

const DECODED = Symbol.for("xlcore.columnar.decoded");
export function decodeWorkbookLayout(layout: WorkbookLayout): WorkbookLayout {
  for (const wire of layout.sheets) {
    const sheet = wire as unknown as Sheet;
    const tagged = sheet as any;
    if (tagged[DECODED]) continue;
    decodeSheet(sheet);
    tagged[DECODED] = true;
  }
  return layout;
}

function decodeSheet(sheet: Sheet): void {
  const wire = sheet as unknown as WireSheet;
  const c = wire.cells;
  sheet.decodedCells = {
    count: c.count,
    r: decodeU32(c.r),
    c: decodeU32(c.c),
    kind: decodeU8(c.kind),
    valueIdx: decodeI32(c.valueIdx),
    formulaIdx: decodeI32(c.formulaIdx),
    styleIdx: decodeI32(c.styleIdx),
    runsIdx: decodeI32(c.runsIdx),
    rowPtr: decodeU32(c.rowPtr),
  };
  const m = wire.rowMeta;
  const index = decodeU32(m.index);
  const byIndex = new Map<number, number>();
  for (let i = 0; i < m.count; i++) byIndex.set(index[i] ?? 0, i);

  const outlineLevelB64 = (m as unknown as { outlineLevel?: string }).outlineLevel ?? "";
  const outlineLevel = outlineLevelB64 ? decodeU8(outlineLevelB64) : new Uint8Array(0);
  sheet.decodedRowMeta = {
    count: m.count,
    index,
    heightPx: decodeF32(m.heightPx),
    styleIdx: decodeI32(m.styleIdx),
    hidden: decodeU8(m.hidden),
    outlineLevel,
    byIndex,
  };

  (sheet as any).cells = undefined;
  (sheet as any).rowMeta = undefined;
}

function decodeBytes(b64: string): Uint8Array {
  const bin = atob(b64);
  const out = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
  return out;
}
function decodeU8(b64: string): Uint8Array {
  return decodeBytes(b64);
}
function decodeU32(b64: string): Uint32Array {
  const bytes = decodeBytes(b64);

  const aligned = new ArrayBuffer(bytes.byteLength);
  new Uint8Array(aligned).set(bytes);
  return new Uint32Array(aligned);
}
function decodeI32(b64: string): Int32Array {
  const bytes = decodeBytes(b64);
  const aligned = new ArrayBuffer(bytes.byteLength);
  new Uint8Array(aligned).set(bytes);
  return new Int32Array(aligned);
}
function decodeF32(b64: string): Float32Array {
  const bytes = decodeBytes(b64);
  const aligned = new ArrayBuffer(bytes.byteLength);
  new Uint8Array(aligned).set(bytes);
  return new Float32Array(aligned);
}

export function materializeCell(sheet: Sheet, i: number): Cell {
  const cells = sheet.decodedCells;
  const valueIdx = cells.valueIdx[i] ?? -1;
  const formulaIdx = cells.formulaIdx[i] ?? -1;
  const styleIdx = cells.styleIdx[i] ?? -1;
  const runsIdx = cells.runsIdx[i] ?? -1;
  const value = valueIdx >= 0 ? sheet.valuePool[valueIdx] : undefined;
  const formula = formulaIdx >= 0 ? sheet.formulaPool[formulaIdx] : undefined;
  const runs: TextRun[] = runsIdx >= 0 ? (sheet.inlineRuns[runsIdx] ?? []) : [];
  return {
    r: cells.r[i] ?? 0,
    c: cells.c[i] ?? 0,
    type: KIND_NAMES[cells.kind[i] ?? 0] ?? "n",
    value,
    formula,
    styleIndex: styleIdx >= 0 ? styleIdx : undefined,
    runs,
  };
}

export function iterCellsInRange(
  sheet: Sheet,
  firstRow: number,
  lastRow: number,
  firstCol: number,
  lastCol: number,
  fn: (cell: Cell, i: number) => void,
): void {
  const meta = sheet.decodedRowMeta;
  const cells = sheet.decodedCells;
  if (meta.count === 0 || cells.count === 0) return;
  if (firstRow > lastRow || firstCol > lastCol) return;
  const startMeta = lowerBound(meta.index, firstRow, 0, meta.count);
  for (let m = startMeta; m < meta.count; m++) {
    const rowIdx = meta.index[m] ?? 0;
    if (rowIdx > lastRow) break;
    const start = cells.rowPtr[m] ?? 0;
    const end = cells.rowPtr[m + 1] ?? cells.count;
    if (start === end) continue;
    let i = lowerBound(cells.c, firstCol, start, end);
    for (; i < end; i++) {
      const col = cells.c[i] ?? 0;
      if (col > lastCol) break;
      fn(materializeCell(sheet, i), i);
    }
  }
}

export function iterAllCells(sheet: Sheet, fn: (cell: Cell, i: number) => void): void {
  const cells = sheet.decodedCells;
  for (let i = 0; i < cells.count; i++) fn(materializeCell(sheet, i), i);
}

export function iterRows(
  sheet: Sheet,
  fn: (row: {
    index: number;
    heightPx: number | undefined;
    styleIndex: number | undefined;
    hidden: boolean;
  }) => void,
): void {
  const meta = sheet.decodedRowMeta;
  for (let i = 0; i < meta.count; i++) {
    const h = meta.heightPx[i] ?? Number.NaN;
    const s = meta.styleIdx[i] ?? -1;
    fn({
      index: meta.index[i] ?? 0,
      heightPx: Number.isNaN(h) ? undefined : h,
      styleIndex: s >= 0 ? s : undefined,
      hidden: (meta.hidden[i] ?? 0) !== 0,
    });
  }
}

export function findCell(sheet: Sheet, r: number, c: number): Cell | undefined {
  const meta = sheet.decodedRowMeta;
  const m = meta.byIndex.get(r);
  if (m === undefined) return undefined;
  const cells = sheet.decodedCells;
  const start = cells.rowPtr[m] ?? 0;
  const end = cells.rowPtr[m + 1] ?? cells.count;
  let lo = start;
  let hi = end - 1;
  while (lo <= hi) {
    const mid = (lo + hi) >> 1;
    const col = cells.c[mid] ?? 0;
    if (col === c) return materializeCell(sheet, mid);
    if (col < c) lo = mid + 1;
    else hi = mid - 1;
  }
  return undefined;
}

function lowerBound(arr: { [i: number]: number }, target: number, lo: number, hi: number): number {
  while (lo < hi) {
    const mid = (lo + hi) >> 1;
    if ((arr[mid] ?? 0) < target) lo = mid + 1;
    else hi = mid;
  }
  return lo;
}
