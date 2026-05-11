import type { Chart } from "./types.js";
export interface Rect {
    x: number;
    y: number;
    w: number;
    h: number;
}
export declare function drawChart(ctx: CanvasRenderingContext2D, chart: Chart, rect: Rect): void;
