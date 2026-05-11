/**
 * Conditional-format value object — the `<x:cfvo>` child of
 * `colorScale`/`dataBar`/`iconSet`. Color-scale CFVOs carry their own
 * color and live on `CfColorScaleStop`; data bars share this colorless
 * shape between the min and max stop.
 */
export type CfvoStop = {
    /**
     * `min`, `max`, `num`, `percent`, `percentile`, `formula`,
     * `automin`, `automax`.
     */
    type: string;
    val?: string;
};
