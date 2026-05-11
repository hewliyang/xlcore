import type { Cell } from "./Cell.js";
export type Row = {
    index: number;
    heightPx?: number;
    cells: Array<Cell>;
    styleIndex?: number;
    hidden: boolean;
    /**
     * OOXML `<row outlineLevel="N">` (0..=7). Wire-only on this
     * transient struct; gets folded into `RowMetaBlob.outline_level`
     * during `compactify_sheet`. Always 0 in serialized JSON
     * (Sheet.rows is `ts(skip)`-hidden).
     */
    outlineLevel: number;
};
