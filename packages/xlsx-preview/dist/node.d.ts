import { Canvas } from "skia-canvas";
import type { RenderOptions } from "./renderTypes.js";
import type { WorkbookLayout } from "./types.js";
export interface RenderPngOptions extends RenderOptions {
    sheetIndex?: number;
    sheetName?: string;
    range?: string;
}
export interface LoadWorkbookFromXlsxOptions {
    sheetIndex?: number;
    sheetName?: string;
}
export declare function loadWorkbookFromXlsx(input: string | ArrayBuffer | Uint8Array, options?: LoadWorkbookFromXlsxOptions): Promise<WorkbookLayout>;
export declare function renderXlsxToCanvas(input: string | ArrayBuffer | Uint8Array, opts?: RenderPngOptions): Promise<Canvas>;
export declare function renderXlsxToPng(input: string | ArrayBuffer | Uint8Array, opts?: RenderPngOptions): Promise<Buffer>;
export declare function renderToCanvas(layout: WorkbookLayout, opts?: RenderPngOptions): Canvas;
export declare function renderToPng(layout: WorkbookLayout, opts?: RenderPngOptions): Promise<Buffer>;
