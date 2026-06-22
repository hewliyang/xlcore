import { decodeWorkbookLayout } from "./columnar.js";
import { clamp } from "./mathUtils.js";
import { balanceFormula, formatFormulaBar } from "./formulaText.js";
import { patchWorkbookSheet } from "./layoutPatch.js";
import type { LoadReport } from "./errors.js";
import {
  attachInteractivity,
  type InteractHandle,
  type PivotFilterEvent,
  type TableFilterEvent,
  type ValidationPickEvent,
  type Selection,
} from "./interact.js";
import { buildGrid, render } from "./render.js";
import { anchorToRect, colLabel } from "./grid.js";
import { frozenDims } from "./panes.js";
import { formatNameBox, resolveWorkbookLocation } from "./previewerRefs.js";
import { CellEditor, type CellEditorHost } from "./cellEditor.js";
import { rangeA1 } from "./api-refs.js";
import { buildDrawingMovedDetail } from "./anchorConvert.js";
import { parseClipboard, readRangeValues, serializeRange } from "./clipboardModel.js";
import { projectFill } from "./fillModel.js";
import { writeClipboard, readClipboard } from "./clipboardIo.js";
import { referencesToHighlights } from "./highlights.js";
import {
  createAutocompletePopover,
  type AutocompletePopoverHandle,
} from "./autocompletePopover.js";
import { createSignatureTip, type SignatureTipHandle } from "./signatureTip.js";
import {
  createValidationDropdown,
  type ValidationDropdownHandle,
} from "./validationDropdown.js";
import type { HighlightRange } from "./renderTypes.js";
import type { DependencyReference } from "./api-schema/DependencyReference.js";
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
import {
  contrastingTextColor,
  makeButton,
  makeTab,
  normalizeSelection,
  virtualSize,
} from "./previewerChrome.js";
import type { Sheet as WireSheet } from "./schema/Sheet.js";
import type { Sheet, WorkbookLayout } from "./types.js";

export interface PreviewerOptions {
  initialSheet?: number | string;
  initialZoom?: number;
  className?: string;
  report?: LoadReport;
  showHidden?: boolean;
  editable?: boolean;
  onDownload?: () => void | Promise<void>;

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

export type {
  PivotFilterController,
  PivotFilterContext,
} from "./pivotFilterPopover.js";
export type {
  TableFilterController,
  TableFilterContext,
} from "./tableFilterPopover.js";

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
  | "celledit"
  | "drawingmoved"
  | "drawingdeleted"
  | "rangecopy"
  | "rangecut"
  | "rangepaste"
  | "imagepaste"
  | "rangefill"
  | "cellclear"
  | "sheetadd";

export type { PivotFilterEvent, TableFilterEvent, ValidationPickEvent } from "./interact.js";

export interface WorkbookPreviewer {
  readonly root: HTMLElement;
  readonly canvas: HTMLCanvasElement;
  readonly layout: WorkbookLayout;

  readonly report?: LoadReport;
  destroy(): void;
  redraw(): void;
  replaceLayout(layout: WorkbookLayout): void;
  patchSheetLayout(layout: WorkbookLayout): void;
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
  activeCell: { r: number; c: number } | null;
  selection: Selection | null;
  selectedDrawing: number | null;
}

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
  private warnedNoPivotController = false;
  private warnedNoTableController = false;

  private readonly tabs: HTMLDivElement;
  private readonly sheetTabs: HTMLDivElement;
  private readonly formulaBar: HTMLDivElement;
  private readonly zoomBox: HTMLDivElement;
  private readonly downloadButton: HTMLButtonElement | null;
  private readonly nameBox: HTMLDivElement;
  private readonly formulaBox: HTMLInputElement;
  private readonly zoomLabel: HTMLSpanElement;
  private readonly zoomOut: HTMLButtonElement;
  private readonly zoomIn: HTMLButtonElement;
  private readonly stage: HTMLDivElement;
  private readonly spacer: HTMLDivElement;

  private readonly editor: CellEditor;
  private readonly sheetStates: SheetState[];
  private readonly tabButtons: Array<HTMLButtonElement | null> = [];
  private readonly showHidden: boolean;
  private readonly editable: boolean;
  private cutRange: Selection | null = null;
  private readonly onDownload?: () => void | Promise<void>;
  private readonly engine?: PreviewerEngine;
  private highlights: HighlightRange[] = [];
  private functionNamesCache: string[] | null = null;
  private readonly autocomplete: AutocompletePopoverHandle;
  private readonly validation: ValidationDropdownHandle;
  private readonly signature: SignatureTipHandle;
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
    this.onDownload = options.onDownload;
    this.engine = options.engine;
    this.sheetStates = this.layout.sheets.map(() => ({
      colOverrides: new Map(),
      rowOverrides: new Map(),
      activeCell: { r: 1, c: 1 },
      selection: { r1: 1, c1: 1, r2: 1, c2: 1 },
      selectedDrawing: null,
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
      this.formulaBox.addEventListener("focus", () => {
        this.editor.disarmPointMode();
        this.scheduleDraw();
      });
      this.formulaBox.addEventListener("blur", () => {
        this.autocomplete.scheduleClose();
        this.signature.scheduleClose();
        this.scheduleDraw();
      });
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
    if (this.onDownload) {
      this.downloadButton = makeButton("Download");
      this.downloadButton.setAttribute("aria-label", "Download workbook");
      this.downloadButton.onclick = () => {
        void this.onDownload?.();
      };
      this.zoomBox.insertBefore(this.downloadButton, this.zoomOut);
    } else {
      this.downloadButton = null;
    }
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
    this.autocomplete = createAutocompletePopover({
      getFunctionNames: () => this.getFunctionNames(),
      onAccept: (input) => {
        input.dispatchEvent(new Event("input"));
      },
    });
    this.validation = createValidationDropdown({
      getEditInput: () => this.editor.getEditInput(),
      getSheet: () => this.getActiveSheet(),
      onAccept: (value) => {
        this.editor.setEditText(value);
        this.editor.commitEdit(null);
        this.canvas.focus({ preventScroll: true });
      },
    });
    this.signature = createSignatureTip({
      isBlocked: () => this.autocomplete.isOpen(),
    });
    const editorHost: CellEditorHost = {
      editable: this.editable,
      getSheet: () => this.getActiveSheet(),
      getColOverrides: () => this.currentState().colOverrides,
      getRowOverrides: () => this.currentState().rowOverrides,
      getActiveCellState: () => this.currentState().activeCell,
      getZoom: () => this.zoom,
      getActiveSheetIndex: () => this.activeSheetIndex,
      getFormulaBox: () => this.formulaBox,
      getStageScrollLeft: () => this.stage.scrollLeft,
      getStageScrollTop: () => this.stage.scrollTop,
      getStageClientWidth: () => this.stage.clientWidth,
      scrollToCell: (r, c) => this.scrollToCell(r, c),
      scheduleDraw: () => this.scheduleDraw(),
      focusCanvas: () => this.canvas.focus({ preventScroll: true }),
      emitCellEdit: (detail) => {
        this.dispatchEvent(new CustomEvent("celledit", { detail }));
      },
    };
    this.editor = new CellEditor({
      host: editorHost,
      autocomplete: this.autocomplete,
      signature: this.signature,
      validation: this.validation,
    });
    this.spacer.append(this.canvas, this.editor.getEditInput());
    this.stage.append(this.spacer);
    if (this.editable) {
      this.formulaBox.addEventListener("input", () => {
        this.editor.armPointMode(this.formulaBox);
        this.autocomplete.update(this.formulaBox);
        this.signature.update(this.formulaBox);
        this.scheduleDraw();
      });
      this.formulaBox.addEventListener("keyup", () => this.signature.update(this.formulaBox));
      this.formulaBox.addEventListener("mousedown", () => {
        this.editor.disarmPointMode();
      });
      this.formulaBox.addEventListener("click", () => this.signature.update(this.formulaBox));
    }
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
    this.editor.hideEditOverlay();
    const prevIndex = this.activeSheetIndex;
    const prevScroll = {
      top: this.stage.scrollTop,
      left: this.stage.scrollLeft,
    };
    this.layout = decodeWorkbookLayout(rawLayout);
    if (this.sheetStates.length !== this.layout.sheets.length) {
      const next = this.layout.sheets.map(
        (_, i) =>
          this.sheetStates[i] ?? {
            colOverrides: new Map<number, number>(),
            rowOverrides: new Map<number, number>(),
            activeCell: { r: 1, c: 1 },
            selection: { r1: 1, c1: 1, r2: 1, c2: 1 },
            selectedDrawing: null,
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

  patchSheetLayout(rawLayout: WorkbookLayout): void {
    if (!patchWorkbookSheet(this.layout, rawLayout)) {
      this.replaceLayout(rawLayout);
      return;
    }
    this.scheduleDraw();
    this.emit("layoutchange");
  }

  destroy(): void {
    this.pivotPopover?.destroy();
    this.pivotPopover = null;
    this.tablePopover?.destroy();
    this.tablePopover = null;
    this.interactHandle?.destroy();
    this.interactHandle = null;
    this.autocomplete.destroy();
    this.signature.destroy();
    this.validation.destroy();
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
      activeCell: this.getActiveCell(),
      selection: this.getSelection(),
      zoom: this.zoom,
    };
  }

  getActiveSheet(): Sheet {
    return (this.layout.sheets[this.activeSheetIndex] ?? this.layout.sheets[0]!) as Sheet;
  }

  private handleCopy(sel: Selection, isCut: boolean): void {
    const sheetName = this.getActiveSheet().name;
    void writeClipboard(serializeRange(this.layout, sheetName, sel));
    this.cutRange = isCut ? sel : null;
    this.dispatchEvent(
      new CustomEvent(isCut ? "rangecut" : "rangecopy", {
        detail: { selection: sel, cut: isCut },
      }),
    );
  }

  private async handlePaste(target: { r: number; c: number }): Promise<void> {
    const clip = await readClipboard();
    if (clip.imageBytes) {
      this.dispatchEvent(
        new CustomEvent("imagepaste", {
          detail: { target, bytes: clip.imageBytes, mime: clip.imageType },
        }),
      );
      return;
    }
    const parsed = parseClipboard(clip);
    const cutRange = this.cutRange;
    this.cutRange = null;
    this.dispatchEvent(
      new CustomEvent("rangepaste", {
        detail: {
          target,
          values: parsed.values,
          formulas: parsed.formulas,
          source: parsed.source,
          sourceSheet: parsed.sourceSheet,
          sourceRange: parsed.sourceRange,
          cutRange,
        },
      }),
    );
  }

  private handleFill(source: Selection, target: Selection): void {
    const sheetName = this.getActiveSheet().name;
    const values = readRangeValues(this.layout, sheetName, source);
    const projected = projectFill(values, target, source);
    this.dispatchEvent(new CustomEvent("rangefill", { detail: { target, values: projected } }));
  }

  private handleClear(sel: Selection): void {
    const r1 = Math.min(sel.r1, sel.r2);
    const c1 = Math.min(sel.c1, sel.c2);
    const ref = rangeA1(r1, c1, Math.abs(sel.r2 - sel.r1) + 1, Math.abs(sel.c2 - sel.c1) + 1);
    this.dispatchEvent(
      new CustomEvent("cellclear", {
        detail: { sheetIndex: this.activeSheetIndex, ref },
      }),
    );
  }

  getActiveSheetIndex(): number {
    return this.activeSheetIndex;
  }

  setActiveSheet(sheet: number | string): void {
    const next = this.resolveSheet(sheet);
    if (next === this.activeSheetIndex) return;
    this.currentState().selectedDrawing = null;
    this.editor.hideEditOverlay();
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
    const active = this.currentState().activeCell;
    return active ? { ...active } : { r: 1, c: 1 };
  }

  getSelection(): Selection {
    const sel = this.currentState().selection;
    return sel ? { ...sel } : { r1: 1, c1: 1, r2: 1, c2: 1 };
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
    state.selectedDrawing = null;
    if (options.scroll) this.scrollToCell(active.r, active.c);
    this.draw();
    this.emit("selectionchange");
  }

  scrollToCell(r: number, c: number): void {
    const sheet = this.getActiveSheet();
    const state = this.currentState();
    state.selectedDrawing = null;
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
    const { splitX, splitY, pcw, prh } = frozenDims(sheet, grid);
    if (cc >= splitX) {
      const frozenX = pcw * z;
      if (x < this.stage.scrollLeft + padX + frozenX) {
        this.stage.scrollLeft = Math.max(0, x - padX - frozenX);
      } else if (x + w > this.stage.scrollLeft + this.stage.clientWidth) {
        this.stage.scrollLeft = x + w - this.stage.clientWidth;
      }
    }
    if (rr >= splitY) {
      const frozenY = prh * z;
      if (y < this.stage.scrollTop + padY + frozenY) {
        this.stage.scrollTop = Math.max(0, y - padY - frozenY);
      } else if (y + h > this.stage.scrollTop + this.stage.clientHeight) {
        this.stage.scrollTop = y + h - this.stage.clientHeight;
      }
    }
  }

  getZoom(): number {
    return this.zoom;
  }

  setZoom(zoom: number): void {
    const next = clamp(Math.round(zoom * 100) / 100, 0.25, 4);
    if (next === this.zoom) return;
    this.editor.hideEditOverlay();
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
    const editCell = this.editor.getEditCell();
    const active = editCell ?? this.currentState().activeCell;
    if (!active) return [];
    let text: string;
    if (editCell) text = this.editor.getEditText();
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
    const baseHighlights = this.computeHighlights();
    const pointHighlight = this.editor.getPointHighlight();
    this.highlights = pointHighlight ? [...baseHighlights, pointHighlight] : baseHighlights;
    const sheet = this.getActiveSheet();
    let selectedDrawingRect = null;
    if (state.selectedDrawing != null) {
      const d = sheet.drawings?.[state.selectedDrawing];
      if (d) {
        const grid = buildGrid(sheet, state.colOverrides, state.rowOverrides);
        selectedDrawingRect = anchorToRect(d, grid);
      }
    }
    render(this.canvas, sheet, this.layout, {
      scale: window.devicePixelRatio || 1,
      zoom: this.zoom,
      colOverrides: state.colOverrides,
      rowOverrides: state.rowOverrides,
      activeCell: state.activeCell,
      selection: state.selection,
      highlights: this.highlights,
      selectedDrawingRect,
      viewport: this.viewport,
    });
    this.nameBox.textContent =
      state.activeCell && state.selection
        ? formatNameBox(state.activeCell, state.selection, this.layout, this.activeSheetIndex)
        : "";
    if (document.activeElement !== this.formulaBox) {
      this.formulaBox.value = state.activeCell
        ? formatFormulaBar(this.getActiveSheet(), state.activeCell)
        : "";
    }
  }

  private attachInteractivity(): void {
    this.editor.hideEditOverlay();
    this.interactHandle?.destroy();
    const state = this.currentState();
    this.interactHandle = attachInteractivity(this.canvas, {
      getSheet: () => this.getActiveSheet(),
      getLayout: () => this.layout,
      zoom: {
        get: () => this.zoom,
        set: (value) => {
          this.editor.hideEditOverlay();
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
          state.activeCell = value;
        },
      },
      selection: {
        get: () => state.selection,
        set: (value) => {
          state.selection = value;
          this.emit("selectionchange");
        },
      },
      selectedDrawing: {
        get: () => state.selectedDrawing,
        set: (value) => {
          state.selectedDrawing = value;
        },
      },
      scrollContainer: this.stage,
      getViewport: () => this.viewport,
      onPivotFilter: (info: PivotFilterEvent) => {
        this.dispatchEvent(new CustomEvent("pivotfilter", { detail: info }));
        if (this.editable && !this.pivotController && !this.warnedNoPivotController) {
          this.warnedNoPivotController = true;
          console.warn(
            "xlsx-preview: pivot filter clicked but no pivotController is wired; pass recalcWorkbook.pivotController to enable it.",
          );
        }
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
        ? (cell, initialText) => this.editor.openEditOverlay(cell, initialText)
        : undefined,
      onCopy: this.editable ? (sel, isCut) => this.handleCopy(sel, isCut) : undefined,
      onPaste: this.editable ? (target) => void this.handlePaste(target) : undefined,
      onFill: this.editable ? (source, target) => this.handleFill(source, target) : undefined,

      onClear: this.editable ? (sel) => this.handleClear(sel) : undefined,
      isPointModeActive: this.editable ? () => this.editor.isPointModeActive() : undefined,
      onPointModeRef: this.editable ? (ref, o) => this.editor.applyPointModeRef(ref, o) : undefined,
      onTableFilter: (info: TableFilterEvent) => {
        this.dispatchEvent(new CustomEvent("tablefilter", { detail: info }));
        if (this.editable && !this.tableController && !this.warnedNoTableController) {
          this.warnedNoTableController = true;
          console.warn(
            "xlsx-preview: table filter clicked but no tableController is wired; pass recalcWorkbook.tableController to enable it.",
          );
        }
        if (this.tableController) {
          if (!this.tablePopover) {
            this.tablePopover = createTableFilterPopover(this.tableController, (layout) => {
              if (layout) this.replaceLayout(layout);
              else this.scheduleDraw();
            });
          }
          this.tablePopover.open(
            {
              field: info.field,
              columnOffset: info.columnOffset,
              rangeRef: info.rangeRef,
            },
            info.rect,
          );
        }
      },
      onValidationPick: this.editable
        ? (info: ValidationPickEvent) => this.openValidationEdit(info)
        : undefined,
      onDrawingMoved: ({ index, prevAnchor, anchor }) => {
        const sheet = this.getActiveSheet();
        this.dispatchEvent(
          new CustomEvent("drawingmoved", {
            detail: buildDrawingMovedDetail(
              sheet.name,
              sheet.drawings?.[index]?.kind,
              index,
              prevAnchor,
              anchor,
            ),
          }),
        );
      },
      onDrawingDelete: ({ index }) => {
        const sheet = this.getActiveSheet();
        const d = sheet.drawings?.[index];
        if (!d) return;
        this.currentState().selectedDrawing = null;
        this.dispatchEvent(
          new CustomEvent("drawingdeleted", {
            detail: buildDrawingMovedDetail(
              sheet.name,
              d.kind,
              index,
              d.anchor,
              d.anchor,
            ),
          }),
        );
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
    if (this.editable) {
      const add = makeButton("+");
      add.setAttribute("aria-label", "New sheet");
      add.title = "New sheet";
      add.style.cssText +=
        "flex:none;align-self:center;font-weight:600;line-height:1;padding:4px 10px;";
      add.onclick = () => {
        this.dispatchEvent(
          new CustomEvent("sheetadd", { detail: { name: this.nextSheetName() } }),
        );
      };
      this.sheetTabs.append(add);
    }
    this.updateActiveTab();
  }

  private nextSheetName(): string {
    const existing = new Set(this.layout.sheets.map((s) => s.name.toLocaleLowerCase()));
    let n = this.layout.sheets.length + 1;
    let name = `Sheet${n}`;
    while (existing.has(name.toLocaleLowerCase())) {
      n++;
      name = `Sheet${n}`;
    }
    return name;
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

  private getFunctionNames(): string[] {
    if (!this.engine) return [];
    if (this.functionNamesCache === null) {
      try {
        this.functionNamesCache = this.engine.functionNames();
      } catch {
        this.functionNamesCache = [];
      }
    }
    return this.functionNamesCache;
  }

  private onFormulaBoxKeyDown(ev: KeyboardEvent): void {
    if (this.autocomplete.handleKey(ev)) return;
    if (this.editor.handlePointKeyboardKey(ev)) return;
    this.editor.resetPointSpanOnType(ev);
    if (ev.key === "Enter") {
      ev.preventDefault();
      const active = this.getActiveCell();
      this.dispatchEvent(
        new CustomEvent("celledit", {
          detail: {
            sheetIndex: this.activeSheetIndex,
            r: active.r,
            c: active.c,
            input: balanceFormula(this.formulaBox.value),
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

  private openValidationEdit(info: ValidationPickEvent): void {
    if (!this.editable) return;
    this.selectCell(info.r, info.c);
    setTimeout(() => {
      this.editor.openEditOverlay({ r: info.r, c: info.c }, null);
      this.validation.refresh(false);
    }, 0);
  }
}


