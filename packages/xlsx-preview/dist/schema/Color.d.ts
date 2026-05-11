/**
 * Color: at least one of `rgb`, `theme`, or `indexed` is set.
 */
export type Color = {
    /**
     * 8-char "AARRGGBB" or 6-char "RRGGBB".
     */
    rgb?: string;
    theme?: number;
    indexed?: number;
    /**
     * -1.0..1.0 (negative = darker, positive = lighter).
     */
    tint?: number;
};
