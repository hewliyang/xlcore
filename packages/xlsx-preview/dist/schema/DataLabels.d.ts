/**
 * `<c:dLbls>` — what to print next to each data point. Mirrors the
 * OOXML `CT_DLbls` block. Rendered text per point is built by joining
 * the enabled fields with `separator` (default `", "`):
 *
 *   `[seriesName][sep][category][sep][value | percent]`
 *
 * Empty when extracted from `<c:delete val="1"/>` (suppression marker).
 */
export type DataLabels = {
    showValue: boolean;
    showCategory: boolean;
    showSeriesName: boolean;
    /**
     * Show value as % of category total. Pie/doughnut natively in
     * Excel; we honor it on any chart type that has a category total.
     */
    showPercent: boolean;
    /**
     * `ctr`, `inEnd`, `inBase`, `outEnd`, `t`, `b`, `l`, `r`, `bestFit`.
     * None ⇒ chart-type default (`outEnd` for column, `r` for bar,
     * `ctr` for line/scatter, `outEnd`/`bestFit` for pie).
     */
    position?: string;
    /**
     * String inserted between fields when more than one show* is on.
     * Default `", "`.
     */
    separator?: string;
    /**
     * Number-format code for the value field, e.g. `"0.0%"`. None
     * falls back to the chart's `valueFormat`.
     */
    numFmt?: string;
};
