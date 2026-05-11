import type { Sheet, WorkbookLayout } from "./types.js";
export interface InteractHandle {
    /** Detach all listeners. */
    destroy(): void;
}
export interface InteractOptions {
    getSheet(): Sheet;
    getLayout(): WorkbookLayout;
    /** Read/write mailbox for the current zoom factor (1 = 100%). */
    zoom: {
        get(): number;
        set(value: number): void;
    };
    /** 1-based column index → width in CSS px. Mutated in place on resize. */
    colOverrides: Map<number, number>;
    /** 1-based row index → height in CSS px. Mutated in place on resize. */
    rowOverrides: Map<number, number>;
    /**
     * Read/write mailbox for the active cell (1-based). `null` means no
     * selection. Updated by clicks and arrow keys; the host can also push
     * external selections in.
     */
    activeCell: {
        get(): {
            r: number;
            c: number;
        } | null;
        set(v: {
            r: number;
            c: number;
        } | null): void;
    };
    /**
     * Read/write mailbox for the multi-cell selection range (1-based,
     * inclusive). When omitted the renderer falls back to a 1×1 range at
     * `activeCell`. Header clicks expand it to whole columns / rows.
     */
    selection?: {
        get(): Selection | null;
        set(v: Selection | null): void;
    };
    /** Optional element to scroll-anchor zoom around and to auto-scroll on arrow-key navigation. */
    scrollContainer?: HTMLElement;
    /**
     * Current viewport offset (logical px, pre-zoom). When provided, the
     * interaction layer assumes the canvas is virtualized: pointer coords get
     * `viewport.x/y` added before being mapped onto the sheet, headers pan
     * with scroll, etc.
     */
    getViewport?: () => {
        x: number;
        y: number;
        w: number;
        h: number;
    } | null;
    /** Called whenever interact mutates state and the canvas should re-paint. */
    redraw(): void;
}
export interface Selection {
    r1: number;
    c1: number;
    r2: number;
    c2: number;
}
/**
 * Wire up interactivity on `canvas`. Idempotent per-canvas: call `destroy()`
 * on the returned handle before reattaching.
 */
export declare function attachInteractivity(canvas: HTMLCanvasElement, opts: InteractOptions): InteractHandle;
