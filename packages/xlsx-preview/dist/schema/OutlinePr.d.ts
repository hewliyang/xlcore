/**
 * `<outlinePr>` defaults from `<sheetPr>`. Both fields default to true
 * when `<outlinePr>` is absent (matches Excel/OOXML spec).
 */
export type OutlinePr = {
    summaryBelow: boolean;
    summaryRight: boolean;
};
