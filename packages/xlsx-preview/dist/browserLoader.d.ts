import { type PreviewerOptions, type WorkbookPreviewer } from "./previewer.js";
import type { WorkbookLayout } from "./types.js";
export interface WorkbookLoadProgress {
    label: string;
}
export interface WorkbookLoaderOptions {
    wasmUrl?: string;
    workerUrl?: string;
    onProgress?: (progress: WorkbookLoadProgress) => void;
}
export interface CreateWorkbookPreviewerFromFileOptions extends WorkbookLoaderOptions, PreviewerOptions {
}
export declare function loadWorkbookFromFile(file: Blob, options?: WorkbookLoaderOptions): Promise<WorkbookLayout>;
export declare function loadWorkbookFromArrayBuffer(bytes: ArrayBuffer, options?: WorkbookLoaderOptions): Promise<WorkbookLayout>;
export declare function createWorkbookPreviewerFromFile(container: HTMLElement, file: Blob, options?: CreateWorkbookPreviewerFromFileOptions): Promise<WorkbookPreviewer>;
