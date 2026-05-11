import type { Sparkline } from "./Sparkline.js";
/**
 * One `<x14:sparklineGroup>` — shared chrome across N `<x14:sparkline>`
 * children. All booleans default false unless noted (matches OOXML).
 */
export type SparklineGroup = {
    /**
     * `"line"` (default), `"column"`, or `"stacked"` (win/loss).
     */
    sparkType: string;
    /**
     * Default 0.75pt — matches Excel's UI default for new sparklines.
     */
    lineWeight: number;
    markers: boolean;
    high: boolean;
    low: boolean;
    first: boolean;
    last: boolean;
    negative: boolean;
    /**
     * `displayXAxis=1` paints a horizontal axis line at zero when the
     * data crosses zero. (Excel calls this "Show Axis".)
     */
    displayXAxis: boolean;
    rightToLeft: boolean;
    /**
     * `"gap"` (default), `"zero"`, or `"span"` — controls how empty
     * cells in the data range are treated.
     */
    displayEmptyCellsAs: string;
    /**
     * `"individual"` (default), `"group"`, or `"custom"`.
     */
    minAxisType: string;
    maxAxisType: string;
    /**
     * Set when `min_axis_type == "custom"`.
     */
    manualMin?: number;
    manualMax?: number;
    /**
     * Resolved when `min_axis_type == "group"` — the renderer should
     * use this as both the per-cell min and max so the entire group
     * shares one y-scale. `None` when not in group mode (or no data).
     */
    groupMin?: number;
    groupMax?: number;
    /**
     * Series fill / line color (hex `RRGGBB`). `None` ⇒ renderer
     * falls back to a sensible default (theme accent1).
     */
    colorSeries?: string;
    colorNegative?: string;
    colorAxis?: string;
    colorMarkers?: string;
    colorFirst?: string;
    colorLast?: string;
    colorHigh?: string;
    colorLow?: string;
    /**
     * Anchored sparklines that share this group's chrome.
     */
    sparklines: Array<Sparkline>;
};
