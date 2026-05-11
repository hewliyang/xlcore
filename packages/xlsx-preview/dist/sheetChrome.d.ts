import type { Dxf, Sheet, WorkbookLayout } from "./types.js";
import type { Grid } from "./grid.js";
import type { Pane, Viewport, Visible } from "./renderTypes.js";
export declare function computeTableState(sheet: Sheet, vis?: Visible): {
    tableDxfs: Map<string, Dxf>;
    filterArrows: Set<string>;
};
export declare function drawFilterArrows(ctx: CanvasRenderingContext2D, sheet: Sheet, g: Grid, vis: Visible, filterArrows: Set<string>): void;
export declare function drawHeaders(ctx: CanvasRenderingContext2D, sheet: Sheet, g: Grid, sel: {
    r1: number;
    c1: number;
    r2: number;
    c2: number;
} | null, vp: Viewport | null, canvasW: number, canvasH: number, panes: Pane[]): void;
export declare function computeHyperlinkDxfs(sheet: Sheet, layout: WorkbookLayout): Map<string, Dxf>;
export declare function drawCommentMarkers(ctx: CanvasRenderingContext2D, sheet: Sheet, g: Grid, vis: Visible): void;
