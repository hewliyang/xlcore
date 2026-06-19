import { Workbook } from "./api.js";
import type {
  ChartAnchor,
  ClearMode,
  DependencyReference,
  ImagePatch,
  PivotUpdate,
  LayoutOptions as WorkbookLayoutOptions,
} from "./api-schema/index.js";
import type { CellInput } from "./api-range.js";
import type {
  EditWorkerOp,
  EditWorkerRequest,
  EditWorkerResponse,
  PivotMeta,
  WorkbookStructure,
} from "./editWorker.js";
import { XlsxLoadError } from "./errors.js";
import type { PreviewerEngine } from "./previewer.js";
import type { WorkbookLayout } from "./types.js";

const DEFAULT_WASM_BINARY_URL = new URL("./xlcore_wasm_bg.wasm", import.meta.url).href;

export interface WorkerWorkbookOpenOptions {
  wasmBinaryUrl?: string;
  workerUrl?: string;
}

export interface ApplyEditInput {
  sheetName: string;
  address: string;
  input: string;
  recalc: boolean;
}

interface OpenResult {
  layout: WorkbookLayout;
  structure: WorkbookStructure;
}

interface EditResult {
  layout: WorkbookLayout;
  structure: WorkbookStructure;
}

export class WorkerWorkbook {
  static async open(
    bytes: ArrayBuffer | Uint8Array,
    options: WorkerWorkbookOpenOptions = {},
  ): Promise<WorkerWorkbook> {
    const wasmBinaryUrl = options.wasmBinaryUrl ?? DEFAULT_WASM_BINARY_URL;
    const worker = createEditWorker(options);
    const proxy = new WorkerWorkbook(worker, wasmBinaryUrl);
    const buffer = toArrayBuffer(bytes);
    const { structure } = await proxy.request<OpenResult>(
      "open",
      { bytes: buffer, wasmBinaryUrl },
      [buffer],
    );
    await proxy.syncShadow(structure);
    return proxy;
  }

  private nextId = 0;
  private readonly pending = new Map<
    number,
    { resolve: (value: unknown) => void; reject: (error: unknown) => void }
  >();
  private shadow: Workbook | null = null;
  private shadowSnapshot = "";
  private cachedFunctionNames: string[] | null = null;

  private constructor(
    private readonly worker: Worker,
    private readonly wasmBinaryUrl: string,
  ) {
    this.worker.onmessage = (event: MessageEvent) => {
      const message = event.data as EditWorkerResponse;
      const entry = this.pending.get(message.id);
      if (!entry) return;
      this.pending.delete(message.id);
      if (message.ok) {
        entry.resolve(message.result);
      } else {
        entry.reject(new XlsxLoadError(message.error));
      }
    };
    this.worker.onerror = (event: ErrorEvent) => {
      const error = new XlsxLoadError({
        code: "Other",
        message: event.message || "Edit worker failed",
      });
      for (const entry of this.pending.values()) {
        entry.reject(error);
      }
      this.pending.clear();
    };
  }

  async applyEdit(input: ApplyEditInput): Promise<{ layout: WorkbookLayout }> {
    const { layout, structure } = await this.request<EditResult>("applyEdit", { ...input });
    await this.syncShadow(structure);
    return { layout };
  }

  async setRangeValues(input: {
    sheetName: string;
    ref: string;
    values: CellInput[][];
    recalc: boolean;
  }): Promise<{ layout: WorkbookLayout }> {
    const { layout, structure } = await this.request<EditResult>("setRangeValues", { ...input });
    await this.syncShadow(structure);
    return { layout };
  }

  async pasteCells(input: {
    sheetName: string;
    row: number;
    column: number;
    values: string[][];
    recalc: boolean;
  }): Promise<{ layout: WorkbookLayout }> {
    const { layout, structure } = await this.request<EditResult>("pasteCells", { ...input });
    await this.syncShadow(structure);
    return { layout };
  }

  async setRangeFormulas(input: {
    sheetName: string;
    ref: string;
    formulas: Array<Array<string | null>>;
    recalc: boolean;
  }): Promise<{ layout: WorkbookLayout }> {
    const { layout, structure } = await this.request<EditResult>("setRangeFormulas", { ...input });
    await this.syncShadow(structure);
    return { layout };
  }

  async copyRange(input: {
    sheetName: string;
    ref: string;
    destSheet: string;
    destRef: string;
    recalc: boolean;
  }): Promise<{ layout: WorkbookLayout }> {
    const { layout, structure } = await this.request<EditResult>("copyRange", { ...input });
    await this.syncShadow(structure);
    return { layout };
  }

  async moveRange(input: {
    sheetName: string;
    ref: string;
    destSheet: string;
    destRef: string;
    recalc: boolean;
  }): Promise<{ layout: WorkbookLayout }> {
    const { layout, structure } = await this.request<EditResult>("moveRange", { ...input });
    await this.syncShadow(structure);
    return { layout };
  }

  async clearRange(input: {
    sheetName: string;
    ref: string;
    mode?: ClearMode;
    recalc: boolean;
  }): Promise<{ layout: WorkbookLayout }> {
    const { layout, structure } = await this.request<EditResult>("clearRange", { ...input });
    await this.syncShadow(structure);
    return { layout };
  }

  async setImage(sheetName: string, patch: ImagePatch): Promise<{ layout: WorkbookLayout }> {
    const { bytes, ...rest } = patch;
    const source = bytes instanceof Uint8Array ? bytes : Uint8Array.from(bytes);
    const copy = source.slice();
    const buffer = copy.buffer;
    const { layout, structure } = await this.request<EditResult>(
      "setImage",
      { sheetName, patch: rest, bytes: buffer },
      [buffer],
    );
    await this.syncShadow(structure);
    return { layout };
  }

  async addSheet(name: string): Promise<{ layout: WorkbookLayout; name: string }> {
    const { layout, structure, name: created } = await this.request<EditResult & { name: string }>(
      "addSheet",
      { name },
    );
    await this.syncShadow(structure);
    return { layout, name: created };
  }

  async recalculate(): Promise<WorkbookLayout> {
    const { layout, structure } = await this.request<EditResult>("recalculate", {});
    await this.syncShadow(structure);
    return layout;
  }

  async layout(options: WorkbookLayoutOptions = {}): Promise<WorkbookLayout> {
    const { layout } = await this.request<{ layout: WorkbookLayout }>("layout", { options });
    return layout;
  }

  async save(): Promise<Uint8Array> {
    const { bytes } = await this.request<{ bytes: Uint8Array }>("save", {});
    return bytes;
  }

  async pivotMetas(): Promise<PivotMeta[]> {
    const { pivots } = await this.request<{ pivots: PivotMeta[] }>("pivotMetas", {});
    return pivots;
  }

  async distinctValues(sourceRef: string, field: string): Promise<string[]> {
    const { values } = await this.request<{ values: string[] }>("distinctValues", {
      sourceRef,
      field,
    });
    return values;
  }

  async updatePivot(sheet: string, id: string, patch: PivotUpdate): Promise<WorkbookLayout> {
    const { layout, structure } = await this.request<EditResult>("updatePivot", {
      sheet,
      id,
      patch,
    });
    await this.syncShadow(structure);
    return layout;
  }

  async moveDrawing(input: {
    sheetName: string;
    kind: string;
    drawingIndex: number;
    anchor: ChartAnchor;
    prevAnchor: ChartAnchor;
  }): Promise<WorkbookLayout> {
    const { layout, structure } = await this.request<EditResult>("moveDrawing", { ...input });
    await this.syncShadow(structure);
    return layout;
  }

  async removeDrawing(input: {
    sheetName: string;
    kind: string;
    drawingIndex: number;
    prevAnchor: ChartAnchor;
  }): Promise<WorkbookLayout> {
    const { layout, structure } = await this.request<EditResult>("removeDrawing", { ...input });
    await this.syncShadow(structure);
    return layout;
  }

  async tableSetFilter(input: {
    rangeRef: string;
    columnOffset: number;
    field: string;
    values: string[];
  }): Promise<WorkbookLayout> {
    const { layout, structure } = await this.request<EditResult>("tableSetFilter", { ...input });
    await this.syncShadow(structure);
    return layout;
  }

  async tableSetSort(input: {
    rangeRef: string;
    columnOffset: number;
    descending: boolean | null;
  }): Promise<WorkbookLayout> {
    const { layout, structure } = await this.request<EditResult>("tableSetSort", { ...input });
    await this.syncShadow(structure);
    return layout;
  }

  get engine(): PreviewerEngine {
    return {
      parseReferences: (
        sheetName: string,
        anchorRef: string,
        formula: string,
      ): DependencyReference[] => {
        if (!this.shadow) return [];
        return this.shadow.parseFormulaReferences(sheetName, anchorRef, formula);
      },
      functionNames: (): string[] => {
        if (!this.shadow) return [];
        this.cachedFunctionNames ??= this.shadow.functionNames();
        return this.cachedFunctionNames;
      },
    };
  }

  dispose(): void {
    this.worker.terminate();
    this.shadow?.dispose();
    this.shadow = null;
    this.pending.clear();
  }

  private request<T>(
    op: EditWorkerOp,
    args: Record<string, unknown>,
    transfer?: Transferable[],
  ): Promise<T> {
    const id = this.nextId++;
    const message: EditWorkerRequest = { id, op, args };
    return new Promise<T>((resolve, reject) => {
      this.pending.set(id, {
        resolve: resolve as (value: unknown) => void,
        reject,
      });
      this.worker.postMessage(message, transfer ?? []);
    });
  }

  private async syncShadow(structure: WorkbookStructure): Promise<void> {
    const snapshot = JSON.stringify(structure);
    if (this.shadow && snapshot === this.shadowSnapshot) return;
    this.shadow?.dispose();
    this.cachedFunctionNames = null;
    const shadow = await Workbook.create({ wasmBinaryUrl: this.wasmBinaryUrl });
    const wanted = new Set(structure.sheets);
    for (const name of structure.sheets) {
      if (!shadow.worksheets().some((ws) => ws.name === name)) {
        shadow.addSheet(name);
      }
    }
    for (const ws of shadow.worksheets()) {
      if (!wanted.has(ws.name)) {
        shadow.removeSheet(ws.name);
      }
    }
    for (const info of structure.definedNames) {
      try {
        shadow.definedNames.set({
          name: info.name,
          reference: info.reference,
          scope: info.scope,
          comment: info.comment,
          hidden: info.hidden,
        });
      } catch {}
    }
    this.shadow = shadow;
    this.shadowSnapshot = snapshot;
  }
}

function createEditWorker(options: WorkerWorkbookOpenOptions): Worker {
  if (!options.workerUrl) {
    return new Worker(new URL("./editWorker.js", import.meta.url), { type: "module" });
  }
  const url = new URL(options.workerUrl, location.href).href;
  if (isCrossOrigin(url)) {
    const shim = `import ${JSON.stringify(url)};`;
    const blobUrl = URL.createObjectURL(new Blob([shim], { type: "text/javascript" }));
    const worker = new Worker(blobUrl, { type: "module" });
    queueMicrotask(() => URL.revokeObjectURL(blobUrl));
    return worker;
  }
  return new Worker(url, { type: "module" });
}

function isCrossOrigin(url: string): boolean {
  if (typeof location === "undefined") return false;
  try {
    return new URL(url, location.href).origin !== location.origin;
  } catch {
    return false;
  }
}

function toArrayBuffer(bytes: ArrayBuffer | Uint8Array): ArrayBuffer {
  if (bytes instanceof Uint8Array) {
    const copy = bytes.slice();
    return copy.buffer;
  }
  return bytes.slice(0);
}
