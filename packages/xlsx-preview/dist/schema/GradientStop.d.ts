import type { Color } from "./Color.js";
export type GradientStop = {
    /**
     * Position along the gradient axis. 0..1.
     */
    position: number;
    color: Color;
};
