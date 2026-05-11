import type { DataLabels } from "./DataLabels.js";
export type ChartSeries = {
    name: string;
    /**
     * Formula for the series name (e.g. `Sheet1!$A$2`). Resolved after
     * sheet extraction if `name` is empty.
     */
    nameRef?: string;
    /**
     * CSS color string. May come from explicit spPr.solidFill or, more
     * commonly, an Office theme accent (`accent1..accent6`).
     */
    color?: string;
    values: Array<number>;
    /**
     * Formula for the values range (e.g. `Sheet1!$B$2:$E$2`). Resolved
     * after sheet extraction if `values` is empty.
     */
    valuesRef?: string;
    /**
     * Numeric x-values for scatter / bubble series. Empty for chart
     * types that use the chart-level `categories` array instead.
     */
    xValues: Array<number>;
    /**
     * Formula for the x-values range (scatter only). Resolved after
     * sheet extraction if `x_values` is empty.
     */
    xValuesRef?: string;
    /**
     * Per-data-point CSS color overrides, parallel to `values` (one
     * entry per category). Empty string at index `i` means "use the
     * series-level `color` (or the renderer's per-slice palette for
     * pie/doughnut)". Sourced from `<c:dPt>` children with explicit
     * `spPr` fills. Empty Vec when no `<c:dPt>` overrides exist.
     */
    pointColors: Array<string>;
    /**
     * Per-series `<c:dLbls>`. Overrides chart-level `data_labels`
     * when present.
     */
    dataLabels?: DataLabels;
};
