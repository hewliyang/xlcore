import type { Dxf, Sheet, WorkbookLayout } from "./types.js";
import type { Grid } from "./grid.js";
import type { Visible } from "./renderTypes.js";
export declare function drawCellText(ctx: CanvasRenderingContext2D, sheet: Sheet, layout: WorkbookLayout, g: Grid, vis: Visible, cfDxfs: Map<string, Dxf>, cfTextSuppress: Set<string>, cfIconReserve: Map<string, number>): void;
export declare function drawFreezeIndicators(ctx: CanvasRenderingContext2D, sheet: Sheet, g: Grid, canvasW: number, canvasH: number): void;
