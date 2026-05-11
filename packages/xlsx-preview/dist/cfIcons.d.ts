import type { Sheet } from "./types.js";
import type { Grid } from "./grid.js";
import type { Visible } from "./renderTypes.js";
export declare function drawCfIcons(ctx: CanvasRenderingContext2D, sheet: Sheet, g: Grid, vis: Visible, cfIconDraw: Map<string, {
    iconSet: string;
    idx: number;
    n: number;
}>): void;
