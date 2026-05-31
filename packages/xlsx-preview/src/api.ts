import init, { WorkbookHandle as WasmWorkbookHandle } from "./xlcore_wasm.js";
import type {
  ApiCellValue,
  CellInfo,
  ClearMode,
  FreezeInfo,
  LayoutOptions as WorkbookLayoutOptions,
  MergeInfo,
  RangeInfo,
  RecalcWorkbook,
  SheetInfo,
  SheetVisibility,
  StylePatch,
} from "./api-schema/index.js";
import type { WorkbookLayout } from "./types.js";

export type {
  AlignmentPatch,
  ApiCellValue,
  ApiError,
  ApiError as ApiErrorPayload,
  ApiErrorCode,
  BorderLinePatch,
  BorderLineStyle,
  BorderPatch,
  CellInfo,
  ClearMode,
  EngineCellValue,
  FillPatch,
  FontPatch,
  FormulaFallback,
  FreezeInfo,
  HorizontalAlign,
  LayoutOptions as WorkbookLayoutOptions,
  MergeInfo,
  RangeInfo,
  RecalcCell,
  RecalcSheet,
  RecalcWorkbook,
  SheetInfo,
  SheetVisibility,
  StylePatch,
  UnderlinePatch,
  VerticalAlign,
} from "./api-schema/index.js";

const DEFAULT_WASM_BINARY_URL = new URL("./xlcore_wasm_bg.wasm", import.meta.url).href;

let wasmReady: Promise<void> | null = null;

export interface WorkbookApiOptions {
  wasmBinaryUrl?: string | URL | RequestInfo | BufferSource | WebAssembly.Module;
}

export type CellInput = string | number | boolean | null | ApiCellValue;

export class Workbook {
  static async create(options: WorkbookApiOptions = {}): Promise<Workbook> {
    await ensureWasm(options);
    return new Workbook(new WasmWorkbookHandle());
  }

  static async open(
    bytes: ArrayBuffer | Uint8Array,
    options: WorkbookApiOptions = {},
  ): Promise<Workbook> {
    await ensureWasm(options);
    return new Workbook(WasmWorkbookHandle.open(toUint8Array(bytes)));
  }

  private constructor(private handle: WasmWorkbookHandle) {}

  sheets(): SheetInfo[] {
    return this.handle.sheets() as SheetInfo[];
  }

  getCell(reference: string): CellInfo {
    return this.handle.getCell(reference) as CellInfo;
  }

  setValue(reference: string, value: CellInput): CellInfo {
    return this.handle.setValue(reference, value) as CellInfo;
  }

  setFormula(reference: string, formula: string): CellInfo {
    return this.handle.setFormula(reference, formula) as CellInfo;
  }

  clear(reference: string, mode?: ClearMode): CellInfo {
    if (mode === undefined) {
      return this.handle.clear(reference) as CellInfo;
    }
    return this.handle.clearWith(reference, mode) as CellInfo;
  }

  getRange(reference: string): RangeInfo {
    return this.handle.getRange(reference) as RangeInfo;
  }

  setRangeValues(reference: string, values: CellInput[][]): RangeInfo {
    return this.handle.setRangeValues(reference, values) as RangeInfo;
  }

  setRangeFormulas(reference: string, formulas: Array<Array<string | null>>): RangeInfo {
    return this.handle.setRangeFormulas(reference, formulas) as RangeInfo;
  }

  setStyle(reference: string, patch: StylePatch): RangeInfo {
    return this.handle.setStyle(reference, patch) as RangeInfo;
  }

  clearRange(reference: string, mode?: ClearMode): RangeInfo {
    if (mode === undefined) {
      return this.handle.clearRange(reference) as RangeInfo;
    }
    return this.handle.clearRangeWith(reference, mode) as RangeInfo;
  }

  merges(sheet: string): MergeInfo[] {
    return this.handle.merges(sheet) as MergeInfo[];
  }

  addMerge(reference: string): MergeInfo {
    return this.handle.addMerge(reference) as MergeInfo;
  }

  removeMerge(reference: string): MergeInfo | null {
    return (this.handle.removeMerge(reference) as MergeInfo | null) ?? null;
  }

  createSheet(name: string): SheetInfo {
    return this.handle.createSheet(name) as SheetInfo;
  }

  renameSheet(oldName: string, newName: string): void {
    this.handle.renameSheet(oldName, newName);
  }

  deleteSheet(name: string): void {
    this.handle.deleteSheet(name);
  }

  moveSheet(name: string, toIndex: number): SheetInfo {
    return this.handle.moveSheet(name, toIndex) as SheetInfo;
  }

  setSheetVisibility(name: string, visibility: SheetVisibility): SheetInfo {
    return this.handle.setSheetVisibility(name, visibility) as SheetInfo;
  }

  setActiveSheet(name: string): SheetInfo {
    return this.handle.setActiveSheet(name) as SheetInfo;
  }

  setRowHeight(sheet: string, row: number, height: number): void {
    this.handle.setRowHeight(sheet, row, height);
  }

  setRowVisible(sheet: string, row: number, visible: boolean): void {
    this.handle.setRowVisible(sheet, row, visible);
  }

  setColumnWidth(sheet: string, column: number, width: number): void {
    this.handle.setColumnWidth(sheet, column, width);
  }

  setColumnVisible(sheet: string, column: number, visible: boolean): void {
    this.handle.setColumnVisible(sheet, column, visible);
  }

  insertRows(sheet: string, before: number, count: number): void {
    this.handle.insertRows(sheet, before, count);
  }

  deleteRows(sheet: string, start: number, count: number): void {
    this.handle.deleteRows(sheet, start, count);
  }

  insertColumns(sheet: string, before: number, count: number): void {
    this.handle.insertColumns(sheet, before, count);
  }

  deleteColumns(sheet: string, start: number, count: number): void {
    this.handle.deleteColumns(sheet, start, count);
  }

  setFreeze(sheet: string, frozenRows: number, frozenColumns: number): FreezeInfo {
    return this.handle.setFreeze(sheet, frozenRows, frozenColumns) as FreezeInfo;
  }

  getFreeze(sheet: string): FreezeInfo {
    return this.handle.getFreeze(sheet) as FreezeInfo;
  }

  recalculate(): RecalcWorkbook {
    return this.handle.recalculate() as RecalcWorkbook;
  }

  layout(options: WorkbookLayoutOptions = {}): WorkbookLayout {
    return this.handle.layout(options) as WorkbookLayout;
  }

  save(): Uint8Array {
    return this.handle.save();
  }

  dispose(): void {
    this.handle.dispose();
  }
}

async function ensureWasm(options: WorkbookApiOptions): Promise<void> {
  wasmReady ??= init({
    module_or_path: options.wasmBinaryUrl ?? DEFAULT_WASM_BINARY_URL,
  }).then(() => undefined);
  await wasmReady;
}

function toUint8Array(bytes: ArrayBuffer | Uint8Array): Uint8Array {
  if (bytes instanceof Uint8Array) {
    return bytes;
  }
  return new Uint8Array(bytes);
}
