import { decodeWorkbookLayout, findCell, iterRows } from "./columnar.js";
import { attachInteractivity, type InteractHandle, type Selection } from "./interact.js";
import { HEADER_H, HEADER_W, buildGrid, render } from "./render.js";
import type { Sheet, WorkbookLayout } from "./types.js";

export interface PreviewerOptions {
  initialSheet?: number | string;
  initialZoom?: number;
  className?: string;
}

export interface PreviewerState {
  activeSheetIndex: number;
  activeCell: { r: number; c: number };
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
  readonly layout: WorkbookLayout;

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
  private readonly sheetStates: SheetState[];
  private readonly tabButtons: HTMLButtonElement[] = [];
  private readonly resizeObserver: ResizeObserver;
  private interactHandle: InteractHandle | null = null;
  private activeSheetIndex = 0;
  private zoom = 1;
  private viewport = { x: 0, y: 0, w: 0, h: 0 };
  private rafPending = false;

  constructor(container: HTMLElement, rawLayout: WorkbookLayout, options: PreviewerOptions) {
    super();
    this.layout = decodeWorkbookLayout(rawLayout);
    this.zoom = clamp(options.initialZoom ?? 1, 0.25, 4);
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
    this.formulaBox.readOnly = true;
    this.formulaBox.setAttribute("aria-label", "Formula or value");
    this.formulaBox.style.cssText =
      "min-width:0;flex:1;height:28px;padding:0 9px;border:1px solid #d1d5db;border-radius:4px;background:#fff;color:#111827;font:12px ui-monospace,SFMono-Regular,Menlo,monospace;";
    this.formulaBar.append(this.nameBox, fxLabel, this.formulaBox);

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
    this.spacer.append(this.canvas);
    this.stage.append(this.spacer);
    this.root.append(this.formulaBar, this.tabs, this.stage);
    container.append(this.root);

    this.activeSheetIndex = this.resolveInitialSheet(options.initialSheet);
    this.zoomOut.onclick = () => this.setZoom(this.zoom - 0.25);
    this.zoomIn.onclick = () => this.setZoom(this.zoom + 0.25);
    this.stage.addEventListener("scroll", this.scheduleDraw, { passive: true });
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

  destroy(): void {
    this.interactHandle?.destroy();
    this.interactHandle = null;
    this.resizeObserver.disconnect();
    this.stage.removeEventListener("scroll", this.scheduleDraw);
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

  private currentState(): SheetState {
    return this.sheetStates[this.activeSheetIndex] ?? this.sheetStates[0]!;
  }

  private draw(): void {
    const state = this.currentState();
    this.recomputeViewport();
    render(this.canvas, this.getActiveSheet(), this.layout, {
      scale: window.devicePixelRatio || 1,
      zoom: this.zoom,
      colOverrides: state.colOverrides,
      rowOverrides: state.rowOverrides,
      activeCell: state.activeCell,
      selection: state.selection,
      viewport: this.viewport,
    });
    this.nameBox.textContent = formatNameBox(state.activeCell, state.selection);
    this.formulaBox.value = formatFormulaBar(this.getActiveSheet(), state.activeCell);
  }

  private attachInteractivity(): void {
    this.interactHandle?.destroy();
    const state = this.currentState();
    this.interactHandle = attachInteractivity(this.canvas, {
      getSheet: () => this.getActiveSheet(),
      getLayout: () => this.layout,
      zoom: {
        get: () => this.zoom,
        set: (value) => {
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
      redraw: this.scheduleDraw,
    });
  }

  private renderTabs(): void {
    this.sheetTabs.replaceChildren();
    this.tabButtons.length = 0;
    this.layout.sheets.forEach((sheet, i) => {
      const button = makeTab(sheet.name);
      button.onclick = () => this.setActiveSheet(i);
      this.sheetTabs.append(button);
      this.tabButtons.push(button);
    });
    this.updateActiveTab();
  }

  private updateActiveTab(): void {
    this.tabButtons.forEach((button, i) => {
      button.classList.toggle("active", i === this.activeSheetIndex);
      button.style.fontWeight = i === this.activeSheetIndex ? "600" : "400";
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
    return typeof active === "number" && active >= 0 && active < this.layout.sheets.length
      ? active
      : 0;
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

function makeTab(label: string): HTMLButtonElement {
  const button = document.createElement("button");
  button.textContent = label;
  button.style.cssText =
    "flex:none;background:#fff;border:1px solid #d1d5db;border-bottom:none;padding:6px 14px;cursor:pointer;font:inherit;font-size:12px;white-space:nowrap;";
  return button;
}

function clamp(v: number, lo: number, hi: number): number {
  return v < lo ? lo : v > hi ? hi : v;
}
