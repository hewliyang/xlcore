import type { Cell, WorkbookLayout } from "./types.js";
import type { Sheet } from "./types.js";
export interface DecodedCells {
    count: number;
    r: Uint32Array;
    c: Uint32Array;
    kind: Uint8Array;
    valueIdx: Int32Array;
    formulaIdx: Int32Array;
    styleIdx: Int32Array;
    runsIdx: Int32Array;
    rowPtr: Uint32Array;
}
export interface DecodedRowMeta {
    count: number;
    index: Uint32Array;
    heightPx: Float32Array;
    styleIdx: Int32Array;
    hidden: Uint8Array;
    outlineLevel: Uint8Array;
    byIndex: Map<number, number>;
}
export declare function decodeWorkbookLayout(layout: WorkbookLayout): WorkbookLayout;
export declare function materializeCell(sheet: Sheet, i: number): Cell;
export declare function iterCellsInRange(sheet: Sheet, firstRow: number, lastRow: number, firstCol: number, lastCol: number, fn: (cell: Cell, i: number) => void): void;
export declare function iterAllCells(sheet: Sheet, fn: (cell: Cell, i: number) => void): void;
export declare function iterRows(sheet: Sheet, fn: (row: {
    index: number;
    heightPx: number | undefined;
    styleIndex: number | undefined;
    hidden: boolean;
}) => void): void;
export declare function findCell(sheet: Sheet, r: number, c: number): Cell | undefined;
