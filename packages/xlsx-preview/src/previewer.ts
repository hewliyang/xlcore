import { decodeWorkbookLayout, findCell, iterRows } from "./columnar.js";
import { colorToCssWithTheme } from "./color.js";
import type { LoadReport } from "./errors.js";
import {
  attachInteractivity,
  type InteractHandle,
  type PivotFilterEvent,
  type TableFilterEvent,
  type Selection,
} from "./interact.js";
import { HEADER_H, HEADER_W, buildGrid, render } from "./render.js";
import { referencesToHighlights } from "./highlights.js";
import type { HighlightRange } from "./renderTypes.js";
import type { DependencyReference } from "./api-schema/DependencyReference.js";
import { cellRect } from "./geometry.js";
import {
  createPivotFilterPopover,
  type PivotFilterController,
  type PivotFilterPopoverHandle,
} from "./pivotFilterPopover.js";
import {
  createTableFilterPopover,
  type TableFilterController,
  type TableFilterPopoverHandle,
} from "./tableFilterPopover.js";
import type { Sheet as WireSheet } from "./schema/Sheet.js";
import type { Sheet, WorkbookLayout } from "./types.js";

export interface PreviewerOptions {
  initialSheet?: number | string;
  initialZoom?: number;
  className?: string;
  report?: LoadReport;
  showHidden?: boolean;
  editable?: boolean;

  pivotController?: PivotFilterController;
  tableController?: TableFilterController;
  engine?: PreviewerEngine;
}

export interface PreviewerEngine {
  parseReferences(sheetName: string, anchorRef: string, formula: string): DependencyReference[];
  functionNames(): string[];
}

const HIGHLIGHT_PALETTE = [
  "#2563eb",
  "#16a34a",
  "#db2777",
  "#ea580c",
  "#0891b2",
  "#9333ea",
  "#ca8a04",
];

export type { PivotFilterController, PivotFilterContext } from "./pivotFilterPopover.js";
export type { TableFilterController, TableFilterContext } from "./tableFilterPopover.js";

function isTabVisible(sheet: WireSheet, showHidden: boolean): boolean {
  const state = sheet.state;
  if (state === "veryHidden") return false;
  if (state === "hidden") return showHidden;
  return true;
}

export interface PreviewerState {
  activeSheetIndex: number;
  activeCell: { r: number; c: number };
  selection: Selection;
  zoom: number;
}

export type PreviewerEventName =
  | "selectionchange"
  | "sheetchange"
  | "zoomchange"
  | "layoutchange"
  | "pivotfilter"
  | "tablefilter"
  | "celledit";

export type { PivotFilterEvent, TableFilterEvent } from "./interact.js";

export interface WorkbookPreviewer {
  readonly root: HTMLElement;
  readonly canvas: HTMLCanvasElement;
  readonly layout: WorkbookLayout;

  readonly report?: LoadReport;
  destroy(): void;
  redraw(): void;
  replaceLayout(layout: WorkbookLayout): void;
  getState(): PreviewerState;
  getActiveSheet(): Sheet;
  getActiveSheetIndex(): number;
  setActiveSheet(sheet: number | string): void;
  getActiveCell(): { r: number; c: number };
  getSelection(): Selection;
  selectCell(r: number, c: number, options?: { scroll?: boolean }): void;
  selectRange(
    selection: Selection,
    options?: { scroll?: boolean; activeCell?: { r: number; c: number } },
  ): void;
  scrollToCell(r: number, c: number): void;
  getZoom(): number;
  setZoom(zoom: number): void;
  on(name: PreviewerEventName, listener: EventListener): void;
  off(name: PreviewerEventName, listener: EventListener): void;
}

interface SheetState {
  colOverrides: Map<number, number>;
  rowOverrides: Map<number, number>;
  activeCell: { r: number; c: number };
  selection: Selection;
}

const VIRTUAL_EXTRA_COLS = 50;
const VIRTUAL_EXTRA_ROWS = 1000;

export function createWorkbookPreviewer(
  container: HTMLElement,
  layout: WorkbookLayout,
  options: PreviewerOptions = {},
): WorkbookPreviewer {
  return new WorkbookPreviewerImpl(container, layout, options);
}

class WorkbookPreviewerImpl extends EventTarget implements WorkbookPreviewer {
  readonly root: HTMLElement;
  readonly canvas: HTMLCanvasElement;
  layout: WorkbookLayout;
  readonly report?: LoadReport;
  private readonly pivotController?: PivotFilterController;
  private pivotPopover: PivotFilterPopoverHandle | null = null;
  private readonly tableController?: TableFilterController;
  private tablePopover: TableFilterPopoverHandle | null = null;

  private readonly tabs: HTMLDivElement;
  private readonly sheetTabs: HTMLDivElement;
  private readonly formulaBar: HTMLDivElement;
  private readonly zoomBox: HTMLDivElement;
  private readonly nameBox: HTMLDivElement;
  private readonly formulaBox: HTMLInputElement;
  private readonly zoomLabel: HTMLSpanElement;
  private readonly zoomOut: HTMLButtonElement;
  private readonly zoomIn: HTMLButtonElement;
  private readonly stage: HTMLDivElement;
  private readonly spacer: HTMLDivElement;

  private readonly editInput: HTMLInputElement;
  private editCell: { r: number; c: number } | null = null;
  private readonly sheetStates: SheetState[];
  private readonly tabButtons: Array<HTMLButtonElement | null> = [];
  private readonly showHidden: boolean;
  private readonly editable: boolean;
  private readonly engine?: PreviewerEngine;
  private highlights: HighlightRange[] = [];
  private readonly resizeObserver: ResizeObserver;
  private interactHandle: InteractHandle | null = null;
  private activeSheetIndex = 0;
  private zoom = 1;
  private viewport = { x: 0, y: 0, w: 0, h: 0 };
  private rafPending = false;

  constructor(container: HTMLElement, rawLayout: WorkbookLayout, options: PreviewerOptions) {
    super();
    this.layout = decodeWorkbookLayout(rawLayout);
    this.report = options.report;
    this.pivotController = options.pivotController;
    this.tableController = options.tableController;
    this.zoom = clamp(options.initialZoom ?? 1, 0.25, 4);
    this.showHidden = options.showHidden === true;
    this.editable = options.editable === true;
    this.engine = options.engine;
    this.sheetStates = this.layout.sheets.map(() => ({
      colOverrides: new Map(),
      rowOverrides: new Map(),
      activeCell: { r: 1, c: 1 },
      selection: { r1: 1, c1: 1, r2: 1, c2: 1 },
    }));

    this.root = document.createElement("div");
    this.root.className = options.className ?? "xlcore-previewer";
    this.root.style.cssText =
      "display:grid;grid-template-rows:auto auto minmax(0,1fr);min-width:0;min-height:0;width:100%;height:100%;overflow:hidden;background:#f4f4f5;font:13px -apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;";

    this.formulaBar = document.createElement("div");
    this.formulaBar.className = "xlcore-formula-bar";
    this.formulaBar.style.cssText =
      "display:flex;gap:6px;align-items:center;padding:6px 8px;background:#f8fafc;border-bottom:1px solid #d1d5db;min-width:0;";
    this.nameBox = document.createElement("div");
    this.nameBox.style.cssText =
      "font:12px ui-monospace,SFMono-Regular,Menlo,monospace;padding:4px 10px;background:#fff;border:1px solid #d1d5db;border-radius:4px;min-width:86px;color:#111827;text-align:center;white-space:nowrap;";
    const fxLabel = document.createElement("div");
    fxLabel.textContent = "fx";
    fxLabel.style.cssText =
      "font:600 12px ui-monospace,SFMono-Regular,Menlo,monospace;color:#4b5563;padding:0 2px;";
    this.formulaBox = document.createElement("input");
    this.formulaBox.readOnly = !this.editable;
    this.formulaBox.setAttribute("aria-label", "Formula or value");
    this.formulaBox.style.cssText =
      "min-width:0;flex:1;height:28px;padding:0 9px;border:1px solid #d1d5db;border-radius:4px;background:#fff;color:#111827;font:12px ui-monospace,SFMono-Regular,Menlo,monospace;";
    this.formulaBar.append(this.nameBox, fxLabel, this.formulaBox);
    if (this.editable) {
      this.formulaBox.addEventListener("keydown", (ev) => this.onFormulaBoxKeyDown(ev));
      this.formulaBox.addEventListener("focus", this.scheduleDraw);
      this.formulaBox.addEventListener("blur", this.scheduleDraw);
    }

    this.tabs = document.createElement("div");
    this.tabs.className = "xlcore-tabs";
    this.tabs.style.cssText =
      "display:flex;align-items:stretch;gap:6px;padding:0 8px;background:#e5e7eb;min-width:0;min-height:31px;overflow:hidden;";
    this.sheetTabs = document.createElement("div");
    this.sheetTabs.className = "xlcore-sheet-tabs";
    this.sheetTabs.style.cssText =
      "display:flex;gap:2px;flex:1 1 auto;min-width:0;overflow-x:auto;overflow-y:hidden;scrollbar-width:thin;";
    this.zoomBox = document.createElement("div");
    this.zoomBox.className = "xlcore-zoom";
    this.zoomBox.style.cssText =
      "margin-left:auto;display:flex;gap:4px;align-items:center;padding-right:8px;flex:none;";
    this.zoomOut = makeButton("-");
    this.zoomLabel = document.createElement("span");
    this.zoomLabel.style.cssText = "font-size:12px;min-width:42px;text-align:center;color:#374151;";
    this.zoomIn = makeButton("+");
    this.zoomBox.append(this.zoomOut, this.zoomLabel, this.zoomIn);
    this.tabs.append(this.sheetTabs, this.zoomBox);

    this.stage = document.createElement("div");
    this.stage.className = "xlcore-stage";
    this.stage.style.cssText =
      "overflow:auto;position:relative;background:#f4f4f5;min-width:0;min-height:0;width:100%;";
    this.spacer = document.createElement("div");
    this.spacer.style.position = "relative";
    this.canvas = document.createElement("canvas");
    this.canvas.style.cssText =
      "position:sticky;top:0;left:0;background:#fff;display:block;box-shadow:0 1px 3px rgba(0,0,0,0.1);";
    this.editInput = document.createElement("input");
    this.editInput.style.cssText =
      "position:absolute;top:0;left:0;display:none;z-index:5;box-sizing:border-box;margin:0;padding:0 3px;border:2px solid #2563eb;outline:none;background:#fff;color:#111827;font:13px -apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;";
    this.spacer.append(this.canvas, this.editInput);
    this.stage.append(this.spacer);
    this.editInput.addEventListener("keydown", (ev) => this.onEditInputKeyDown(ev));
    this.editInput.addEventListener("input", this.scheduleDraw);
    this.editInput.addEventListener("blur", () => this.commitEdit(null));
    if (this.editable) this.formulaBox.addEventListener("input", this.scheduleDraw);
    this.root.append(this.formulaBar, this.tabs, this.stage);
    container.append(this.root);

    this.activeSheetIndex = this.resolveInitialSheet(options.initialSheet);
    this.zoomOut.onclick = () => this.setZoom(this.zoom - 0.25);
    this.zoomIn.onclick = () => this.setZoom(this.zoom + 0.25);
    this.stage.addEventListener("scroll", this.scheduleDraw, { passive: true });
    this.canvas.addEventListener(
      "xlcore-hyperlink-jump",
      this.handleHyperlinkJump as EventListener,
    );
    window.addEventListener("xlcore-image-ready", this.scheduleDraw);
    this.resizeObserver = new ResizeObserver(() => {
      this.updateSpacerSize();
      this.scheduleDraw();
    });
    this.resizeObserver.observe(this.stage);

    this.renderTabs();
    this.attachInteractivity();
    this.updateZoomLabel();
    this.updateSpacerSize();
    this.draw();
  }

  replaceLayout(rawLayout: WorkbookLayout): void {
    this.hideEditOverlay();
    const prevIndex = this.activeSheetIndex;
    const prevScroll = { top: this.stage.scrollTop, left: this.stage.scrollLeft };
    this.layout = decodeWorkbookLayout(rawLayout);
    if (this.sheetStates.length !== this.layout.sheets.length) {
      const next = this.layout.sheets.map(
        (_, i) =>
          this.sheetStates[i] ?? {
            colOverrides: new Map<number, number>(),
            rowOverrides: new Map<number, number>(),
            activeCell: { r: 1, c: 1 },
            selection: { r1: 1, c1: 1, r2: 1, c2: 1 },
          },
      );
      this.sheetStates.length = 0;
      this.sheetStates.push(...next);
    }
    this.activeSheetIndex = Math.min(prevIndex, this.layout.sheets.length - 1);
    this.renderTabs();
    this.attachInteractivity();
    this.updateSpacerSize();
    this.stage.scrollTop = prevScroll.top;
    this.stage.scrollLeft = prevScroll.left;
    this.draw();
    this.emit("layoutchange");
  }

  destroy(): void {
    this.pivotPopover?.destroy();
    this.pivotPopover = null;
    this.tablePopover?.destroy();
    this.tablePopover = null;
    this.interactHandle?.destroy();
    this.interactHandle = null;
    this.resizeObserver.disconnect();
    this.stage.removeEventListener("scroll", this.scheduleDraw);
    this.canvas.removeEventListener(
      "xlcore-hyperlink-jump",
      this.handleHyperlinkJump as EventListener,
    );
    window.removeEventListener("xlcore-image-ready", this.scheduleDraw);
    this.root.remove();
  }

  redraw(): void {
    this.draw();
  }

  getState(): PreviewerState {
    return {
      activeSheetIndex: this.activeSheetIndex,
      activeCell: { ...this.currentState().activeCell },
      selection: { ...this.currentState().selection },
      zoom: this.zoom,
    };
  }

  getActiveSheet(): Sheet {
    return (this.layout.sheets[this.activeSheetIndex] ?? this.layout.sheets[0]!) as Sheet;
  }

  getActiveSheetIndex(): number {
    return this.activeSheetIndex;
  }

  setActiveSheet(sheet: number | string): void {
    const next = this.resolveSheet(sheet);
    if (next === this.activeSheetIndex) return;
    this.hideEditOverlay();
    this.activeSheetIndex = next;
    this.stage.scrollTop = 0;
    this.stage.scrollLeft = 0;
    this.attachInteractivity();
    this.updateActiveTab();
    this.updateSpacerSize();
    this.draw();
    this.scrollActiveTabIntoView();
    this.emit("sheetchange");
  }

  getActiveCell(): { r: number; c: number } {
    return { ...this.currentState().activeCell };
  }

  getSelection(): Selection {
    return { ...this.currentState().selection };
  }

  selectCell(r: number, c: number, options: { scroll?: boolean } = {}): void {
    this.selectRange(
      { r1: r, c1: c, r2: r, c2: c },
      { scroll: options.scroll, activeCell: { r, c } },
    );
  }

  selectRange(
    selection: Selection,
    options: { scroll?: boolean; activeCell?: { r: number; c: number } } = {},
  ): void {
    const grid = buildGrid(
      this.getActiveSheet(),
      this.currentState().colOverrides,
      this.currentState().rowOverrides,
    );
    const range = normalizeSelection(selection, grid.maxRow, grid.maxCol);
    const active = options.activeCell
      ? {
          r: clamp(Math.floor(options.activeCell.r), range.r1, range.r2),
          c: clamp(Math.floor(options.activeCell.c), range.c1, range.c2),
        }
      : { r: range.r1, c: range.c1 };
    const state = this.currentState();
    state.activeCell = active;
    state.selection = range;
    if (options.scroll) this.scrollToCell(active.r, active.c);
    this.draw();
    this.emit("selectionchange");
  }

  scrollToCell(r: number, c: number): void {
    const sheet = this.getActiveSheet();
    const state = this.currentState();
    const grid = buildGrid(sheet, state.colOverrides, state.rowOverrides);
    const rr = clamp(Math.floor(r), 1, grid.maxRow);
    const cc = clamp(Math.floor(c), 1, grid.maxCol);
    const z = this.zoom;
    const x = (grid.colX[cc] ?? 0) * z;
    const y = (grid.rowY[rr] ?? 0) * z;
    const w = (grid.colW[cc] ?? 0) * z;
    const h = (grid.rowH[rr] ?? 0) * z;
    const padX = grid.originX * z;
    const padY = grid.originY * z;
    if (x < this.stage.scrollLeft + padX) this.stage.scrollLeft = Math.max(0, x - padX);
    else if (x + w > this.stage.scrollLeft + this.stage.clientWidth) {
      this.stage.scrollLeft = x + w - this.stage.clientWidth;
    }
    if (y < this.stage.scrollTop + padY) this.stage.scrollTop = Math.max(0, y - padY);
    else if (y + h > this.stage.scrollTop + this.stage.clientHeight) {
      this.stage.scrollTop = y + h - this.stage.clientHeight;
    }
  }

  getZoom(): number {
    return this.zoom;
  }

  setZoom(zoom: number): void {
    const next = clamp(Math.round(zoom * 100) / 100, 0.25, 4);
    if (next === this.zoom) return;
    this.hideEditOverlay();
    this.zoom = next;
    this.updateZoomLabel();
    this.updateSpacerSize();
    this.draw();
    this.emit("zoomchange");
  }

  on(name: PreviewerEventName, listener: EventListener): void {
    this.addEventListener(name, listener);
  }

  off(name: PreviewerEventName, listener: EventListener): void {
    this.removeEventListener(name, listener);
  }

  private readonly scheduleDraw = () => {
    if (this.rafPending) return;
    this.rafPending = true;
    requestAnimationFrame(() => {
      this.rafPending = false;
      this.draw();
    });
  };

  private readonly handleHyperlinkJump = (event: Event) => {
    const location = (event as CustomEvent<{ location?: string }>).detail?.location;
    if (!location) return;
    const target = resolveWorkbookLocation(this.layout, location, this.activeSheetIndex);
    if (!target) return;
    this.setActiveSheet(target.sheetIndex);
    this.selectCell(target.r, target.c, { scroll: true });
  };

  private currentState(): SheetState {
    return this.sheetStates[this.activeSheetIndex] ?? this.sheetStates[0]!;
  }

  private computeHighlights(): HighlightRange[] {
    if (!this.engine) return [];
    const sheet = this.getActiveSheet();
    const active = this.editCell ?? this.currentState().activeCell;
    let text: string;
    if (this.editCell) text = this.editInput.value;
    else if (document.activeElement === this.formulaBox) text = this.formulaBox.value;
    else text = formatFormulaBar(sheet, active);
    if (!text.startsWith("=")) return [];
    const anchor = colLabel(active.c) + active.r;
    let refs: DependencyReference[];
    try {
      refs = this.engine.parseReferences(sheet.name, anchor, text);
    } catch {
      return [];
    }
    return referencesToHighlights(refs, sheet.name, HIGHLIGHT_PALETTE);
  }

  private draw(): void {
    const state = this.currentState();
    this.recomputeViewport();
    this.highlights = this.computeHighlights();
    render(this.canvas, this.getActiveSheet(), this.layout, {
      scale: window.devicePixelRatio || 1,
      zoom: this.zoom,
      colOverrides: state.colOverrides,
      rowOverrides: state.rowOverrides,
      activeCell: state.activeCell,
      selection: state.selection,
      highlights: this.highlights,
      viewport: this.viewport,
    });
    this.nameBox.textContent = formatNameBox(state.activeCell, state.selection);
    if (document.activeElement !== this.formulaBox) {
      this.formulaBox.value = formatFormulaBar(this.getActiveSheet(), state.activeCell);
    }
  }

  private attachInteractivity(): void {
    this.hideEditOverlay();
    this.interactHandle?.destroy();
    const state = this.currentState();
    this.interactHandle = attachInteractivity(this.canvas, {
      getSheet: () => this.getActiveSheet(),
      getLayout: () => this.layout,
      zoom: {
        get: () => this.zoom,
        set: (value) => {
          this.hideEditOverlay();
          this.zoom = value;
          this.updateZoomLabel();
          this.updateSpacerSize();
          this.emit("zoomchange");
        },
      },
      colOverrides: state.colOverrides,
      rowOverrides: state.rowOverrides,
      activeCell: {
        get: () => state.activeCell,
        set: (value) => {
          if (value) state.activeCell = value;
        },
      },
      selection: {
        get: () => state.selection,
        set: (value) => {
          if (value) {
            state.selection = value;
            this.emit("selectionchange");
          }
        },
      },
      scrollContainer: this.stage,
      getViewport: () => this.viewport,
      onPivotFilter: (info: PivotFilterEvent) => {
        this.dispatchEvent(new CustomEvent("pivotfilter", { detail: info }));
        if (this.pivotController) {
          if (!this.pivotPopover) {
            this.pivotPopover = createPivotFilterPopover(this.pivotController, (layout) => {
              if (layout) this.replaceLayout(layout);
              else this.scheduleDraw();
            });
          }
          this.pivotPopover.open(
            { pivot: info.pivot, field: info.field, axis: info.axis },
            info.rect,
          );
        }
      },
      onEditStart: this.editable
        ? (cell, initialText) => this.openEditOverlay(cell, initialText)
        : undefined,
      onTableFilter: (info: TableFilterEvent) => {
        this.dispatchEvent(new CustomEvent("tablefilter", { detail: info }));
        if (this.tableController) {
          if (!this.tablePopover) {
            this.tablePopover = createTableFilterPopover(this.tableController, (layout) => {
              if (layout) this.replaceLayout(layout);
              else this.scheduleDraw();
            });
          }
          this.tablePopover.open(
            { field: info.field, columnOffset: info.columnOffset, rangeRef: info.rangeRef },
            info.rect,
          );
        }
      },
      redraw: this.scheduleDraw,
    });
  }

  private renderTabs(): void {
    this.sheetTabs.replaceChildren();
    this.tabButtons.length = 0;
    this.layout.sheets.forEach((sheet, i) => {
      if (!isTabVisible(sheet, this.showHidden)) {
        this.tabButtons[i] = null;
        return;
      }
      const button = makeTab(sheet.name, sheet, this.layout);
      if (sheet.state === "hidden") {
        button.style.fontStyle = "italic";
        button.style.opacity = "0.6";
        button.title = `${sheet.name} (hidden)`;
      }
      button.onclick = () => this.setActiveSheet(i);
      this.sheetTabs.append(button);
      this.tabButtons[i] = button;
    });
    this.updateActiveTab();
  }

  private updateActiveTab(): void {
    this.tabButtons.forEach((button, i) => {
      if (!button) return;
      const active = i === this.activeSheetIndex;
      button.classList.toggle("active", active);
      button.style.fontWeight = active ? "600" : "400";

      const tab = button.dataset.tabColor;
      if (active) {
        button.style.background = "#fff";
        button.style.color = tab ?? "#111827";
      } else if (tab) {
        button.style.background = tab;
        button.style.color = contrastingTextColor(tab);
      } else {
        button.style.background = "#fff";
        button.style.color = "#111827";
      }
    });
  }

  private scrollActiveTabIntoView(): void {
    const activeButton = this.tabButtons[this.activeSheetIndex];
    if (!activeButton) return;
    activeButton.scrollIntoView({ block: "nearest", inline: "nearest" });
  }

  private recomputeViewport(): void {
    this.viewport = {
      x: this.stage.scrollLeft / this.zoom,
      y: this.stage.scrollTop / this.zoom,
      w: this.stage.clientWidth / this.zoom,
      h: this.stage.clientHeight / this.zoom,
    };
  }

  private updateSpacerSize(): void {
    const vs = virtualSize(this.getActiveSheet(), this.currentState());
    this.spacer.style.width = `${vs.w * this.zoom}px`;
    this.spacer.style.height = `${vs.h * this.zoom}px`;
  }

  private updateZoomLabel(): void {
    this.zoomLabel.textContent = `${Math.round(this.zoom * 100)}%`;
  }

  private resolveInitialSheet(sheet: number | string | undefined): number {
    if (sheet !== undefined) return this.resolveSheet(sheet);
    const active = this.layout.activeSheetIndex;
    const firstVisible = this.layout.sheets.findIndex((s) => isTabVisible(s, this.showHidden));
    const safeFallback = firstVisible >= 0 ? firstVisible : 0;
    if (
      typeof active === "number" &&
      active >= 0 &&
      active < this.layout.sheets.length &&
      isTabVisible(this.layout.sheets[active]!, this.showHidden)
    ) {
      return active;
    }
    return safeFallback;
  }

  private resolveSheet(sheet: number | string): number {
    if (typeof sheet === "number") {
      const i = Math.floor(sheet);
      if (i < 0 || i >= this.layout.sheets.length)
        throw new RangeError(`sheet index out of range: ${sheet}`);
      return i;
    }
    const i = this.layout.sheets.findIndex((s) => s.name === sheet);
    if (i < 0) throw new Error(`sheet not found: ${sheet}`);
    return i;
  }

  private emit(name: PreviewerEventName): void {
    this.dispatchEvent(new CustomEvent(name, { detail: this.getState() }));
  }

  private onFormulaBoxKeyDown(ev: KeyboardEvent): void {
    if (ev.key === "Enter") {
      ev.preventDefault();
      const active = this.getActiveCell();
      this.dispatchEvent(
        new CustomEvent("celledit", {
          detail: {
            sheetIndex: this.activeSheetIndex,
            r: active.r,
            c: active.c,
            input: this.formulaBox.value,
            commitMove: "down",
          },
        }),
      );
      this.formulaBox.blur();
    } else if (ev.key === "Escape") {
      ev.preventDefault();
      this.formulaBox.value = formatFormulaBar(this.getActiveSheet(), this.getActiveCell());
      this.formulaBox.blur();
    }
  }

  private openEditOverlay(cell: { r: number; c: number }, initialText: string | null): void {
    if (!this.editable) return;
    const sheet = this.getActiveSheet();
    const state = this.currentState();
    this.scrollToCell(cell.r, cell.c);
    const grid = buildGrid(sheet, state.colOverrides, state.rowOverrides);
    const rect = cellRect(grid, cell.r, cell.c);
    const z = this.zoom;
    this.editCell = { r: cell.r, c: cell.c };
    this.editInput.style.left = `${rect.x * z}px`;
    this.editInput.style.top = `${rect.y * z}px`;
    this.editInput.style.width = `${Math.max(rect.w * z, 24)}px`;
    this.editInput.style.height = `${Math.max(rect.h * z, 16)}px`;
    this.editInput.style.display = "block";
    this.editInput.value = initialText ?? formatFormulaBar(sheet, cell);
    this.editInput.focus({ preventScroll: true });
    const end = this.editInput.value.length;
    this.editInput.setSelectionRange(end, end);
  }

  private hideEditOverlay(): void {
    if (!this.editCell) return;
    this.editCell = null;
    this.editInput.style.display = "none";
    this.editInput.value = "";
  }

  private commitEdit(commitMove: "down" | "right" | "up" | "left" | null): void {
    const cell = this.editCell;
    if (!cell) return;
    const input = this.editInput.value;
    this.hideEditOverlay();
    this.dispatchEvent(
      new CustomEvent("celledit", {
        detail: {
          sheetIndex: this.activeSheetIndex,
          r: cell.r,
          c: cell.c,
          input,
          commitMove,
        },
      }),
    );
  }

  private onEditInputKeyDown(ev: KeyboardEvent): void {
    if (ev.key === "Enter") {
      ev.preventDefault();
      this.commitEdit(ev.shiftKey ? "up" : "down");
      this.canvas.focus({ preventScroll: true });
    } else if (ev.key === "Tab") {
      ev.preventDefault();
      this.commitEdit(ev.shiftKey ? "left" : "right");
      this.canvas.focus({ preventScroll: true });
    } else if (ev.key === "Escape") {
      ev.preventDefault();
      this.hideEditOverlay();
      this.canvas.focus({ preventScroll: true });
    }
  }
}

function virtualSize(sheet: Sheet, state: SheetState): { w: number; h: number } {
  const dw = sheet.defaultColWidthPx || 64;
  const dh = sheet.defaultRowHeightPx || 18;
  const maxCol = Math.min(16384, Math.max(sheet.maxCol + 2, sheet.maxCol + VIRTUAL_EXTRA_COLS));
  const maxRow = Math.min(1048576, Math.max(sheet.maxRow + 5, sheet.maxRow + VIRTUAL_EXTRA_ROWS));
  let w = HEADER_W + maxCol * dw;
  let h = HEADER_H + maxRow * dh;
  const colWidths = new Map<number, number>();
  for (const c of sheet.cols) {
    for (let i = c.min; i <= c.max; i++) colWidths.set(i, c.hidden ? 0 : c.widthPx);
  }
  for (const [c, v] of state.colOverrides) colWidths.set(c, Math.max(0, v));
  for (const [c, v] of colWidths) if (c >= 1 && c <= maxCol) w += v - dw;
  const rowHeights = new Map<number, number>();
  iterRows(sheet, (row) => {
    if (row.hidden) rowHeights.set(row.index, 0);
    else if (row.heightPx !== undefined) rowHeights.set(row.index, row.heightPx);
  });
  for (const [r, v] of state.rowOverrides) rowHeights.set(r, Math.max(0, v));
  for (const [r, v] of rowHeights) if (r >= 1 && r <= maxRow) h += v - dh;
  return { w, h };
}

function normalizeSelection(selection: Selection, maxRow: number, maxCol: number): Selection {
  const r1 = clamp(Math.floor(Math.min(selection.r1, selection.r2)), 1, maxRow);
  const r2 = clamp(Math.floor(Math.max(selection.r1, selection.r2)), 1, maxRow);
  const c1 = clamp(Math.floor(Math.min(selection.c1, selection.c2)), 1, maxCol);
  const c2 = clamp(Math.floor(Math.max(selection.c1, selection.c2)), 1, maxCol);
  return { r1, c1, r2, c2 };
}

function formatNameBox(active: { r: number; c: number }, selection: Selection): string {
  if (selection.r1 !== selection.r2 || selection.c1 !== selection.c2) {
    return `${colLabel(active.c)}${active.r}  (${selection.r2 - selection.r1 + 1}R×${selection.c2 - selection.c1 + 1}C)`;
  }
  return colLabel(active.c) + active.r;
}

function formatFormulaBar(sheet: Sheet, active: { r: number; c: number }): string {
  const cell = findCell(sheet, active.r, active.c);
  if (!cell) return "";
  if (cell.formula) return cell.formula.startsWith("=") ? cell.formula : `=${cell.formula}`;
  if (cell.value !== undefined) return String(cell.value);
  if (cell.runs && cell.runs.length > 0) return cell.runs.map((run) => run.text).join("");
  return "";
}

function colLabel(n: number): string {
  let s = "";
  let cur = Math.max(1, Math.floor(n));
  while (cur > 0) {
    const r = (cur - 1) % 26;
    s = String.fromCharCode(65 + r) + s;
    cur = Math.floor((cur - 1) / 26);
  }
  return s;
}

function makeButton(label: string): HTMLButtonElement {
  const button = document.createElement("button");
  button.textContent = label;
  button.style.cssText =
    "background:#fff;border:1px solid #d1d5db;padding:4px 10px;cursor:pointer;font:inherit;font-size:12px;border-radius:4px;";
  return button;
}

function makeTab(label: string, sheet: WireSheet, layout: WorkbookLayout): HTMLButtonElement {
  const button = document.createElement("button");
  button.textContent = label;
  let bg = "#fff";
  let fg = "#111827";
  if (sheet.tabColor) {
    const tab = colorToCssWithTheme(sheet.tabColor, layout.theme, "#9ca3af");
    button.dataset.tabColor = tab;
    bg = tab;
    fg = contrastingTextColor(tab);
  }
  button.style.cssText = `flex:none;background:${bg};color:${fg};border:1px solid #d1d5db;border-bottom:none;padding:6px 14px;cursor:pointer;font:inherit;font-size:12px;white-space:nowrap;`;
  return button;
}

function contrastingTextColor(css: string): string {
  if (css.length !== 7 || css[0] !== "#") return "#111827";
  const r = parseInt(css.slice(1, 3), 16);
  const g = parseInt(css.slice(3, 5), 16);
  const b = parseInt(css.slice(5, 7), 16);
  const luma = (r * 299 + g * 587 + b * 114) / 1000;
  return luma > 140 ? "#111827" : "#ffffff";
}

function resolveWorkbookLocation(
  layout: WorkbookLayout,
  rawLocation: string,
  activeSheetIndex: number,
): { sheetIndex: number; r: number; c: number } | null {
  const location = rawLocation.trim().replace(/^#/, "");
  const direct = parseSheetCellLocation(location, layout, activeSheetIndex);
  if (direct) return direct;

  const wanted = location.toLocaleLowerCase();
  const names = layout.definedNames ?? [];
  const local = names.find(
    (n) => n.name.toLocaleLowerCase() === wanted && n.localSheetId === activeSheetIndex,
  );
  const global = names.find(
    (n) => n.name.toLocaleLowerCase() === wanted && n.localSheetId === undefined,
  );
  const named = local ?? global;
  if (!named) return null;
  return parseSheetCellLocation(named.formula, layout, named.localSheetId ?? activeSheetIndex);
}

function parseSheetCellLocation(
  raw: string,
  layout: WorkbookLayout,
  fallbackSheetIndex: number,
): { sheetIndex: number; r: number; c: number } | null {
  const ref = raw.trim().replace(/^=/, "");
  const bang = findUnquotedBang(ref);
  let sheetIndex = fallbackSheetIndex;
  let addr = ref;
  if (bang >= 0) {
    const sheetName = unquoteSheetName(ref.slice(0, bang));
    const idx = layout.sheets.findIndex((s) => s.name === sheetName);
    if (idx < 0) return null;
    sheetIndex = idx;
    addr = ref.slice(bang + 1);
  }
  const m = addr.match(/\$?([A-Za-z]{1,3})\$?(\d+)/);
  if (!m) return null;
  return { sheetIndex, r: Number(m[2]), c: colNameToIndex(m[1]!) };
}

function findUnquotedBang(s: string): number {
  let quoted = false;
  for (let i = 0; i < s.length; i++) {
    const ch = s[i];
    if (ch === "'") {
      if (quoted && s[i + 1] === "'") i++;
      else quoted = !quoted;
    } else if (ch === "!" && !quoted) return i;
  }
  return -1;
}

function unquoteSheetName(s: string): string {
  const trimmed = s.trim();
  if (trimmed.startsWith("'") && trimmed.endsWith("'")) {
    return trimmed.slice(1, -1).replace(/''/g, "'");
  }
  return trimmed;
}

function colNameToIndex(s: string): number {
  let n = 0;
  for (const ch of s.toUpperCase()) n = n * 26 + (ch.charCodeAt(0) - 64);
  return n;
}

function clamp(v: number, lo: number, hi: number): number {
  return v < lo ? lo : v > hi ? hi : v;
}
