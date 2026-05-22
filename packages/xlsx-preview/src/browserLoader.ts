import {
  EMPTY_LOAD_REPORT,
  type LoadReport,
  XlsxLoadError,
  type XlsxLoadErrorPayload,
  xlsxLoadErrorPayloadFromUnknown,
} from "./errors.js";
import {
  createWorkbookPreviewer,
  type PreviewerOptions,
  type WorkbookPreviewer,
} from "./previewer.js";
import { resolveWorkbookFormat, type WorkbookSourceFormat } from "./sourceFormat.js";
import type { WorkbookLayout } from "./types.js";

export interface WorkbookLoadProgress {
  label: string;
}

/** Subset of the Rust `CsvOptions` that's safe to expose to JS. */
export interface CsvLoadOptions {
  /** Single byte (`,`, `\t`, `;`, `|`) or the literal `"tab"`. */
  delimiter?: string;
  /** Rendered-row truncation cap; warning is reported via `LoadReport.warnings`. */
  maxRows?: number;
  /** Sheet name shown in the renderer's tab strip. */
  sheetName?: string;
}

/** Subset of the Rust `ParquetOptions` exposed to JS. */
export interface ParquetLoadOptions {
  /** Rendered-row truncation cap, including the synthetic header row. */
  maxRows?: number;
  sheetName?: string;
}

export interface WorkbookLoaderOptions {
  wasmBinaryUrl?: string;

  workerUrl?: string;
  onProgress?: (progress: WorkbookLoadProgress) => void;

  /** XLSX only: extract a single sheet by zero-based index. */
  sheetIndex?: number;

  /** XLSX only: extract a single sheet by name. Takes precedence over sheetIndex. */
  sheetName?: string;

  /**
   * Source format. `"auto"` (default) sniffs Parquet/XLSX byte signatures
   * first, then falls back to a `File`'s name/type when available.
   */
  format?: "auto" | WorkbookSourceFormat;

  /** Forwarded to the rust CSV reader when `format === "csv"`. */
  csvOptions?: CsvLoadOptions;

  /** Forwarded to the rust parquet reader when `format === "parquet"`. */
  parquetOptions?: ParquetLoadOptions;
}

export interface CreateWorkbookPreviewerFromFileOptions
  extends WorkbookLoaderOptions,
    PreviewerOptions {}

export interface LoadedWorkbook {
  layout: WorkbookLayout;
  report: LoadReport;
}

const DEFAULT_WASM_BINARY_URL = new URL("./xlcore_wasm_bg.wasm", import.meta.url).href;

export async function loadWorkbookFromFile(
  file: Blob,
  options: WorkbookLoaderOptions = {},
): Promise<WorkbookLayout> {
  return (await loadWorkbookFromFileWithReport(file, options)).layout;
}

export async function loadWorkbookFromFileWithReport(
  file: Blob,
  options: WorkbookLoaderOptions = {},
): Promise<LoadedWorkbook> {
  progress(options, "Reading file");
  const bytes = await file.arrayBuffer();
  const format = resolveWorkbookFormat(options.format, bytes, {
    fileName: (file as File).name,
    mimeType: file.type,
  });
  const resolved: WorkbookLoaderOptions = {
    ...options,
    format,
    csvOptions: {
      // Default the sheet tab name to the file's basename if the caller didn't.
      sheetName: defaultSheetName(file),
      ...options.csvOptions,
    },
    parquetOptions: {
      sheetName: defaultSheetName(file),
      ...options.parquetOptions,
    },
  };
  return loadWorkbookFromArrayBufferWithReport(bytes, resolved);
}

export async function loadWorkbookFromArrayBuffer(
  bytes: ArrayBuffer,
  options: WorkbookLoaderOptions = {},
): Promise<WorkbookLayout> {
  return (await loadWorkbookFromArrayBufferWithReport(bytes, options)).layout;
}

export async function loadWorkbookFromArrayBufferWithReport(
  bytes: ArrayBuffer,
  options: WorkbookLoaderOptions = {},
): Promise<LoadedWorkbook> {
  const format = resolveWorkbookFormat(options.format, bytes);
  const resolvedOptions = { ...options, format };
  const worker = createExtractionWorker(options);
  const workerBytes = bytes.slice(0);
  return await new Promise((resolve, reject) => {
    worker.onmessage = (event) => {
      const message = event.data as
        | { type: "stage"; label: string }
        | { type: "loaded"; layout: WorkbookLayout; report: LoadReport }
        | { type: "error"; payload: XlsxLoadErrorPayload };
      if (message.type === "stage") {
        progress(options, message.label);
      } else if (message.type === "loaded") {
        worker.terminate();
        resolve({
          layout: message.layout,
          report: message.report ?? EMPTY_LOAD_REPORT,
        });
      } else if (message.type === "error") {
        worker.terminate();
        reject(new XlsxLoadError(xlsxLoadErrorPayloadFromUnknown(message.payload)));
      }
    };
    worker.onerror = (event) => {
      worker.terminate();
      reject(
        new XlsxLoadError({
          code: "Other",
          message: event.message || "Workbook worker failed",
        }),
      );
    };
    worker.postMessage(
      {
        bytes: workerBytes,
        wasmBinaryUrl: options.wasmBinaryUrl ?? DEFAULT_WASM_BINARY_URL,
        format,
        xlsxOptions:
          format === "xlsx"
            ? { sheetIndex: options.sheetIndex, sheetName: options.sheetName }
            : undefined,
        csvOptions: format === "csv" ? resolvedOptions.csvOptions : undefined,
        parquetOptions: format === "parquet" ? resolvedOptions.parquetOptions : undefined,
      },
      [workerBytes],
    );
  });
}

export async function createWorkbookPreviewerFromFile(
  container: HTMLElement,
  file: Blob,
  options: CreateWorkbookPreviewerFromFileOptions = {},
): Promise<WorkbookPreviewer> {
  const { layout, report } = await loadWorkbookFromFileWithReport(file, options);
  progress(options, "Preparing preview");
  const previewer = createWorkbookPreviewer(container, layout, { ...options, report });
  progress(options, "Rendering canvas");
  return previewer;
}

function createExtractionWorker(options: WorkbookLoaderOptions): Worker {
  if (!options.workerUrl) {
    return new Worker(new URL("./xlsxWorker.js", import.meta.url), {
      type: "module",
    });
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

function progress(options: WorkbookLoaderOptions, label: string): void {
  options.onProgress?.({ label });
}

function defaultSheetName(file: Blob): string {
  const name = (file as File).name ?? "";
  const base = name.replace(/\.[^./\\]+$/, "").trim();
  return base || "data";
}
