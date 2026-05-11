import type { Sheet } from "./types.js";
import type { Grid } from "./grid.js";
export type DrawableImage = {
    complete?: boolean;
    naturalWidth?: number;
    naturalHeight?: number;
    width?: number;
    height?: number;
    decoding?: "async" | "sync" | "auto";
    onload?: ((event?: Event) => void) | null;
    src?: string | Uint8Array | ArrayBuffer;
};
export declare function preloadDrawingImages(sheet: Sheet, load: (bytes: Uint8Array) => Promise<DrawableImage>): Promise<void>;
export declare function drawDrawings(ctx: CanvasRenderingContext2D, sheet: Sheet, g: Grid): void;
