import type { Color } from "./Color.js";
export type CfColorScaleStop = {
    /**
     * `min`, `max`, `num`, `percent`, `percentile`, `formula`.
     */
    type: string;
    val?: string;
    color: Color;
};
