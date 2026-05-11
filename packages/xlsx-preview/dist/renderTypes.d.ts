export interface Viewport {
    x: number;
    y: number;
    w: number;
    h: number;
}
export interface RenderOptions {
    scale?: number;
    zoom?: number;
    renderHeaders?: boolean;
    colOverrides?: Map<number, number>;
    rowOverrides?: Map<number, number>;
    activeCell?: {
        r: number;
        c: number;
    } | null;
    selection?: {
        r1: number;
        c1: number;
        r2: number;
        c2: number;
    } | null;
    viewport?: Viewport;
}
export type Visible = {
    firstCol: number;
    lastCol: number;
    firstRow: number;
    lastRow: number;
};
export interface Pane {
    cx: number;
    cy: number;
    cw: number;
    ch: number;
    tx: number;
    ty: number;
    vis: Visible;
    kind: "tl" | "tr" | "bl" | "br";
}
