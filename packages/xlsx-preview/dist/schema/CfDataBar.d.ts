import type { CfvoStop } from "./CfvoStop.js";
import type { Color } from "./Color.js";
/**
 * `dataBar` conditional-format rule. Fills each cell with a horizontal
 * bar whose length is proportional to `(value - min) / (max - min)`,
 * constrained to `[min_length_pct%, max_length_pct%]` of the cell
 * width. When the data range straddles zero the bar splits at the
 * origin: negatives paint `negative_color` leftward, positives paint
 * `color` rightward. Mirrors `<x:dataBar>` in worksheet XML; defaults
 * match ECMA-376 §18.3.1.28.
 */
export type CfDataBar = {
    min: CfvoStop;
    max: CfvoStop;
    /**
     * Fill color for positive (or all) bar segments. Defaults to
     * Excel's standard `#638EC6` blue when the source XML omits the
     * `<color>` child (some writers do).
     */
    color: Color;
    /**
     * Fill color for negative bar segments. None ⇒ red `#FF0000`
     * (Excel default), but renderer should only use it when the data
     * range actually contains negatives.
     */
    negativeColor?: Color;
    /**
     * Minimum bar length as percent of cell width (default 10).
     */
    minLengthPct: number;
    /**
     * Maximum bar length as percent of cell width (default 90).
     */
    maxLengthPct: number;
    /**
     * When false, the cell value is hidden and only the bar paints.
     */
    showValue: boolean;
    /**
     * When true (Excel 2010+ default), the bar fill paints as a
     * linear gradient from `color` at the axis to transparent at
     * the bar's outer edge. When false, paints as a solid block.
     * Stored only on the x14 extension (`<x14:dataBar gradient="..."/>`),
     * which we don't parse yet — defaults to `true` to match what
     * modern Excel + SpreadJS author and what users see by default.
     */
    gradient: boolean;
};
