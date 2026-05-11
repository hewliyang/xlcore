import type { Sheet } from "./types.js";
import type { Grid } from "./grid.js";
import type { RenderOptions } from "./renderTypes.js";
export declare function resolveSelection(opts: RenderOptions, g: Grid): {
    r1: number;
    c1: number;
    r2: number;
    c2: number;
} | null;
export declare function drawSelection(ctx: CanvasRenderingContext2D, sheet: Sheet, g: Grid, sel: {
    r1: number;
    c1: number;
    r2: number;
    c2: number;
}, active: {
    r: number;
    c: number;
} | null): void;
