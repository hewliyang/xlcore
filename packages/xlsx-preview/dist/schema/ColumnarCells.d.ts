export type ColumnarCells = {
    count: number;
    /**
     * 1-based row indices, u32 LE.
     */
    r: string;
    /**
     * 1-based col indices, u32 LE.
     */
    c: string;
    /**
     * Kind enum, u8.
     */
    kind: string;
    /**
     * Index into `Sheet.value_pool`, i32 LE; -1 = no value.
     */
    valueIdx: string;
    /**
     * Index into `Sheet.formula_pool`, i32 LE; -1 = no formula.
     */
    formulaIdx: string;
    /**
     * `Cell.styleIndex`, i32 LE; -1 = no explicit style.
     */
    styleIdx: string;
    /**
     * Index into `Sheet.inline_runs`, i32 LE; -1 = no inline runs.
     */
    runsIdx: string;
    /**
     * Row-pointer array: cells for `row_meta.index[i]` live in
     * `[row_ptr[i], row_ptr[i+1])`. u32 LE, length == `row_meta.count + 1`.
     */
    rowPtr: string;
};
