import type { Sheet } from "./types.js";
import type { Grid } from "./grid.js";
import type { Visible } from "./renderTypes.js";
export declare function drawGridLines(ctx: CanvasRenderingContext2D, sheet: Sheet, g: Grid, vis: Visible): void;
export interface CellRect {
    x: number;
    y: number;
    w: number;
    h: number;
}
export declare function cellRect(g: Grid, r: number, c: number): CellRect;
export declare function mergedRect(g: Grid, m: {
    r1: number;
    c1: number;
    r2: number;
    c2: number;
}): CellRect;
export declare function buildMergeMaps(sheet: Sheet): {
    covered: Set<string>;
    topLeftOf: Map<string, {
        r1: number;
        c1: number;
        r2: number;
        c2: number;
    }>;
};
export declare function rectFor(sheet: Sheet, g: Grid, r: number, c: number, topLeftOf: Map<string, {
    r1: number;
    c1: number;
    r2: number;
    c2: number;
}>): CellRect;
export { findCell } from "./columnar.js";
