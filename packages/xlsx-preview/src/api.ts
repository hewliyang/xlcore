import init, { WorkbookHandle as WasmWorkbookHandle } from "./xlcore_wasm.js";
import type { WorkbookLayout } from "./types.js";

const DEFAULT_WASM_BINARY_URL = new URL("./xlcore_wasm_bg.wasm", import.meta.url).href;

let wasmReady: Promise<void> | null = null;

export interface WorkbookApiOptions {
  wasmBinaryUrl?: string | URL | RequestInfo | BufferSource | WebAssembly.Module;
}

export interface WorkbookLayoutOptions {
  sheetIndex?: number;
  sheetName?: string;
}

export interface SheetInfo {
  index: number;
  id: number;
  name: string;
  state?: "hidden" | "veryHidden";
  rowCount: number;
  columnCount: number;
  active: boolean;
}

export type ApiCellValue =
  | { type: "blank" }
  | { type: "string"; value: string }
  | { type: "number"; value: number }
  | { type: "boolean"; value: boolean }
  | { type: "error"; value: string };

export type CellInput = string | number | boolean | null | ApiCellValue;

export interface CellInfo {
  sheet: string;
  reference: string;
  row: number;
  column: number;
  value: ApiCellValue;
  formula?: string;
  styleIndex?: number;
}

export interface ApiErrorPayload {
  code: string;
  message: string;
  sheet?: string;
  reference?: string;
  part?: string;
}

export interface RecalcWorkbook {
  sheets: RecalcSheet[];
}

export interface RecalcSheet {
  index: number;
  name: string;
  cells: RecalcCell[];
}

export interface RecalcCell {
  r: number;
  c: number;
  formula: string;
  cachedValue?: ApiCellValue;
  value: ApiCellValue;
  fallback?: {
    kind: string;
    message: string;
  };
}

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
