import init, { WorkbookHandle as WasmWorkbookHandle } from "./xlcore_wasm.js";
import type {
  ApiCellValue,
  CellInfo,
  LayoutOptions as WorkbookLayoutOptions,
  MergeInfo,
  RangeInfo,
  RecalcWorkbook,
  SheetInfo,
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
  EngineCellValue,
  FillPatch,
  FontPatch,
  FormulaFallback,
  HorizontalAlign,
  LayoutOptions as WorkbookLayoutOptions,
  MergeInfo,
  RangeInfo,
  RecalcCell,
  RecalcSheet,
  RecalcWorkbook,
  SheetInfo,
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

  clear(reference: string): CellInfo {
    return this.handle.clear(reference) as CellInfo;
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

  clearRange(reference: string): RangeInfo {
    return this.handle.clearRange(reference) as RangeInfo;
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
