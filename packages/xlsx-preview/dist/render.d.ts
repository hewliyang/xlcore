import type { Sheet, WorkbookLayout } from "./types.js";
export { applyTint } from "./color.js";
export { HEADER_H, HEADER_W, buildGrid } from "./grid.js";
export { paneAtPoint, frozenDims } from "./panes.js";
import type { RenderOptions } from "./renderTypes.js";
export type { RenderOptions, Viewport } from "./renderTypes.js";
export declare function render(canvas: HTMLCanvasElement | {
    width: number;
    height: number;
    getContext(t: "2d"): CanvasRenderingContext2D | null;
}, sheet: Sheet, layout: WorkbookLayout, opts?: RenderOptions): void;
