import { createWorkbookPreviewer, type PreviewerOptions, type WorkbookPreviewer } from "./previewer.js";
import type { WorkbookLayout } from "./types.js";

export interface WorkbookLoadProgress {
  label: string;
}

export interface WorkbookLoaderOptions {
  wasmUrl?: string;
  workerUrl?: string;
  onProgress?: (progress: WorkbookLoadProgress) => void;
}

export interface CreateWorkbookPreviewerFromFileOptions extends WorkbookLoaderOptions, PreviewerOptions {}

const DEFAULT_WASM_URL = new URL("./xlcore_wasm.js", import.meta.url).toString();
const DEFAULT_WORKER_URL = new URL("./xlsxWorker.js", import.meta.url).toString();

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
    worker.postMessage({ bytes, wasmUrl: options.wasmUrl ?? DEFAULT_WASM_URL }, [bytes]);
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
  const workerUrl = options.workerUrl ?? DEFAULT_WORKER_URL;
  try {
    return new Worker(workerUrl, { type: "module" });
  } catch {
    return createBlobWorker();
  }
}

function createBlobWorker(): Worker {
  const source = `
let wasmModulePromise = null;
const stage = (label) => self.postMessage({ type: "stage", label });
self.onmessage = async (event) => {
  try {
    const { bytes, wasmUrl } = event.data;
    stage("Loading WASM");
    wasmModulePromise ??= import(wasmUrl).then(async (mod) => {
      await mod.default();
      return mod;
    });
    const mod = await wasmModulePromise;
    stage("Extracting OOXML");
    const layout = mod.extract_xlsx(new Uint8Array(bytes), undefined);
    self.postMessage({ type: "layout", layout });
  } catch (error) {
    self.postMessage({ type: "error", message: error && error.stack ? error.stack : String(error) });
  }
};
`;
  return new Worker(URL.createObjectURL(new Blob([source], { type: "text/javascript" })));
}

function progress(options: WorkbookLoaderOptions, label: string): void {
  options.onProgress?.({ label });
}
