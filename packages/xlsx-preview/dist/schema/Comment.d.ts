import type { TextRun } from "./TextRun.js";
/**
 * One `<comment>` entry from the worksheet's comments part.
 */
export type Comment = {
    /**
     * 1-based row.
     */
    r: number;
    /**
     * 1-based column.
     */
    c: number;
    /**
     * Resolved author name (looked up in the comments part's
     * `<authors>` table). Empty when `authorId` is out-of-range.
     */
    author: string;
    /**
     * Concatenated plain-text body (matches `runs` joined).
     */
    text: string;
    /**
     * Per-run styled spans, mirroring the SST rich-text shape. Empty
     * for plain-text comments — renderer falls back to a default
     * font in that case.
     */
    runs: Array<TextRun>;
};
