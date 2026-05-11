import { type Selection } from "./interact.js";
import type { Sheet, WorkbookLayout } from "./types.js";
export interface PreviewerOptions {
    initialSheet?: number | string;
    initialZoom?: number;
    className?: string;
}
export interface PreviewerState {
    activeSheetIndex: number;
    activeCell: {
        r: number;
        c: number;
    };
    selection: Selection;
    zoom: number;
}
export type PreviewerEventName = "selectionchange" | "sheetchange" | "zoomchange" | "layoutchange";
export interface WorkbookPreviewer {
    readonly root: HTMLElement;
    readonly canvas: HTMLCanvasElement;
    readonly layout: WorkbookLayout;
    destroy(): void;
    redraw(): void;
    getState(): PreviewerState;
    getActiveSheet(): Sheet;
    getActiveSheetIndex(): number;
    setActiveSheet(sheet: number | string): void;
    getActiveCell(): {
        r: number;
        c: number;
    };
    getSelection(): Selection;
    selectCell(r: number, c: number, options?: {
        scroll?: boolean;
    }): void;
    selectRange(selection: Selection, options?: {
        scroll?: boolean;
        activeCell?: {
            r: number;
            c: number;
        };
    }): void;
    scrollToCell(r: number, c: number): void;
    getZoom(): number;
    setZoom(zoom: number): void;
    on(name: PreviewerEventName, listener: EventListener): void;
    off(name: PreviewerEventName, listener: EventListener): void;
}
export declare function createWorkbookPreviewer(container: HTMLElement, layout: WorkbookLayout, options?: PreviewerOptions): WorkbookPreviewer;
