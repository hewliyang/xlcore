import type { Color } from "./Color.js";
export type BorderLine = {
    /**
     * "thin","medium","thick","double","dotted","dashed","hair", etc.
     */
    style: string;
    color?: Color;
};
