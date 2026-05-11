import type { Drawing, Sheet } from "./types.js";
export declare const HEADER_H = 22;
export declare const HEADER_W = 44;
export declare const OUTLINE_GUTTER_STEP = 12;
export declare const OUTLINE_GUTTER_PAD = 4;
export interface Grid {
    colX: number[];
    colW: number[];
    rowY: number[];
    rowH: number[];
    totalW: number;
    totalH: number;
    maxCol: number;
    maxRow: number;
    rowGutterW: number;
    colGutterH: number;
    originX: number;
    originY: number;
    rowOutlineDepth: number;
    colOutlineDepth: number;
}
export declare function buildGrid(sheet: Sheet, colOverrides?: Map<number, number>, rowOverrides?: Map<number, number>, requiredFarX?: number, requiredFarY?: number): Grid;
export declare function colLabel(n: number): string;
export declare function anchorToRect(d: Drawing, g: Grid): {
    x: number;
    y: number;
    w: number;
    h: number;
} | null;
