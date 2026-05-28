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
import type { WorkbookLayout } from "./types.js";

export interface WorkbookLoadProgress {
  label: string;
}

export interface WorkbookLoaderOptions {
  wasmBinaryUrl?: string;

  workerUrl?: string;
  onProgress?: (progress: WorkbookLoadProgress) => void;
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
  return loadWorkbookFromArrayBufferWithReport(bytes, options);
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
  const worker = createExtractionWorker(options);
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
        bytes,
        wasmBinaryUrl: options.wasmBinaryUrl ?? DEFAULT_WASM_BINARY_URL,
      },
      [bytes],
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
