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
  const layout = await loadWorkbookFromFile(file, options);
  progress(options, "Preparing preview");
  const previewer = createWorkbookPreviewer(container, layout, options);
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
