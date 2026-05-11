import type { Chart } from "./Chart.js";
import type { DrawingAnchor } from "./DrawingAnchor.js";
import type { Image } from "./Image.js";
/**
 * One drawing object placed on the sheet, with its xlsx cell-anchor.
 */
export type Drawing = {
    /**
     * `chart`, `image`, `shape` (only `chart` and `image` are rendered).
     */
    kind: string;
    anchor: DrawingAnchor;
    chart?: Chart;
    image?: Image;
};
