import type { Color } from "./Color.js";
/**
 * Differential format — a sparse style overlay applied on top of a
 * cell's base style when a CF rule matches. Mirrors `<x:dxf>` in
 * `xl/styles.xml`. Every field is optional; missing fields mean
 * "inherit from base".
 */
export type Dxf = {
    fontColor?: Color;
    bold?: boolean;
    italic?: boolean;
    strike?: boolean;
    underline?: boolean;
    /**
     * OOXML `<u val="..."/>` variant; see `Font.underline_style`.
     */
    underlineStyle?: string;
    /**
     * Fill foreground color (solid pattern). Background is rare in dxfs.
     */
    fillColor?: Color;
    /**
     * Override number-format code, e.g. `"0.00%"`.
     */
    numFmt?: string;
    /**
     * `<vertAlign val="..."/>` override from a dxf font block. See
     * `TextRun.vert_align`.
     */
    vertAlign?: string;
};
