import type { Sheet } from "./types.js";
import type { Grid } from "./grid.js";
import type { Pane, Viewport } from "./renderTypes.js";
export declare function splitPanes(sheet: Sheet, g: Grid, vp: Viewport | null, canvasW: number, canvasH: number): Pane[];
export declare function paneAtPoint(sheet: Sheet, g: Grid, vp: Viewport | null, canvasW: number, canvasH: number, cx: number, cy: number): {
    tx: number;
    ty: number;
    kind: "tl" | "tr" | "bl" | "br";
} | null;
export declare function frozenDims(sheet: Sheet, g: Grid): {
    splitX: number;
    splitY: number;
    pcw: number;
    prh: number;
};
