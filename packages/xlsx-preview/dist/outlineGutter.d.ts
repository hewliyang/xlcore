import type { Sheet } from "./types.js";
import type { Grid } from "./grid.js";
export interface OutlineRun {
    axis: "row" | "col";
    level: number;
    start: number;
    end: number;
    summary: number;
}
export declare function computeOutlineRuns(sheet: Sheet, g: Grid): OutlineRun[];
export declare function isOutlineRunCollapsed(run: OutlineRun, g: Grid): boolean;
export interface OutlineGutterView {
    sx: number;
    sy: number;
    splitX: number;
    splitY: number;
    pcw: number;
    prh: number;
    canvasW: number;
    canvasH: number;
}
export interface OutlineButtonHit {
    run: OutlineRun;
    cx: number;
    cy: number;
    collapsed: boolean;
}
export declare function outlineButtonHits(sheet: Sheet, g: Grid, view: OutlineGutterView): OutlineButtonHit[];
export interface OutlineCornerHit {
    axis: "row" | "col";
    level: number;
    cx: number;
    cy: number;
}
export declare function outlineCornerHits(g: Grid): OutlineCornerHit[];
export declare const OUTLINE_BUTTON_HIT_RADIUS = 7;
export declare function drawOutlineButtons(ctx: CanvasRenderingContext2D, sheet: Sheet, g: Grid, view: OutlineGutterView): void;
export declare function drawCollapsedRowTicks(ctx: CanvasRenderingContext2D, g: Grid, sy: number, splitY: number, prh: number, canvasH: number, rowScrollVis: {
    firstRow: number;
    lastRow: number;
}): void;
export declare function drawCollapsedColTicks(ctx: CanvasRenderingContext2D, g: Grid, sx: number, splitX: number, pcw: number, canvasW: number, colScrollVis: {
    firstCol: number;
    lastCol: number;
}): void;
export declare function drawOutlineCornerButtons(ctx: CanvasRenderingContext2D, g: Grid): void;
export declare function drawRowOutlineGutter(ctx: CanvasRenderingContext2D, sheet: Sheet, g: Grid, sy: number, splitY: number, prh: number, canvasH: number): void;
export declare function drawColOutlineGutter(ctx: CanvasRenderingContext2D, sheet: Sheet, g: Grid, sx: number, splitX: number, pcw: number, canvasW: number): void;
