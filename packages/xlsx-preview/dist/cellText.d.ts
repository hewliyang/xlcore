import type { Cell, CellFormat, Sheet, WorkbookLayout } from "./types.js";
export interface ResolvedText {
    text: string;
    defaultAlign: "left" | "right" | "center";
    formatColor?: string;
    /** Accounting `*x` fill chars (see `FormatResult.fills`). Each `\u0001`
     *  in `text` is a placeholder the caller must expand against the cell
     *  width. Undefined when the format had no `*x` token. */
    fills?: string[];
}
/** Resolve the effective `CellFormat` (xf) for a cell, applying the
 *  OOXML §18.3.1.4 fallback chain:
 *
 *    cell.s → row.s → col.style → xf 0
 *
 *  Excel writes formula cells (`<c><f/><v/></c>`) without an `s`
 *  attribute when their style matches xf 0; we used to fall through
 *  to `undefined` and then render with the `formatGeneral()` default,
 *  which dropped thousands separators / accounting parens / decimal
 *  precision on every formula cell whose author left the format on
 *  the default xf. Walking the row/col fallbacks first matches what
 *  Excel and SpreadJS do. */
export declare function resolveCellXf(cell: Cell, sheet: Sheet, layout: WorkbookLayout): CellFormat | undefined;
export declare function resolveCellText(cell: Cell, layout: WorkbookLayout, xf: CellFormat | undefined): ResolvedText;
export declare function cellTextValue(cell: Cell, layout: WorkbookLayout): string;
export declare function cellNumericValue(cell: Cell): number | null;
