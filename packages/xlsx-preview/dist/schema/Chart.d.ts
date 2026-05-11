import type { ChartSeries } from "./ChartSeries.js";
import type { DataLabels } from "./DataLabels.js";
export type Chart = {
    /**
     * `column`, `bar`, `line`, `pie`, `area`, `scatter`, `unknown`.
     * `column` and `bar` collapse into one `BarChart` schema entry; the
     * barDir attribute distinguishes them.
     */
    type: string;
    title?: string;
    series: Array<ChartSeries>;
    /**
     * X-axis labels (categories). Often pulled from the cat strRef cache;
     * if absent the renderer falls back to series-relative indices.
     */
    categories: Array<string>;
    /**
     * Formula reference (e.g. `Sheet1!$B$1:$E$1`) used to populate
     * `categories` from live workbook data when the chart's strCache is
     * empty. Resolution happens after sheets are extracted.
     */
    categoriesRef?: string;
    /**
     * `t`, `b`, `l`, `r`, `tr` (ECMA-376 legend positions).
     */
    legendPos?: string;
    /**
     * Number-format for the value axis (e.g. "$#,##0").
     */
    valueFormat?: string;
    /**
     * `clustered`, `stacked`, `percentStacked`, `standard` (line/area).
     */
    grouping?: string;
    /**
     * `col` or `bar` (only meaningful for chart_type == bar).
     */
    barDir?: string;
    /**
     * Scatter style: `line`, `lineMarker`, `marker`, `smooth`,
     * `smoothMarker`. Only meaningful for chart_type == scatter.
     * When `None`, the renderer treats the chart as marker-only
     * (matches Excel's UI default even though the OOXML enum
     * default is `line`).
     */
    scatterStyle?: string;
    /**
     * Chart-level `<c:dLbls>` — the per-chart-group default. Series-
     * level `dataLabels` overrides on a per-series basis. None ⇒ no
     * labels (Excel's default for every chart type).
     */
    dataLabels?: DataLabels;
};
