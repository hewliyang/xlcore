// Columnar cell-storage decoder + iteration helpers.
//
// The Rust extractor ships per-sheet cells as base64-encoded
// little-endian typed-array blobs (see
// `crates/xlcore-export/src/columnar.rs`). We decode those blobs
// once at workbook-load time and cache the typed-array views on the
// `Sheet` object. Hot paint loops then iterate by index over the
// columns directly; random-access lookups go through `findCell`,
// which materializes a `Cell` POJO using the per-sheet string pools.
//
// Why this shape:
//   * Wire is ~2× smaller after gzip than the old per-cell JSON, but
//     the bigger win is parse-time and heap. Decoding 7 typed arrays
//     per sheet is one structured copy each instead of materializing
//     millions of tiny `{r, c, type, value, styleIndex, runs}` objects.
//   * Random access stays cheap: rows are sorted by index, cells are
//     sorted by col within row, and `rowPtr` lets us binary-search a
//     row in O(log rowCount) and a col in O(log cellsInRow).
//   * The renderer's existing `cell.r/c/type/value/styleIndex/runs`
//     access pattern is preserved by `materializeCell` — only the
//     iteration entry points (`for (const row of sheet.rows) ...`)
//     change. Callbacks still see normal POJOs.

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
  /// Length = decodedRowMeta.count + 1. Cells for rowMeta.index[i]
  /// live in [rowPtr[i], rowPtr[i+1]).
  rowPtr: Uint32Array;
}

export interface DecodedRowMeta {
  count: number;
  index: Uint32Array;
  heightPx: Float32Array; // NaN ⇒ default height
  styleIdx: Int32Array;   // -1 ⇒ no row-level style
  hidden: Uint8Array;
  /// OOXML `<row outlineLevel="N">`, 0..=7. Length == count when any
  /// row is grouped, otherwise length 0 (treat as all-zeros). Cheap
  /// to test before the per-row paint loop bothers to look.
  outlineLevel: Uint8Array;
  /// rowIndex (1-based) → position in the meta arrays. Built once at
  /// decode time so `findCell` doesn't have to binary-search rowMeta
  /// repeatedly. Sheets with millions of rows still fit comfortably
  /// — Map handles 10M-entry workloads at sub-µs lookups.
  byIndex: Map<number, number>;
}

// The runtime shape lives in `./types.ts` as an interface that extends
// the auto-generated wire type with these decoded fields. We can't
// module-merge the wire type directly because ts-rs emits `type`
// aliases (not interfaces).

// Cell-kind enum byte → wire string (matches `Cell.type`).
const KIND_NAMES: readonly string[] = ["n", "s", "inline", "b", "e", "str", "f"];

/// Decode every sheet's columnar blobs in-place. Idempotent: callable
/// twice with no effect (we tag decoded sheets with a private symbol).
const DECODED = Symbol.for("xlcore.columnar.decoded");
export function decodeWorkbookLayout(layout: WorkbookLayout): WorkbookLayout {
  for (const wire of layout.sheets) {
    // The runtime fields don't exist yet on the wire object; decodeSheet
    // mutates `wire` in place to add them, after which it satisfies the
    // augmented `Sheet` interface from `./types.js`.
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
  // outlineLevel is omitted from the wire when every row is at level 0.
  // We synthesize an empty Uint8Array in that case so callers don't need
  // a length check (zero-length view returns `undefined` on indexed access,
  // which our `?? 0` fallbacks below already handle).
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
  // Free the b64 strings — keeping them around doubles per-sheet
  // memory for nothing. The wire fields are unused after decode.
  (sheet as any).cells = undefined;
  (sheet as any).rowMeta = undefined;
}

// ----------------------------------------------------------------
// base64 → typed array. We avoid Buffer for browser portability and
// use `atob` plus a single-pass byte copy, which is fastest in V8/
// JSC for blobs in this size range. The Uint*Array constructors
// then alias the byte buffer with no copy.
// ----------------------------------------------------------------
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
  // Typed-array byte alignment: `atob` outputs may land on any
  // address. Copy into a fresh buffer to guarantee 4-byte alignment.
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

// ----------------------------------------------------------------
// Public iteration + lookup API. These are what the renderer calls.
// ----------------------------------------------------------------

/// Materialize the cell at column-index `i` (into `decodedCells`) as
/// a regular `Cell` POJO. Used by the per-frame paint loop and by
/// `findCell`. Allocation cost is one object + 0–2 string lookups
/// per visible cell; on a typical viewport this is well under 1 ms.
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

/// Iterate every cell whose (r,c) lies in
/// `[firstRow, lastRow] × [firstCol, lastCol]`, in row-major order.
/// Calls `fn(cell, i)` per match. `i` is the index into
/// `sheet.decodedCells.*` — handy when the caller wants to skip the
/// `materializeCell` allocation and read columns directly (e.g. to
/// pre-filter on `kind` before pulling the value out of the pool).
///
/// Performance notes:
///   * `lowerBoundRow` / `lowerBoundCol` are O(log) inside their
///     respective sorted ranges, so a typical viewport (~50 rows ×
///     ~30 cols) only touches the ~1500 cells it paints, not the
///     million-cell sheet.
///   * Returns early when row/col indices exceed the upper bound.
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

/// Iterate every cell on the sheet (row-major). Used by code paths
/// that build whole-sheet indexes (e.g. CF data-bar min/max scan).
export function iterAllCells(
  sheet: Sheet,
  fn: (cell: Cell, i: number) => void,
): void {
  const cells = sheet.decodedCells;
  for (let i = 0; i < cells.count; i++) fn(materializeCell(sheet, i), i);
}

/// Iterate row metadata (index, heightPx, styleIndex, hidden) in
/// ascending row-index order. Replacement for `for (const row of
/// sheet.rows)` in places that don't read cell contents.
export function iterRows(
  sheet: Sheet,
  fn: (row: { index: number; heightPx: number | undefined; styleIndex: number | undefined; hidden: boolean }) => void,
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

/// O(log rowCount + log cellsInRow) lookup. Returns `undefined` when
/// the row has no recorded cells or when (r,c) is empty.
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

/// First index `i` in `[lo, hi)` such that `arr[i] >= target`, or `hi`
/// if no such element. Standard branchless binary search.
function lowerBound(arr: { [i: number]: number }, target: number, lo: number, hi: number): number {
  while (lo < hi) {
    const mid = (lo + hi) >> 1;
    if ((arr[mid] ?? 0) < target) lo = mid + 1;
    else hi = mid;
  }
  return lo;
}
