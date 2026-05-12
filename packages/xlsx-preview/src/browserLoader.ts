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
  /** URL of `xlcore_wasm_bg.wasm`; defaults to the package asset. */
  wasmBinaryUrl?: string;
  /** URL of `xlsxWorker.js`; defaults to the package worker. */
  workerUrl?: string;
  onProgress?: (progress: WorkbookLoadProgress) => void;
}

export interface CreateWorkbookPreviewerFromFileOptions
  extends WorkbookLoaderOptions,
    PreviewerOptions {}

const DEFAULT_WASM_BINARY_URL = new URL("./xlcore_wasm_bg.wasm", import.meta.url).href;

export async function loadWorkbookFromFile(
  file: Blob,
  options: WorkbookLoaderOptions = {},
): Promise<WorkbookLayout> {
  progress(options, "Reading file");
  const bytes = await file.arrayBuffer();
  return loadWorkbookFromArrayBuffer(bytes, options);
}

export async function loadWorkbookFromArrayBuffer(
  bytes: ArrayBuffer,
  options: WorkbookLoaderOptions = {},
): Promise<WorkbookLayout> {
  const worker = createExtractionWorker(options);
  return await new Promise((resolve, reject) => {
    worker.onmessage = (event) => {
      const message = event.data as
        | { type: "stage"; label: string }
        | { type: "layout"; layout: WorkbookLayout }
        | { type: "error"; message: string };
      if (message.type === "stage") {
        progress(options, message.label);
      } else if (message.type === "layout") {
        worker.terminate();
        resolve(message.layout);
      } else if (message.type === "error") {
        worker.terminate();
        reject(new Error(message.message));
      }
    };
    worker.onerror = (event) => {
      worker.terminate();
      reject(new Error(event.message || "Workbook worker failed"));
    };
    worker.postMessage(
      { bytes, wasmBinaryUrl: options.wasmBinaryUrl ?? DEFAULT_WASM_BINARY_URL },
      [bytes],
    );
  });
}

export async function createWorkbookPreviewerFromFile(
  container: HTMLElement,
  file: Blob,
  options: CreateWorkbookPreviewerFromFileOptions = {},
): Promise<WorkbookPreviewer> {
  const layout = await loadWorkbookFromFile(file, options);
  progress(options, "Preparing preview");
  const previewer = createWorkbookPreviewer(container, layout, options);
  progress(options, "Rendering canvas");
  return previewer;
}

function createExtractionWorker(options: WorkbookLoaderOptions): Worker {
  if (options.workerUrl) {
    return new Worker(options.workerUrl, { type: "module" });
  }
  return new Worker(new URL("./xlsxWorker.js", import.meta.url), { type: "module" });
}

function progress(options: WorkbookLoaderOptions, label: string): void {
  options.onProgress?.({ label });
}
