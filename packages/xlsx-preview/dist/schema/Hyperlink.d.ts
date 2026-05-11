import type { Merge } from "./Merge.js";
/**
 * One `<hyperlink>` entry from the worksheet `<hyperlinks>` block.
 * At least one of `target` / `location` is set.
 */
export type Hyperlink = {
    /**
     * Range covered by this hyperlink (often a single cell, but the
     * schema allows e.g. `A1:B3`).
     */
    range: Merge;
    /**
     * External absolute target — the `Target` of the `r:id` rel.
     * `None` for in-workbook (`location`) links.
     */
    target?: string;
    /**
     * In-workbook bookmark, e.g. `'Sheet 2'!A1`. Mutually-not-exclusive
     * with `target` — Excel sometimes emits both.
     */
    location?: string;
    /**
     * Hover tooltip text.
     */
    tooltip?: string;
    /**
     * Display-string override (rare; the cell's own value is the
     * usual visible text).
     */
    display?: string;
};
