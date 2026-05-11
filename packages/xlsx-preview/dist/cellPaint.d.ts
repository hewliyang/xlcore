import type { Fill, WorkbookLayout } from "./types.js";
import type { Sheet } from "./types.js";
import type { Grid } from "./grid.js";
import type { CellRect } from "./geometry.js";
import type { Visible } from "./renderTypes.js";
export declare function paintFill(ctx: CanvasRenderingContext2D, rect: CellRect, fill: Fill): void;
export declare function drawDefaultFills(ctx: CanvasRenderingContext2D, sheet: Sheet, layout: WorkbookLayout, g: Grid, vis: Visible): void;
export declare function drawCellBackgrounds(ctx: CanvasRenderingContext2D, sheet: Sheet, layout: WorkbookLayout, g: Grid, vis: Visible): void;
export declare function drawCellBorders(ctx: CanvasRenderingContext2D, sheet: Sheet, layout: WorkbookLayout, g: Grid, vis: Visible): void;
