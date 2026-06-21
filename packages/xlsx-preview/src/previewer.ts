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
import {
  formatNameBox,
  parsePointHighlight,
  resolveWorkbookLocation,
} from "./previewerRefs.js";
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
import { lookupSignature, signatureAt } from "./formulaSignature.js";
import { applyReferenceAtCaret, caretAcceptsReference, type RefSpan } from "./formulaPointMode.js";
import type { HighlightRange } from "./renderTypes.js";
import type { DependencyReference } from "./api-schema/DependencyReference.js";
import { buildMergeMaps, rectFor } from "./geometry.js";
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

  private readonly editInput: HTMLInputElement;
  private editCell: { r: number; c: number } | null = null;
  private editEnterMode = false;
  private pointModeArmed = false;
  private editBaseLeft = 0;
  private editBaseWidth = 0;
  private pointKeyAnchor: { r: number; c: number } | null = null;
  private pointKeyCursor: { r: number; c: number } | null = null;
  private readonly sheetStates: SheetState[];
  private readonly tabButtons: Array<HTMLButtonElement | null> = [];
  private readonly showHidden: boolean;
  private readonly editable: boolean;
  private cutRange: Selection | null = null;
  private readonly onDownload?: () => void | Promise<void>;
  private readonly engine?: PreviewerEngine;
  private highlights: HighlightRange[] = [];
  private pointHighlight: HighlightRange | null = null;
  private functionNamesCache: string[] | null = null;
  private readonly autocomplete: AutocompletePopoverHandle;
  private readonly validationMenu: HTMLDivElement;
  private validationOptions: string[] | null = null;
  private validationFiltered: string[] = [];
  private validationActive = 0;
  private validationTyped = false;
  private readonly signatureTip: HTMLDivElement;
  private signatureFor: HTMLInputElement | null = null;
  private signatureBlurTimer: ReturnType<typeof setTimeout> | null = null;
  private activeRefSpan: RefSpan | null = null;
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
        this.pointModeArmed = false;
        this.scheduleDraw();
      });
      this.formulaBox.addEventListener("blur", () => {
        this.autocomplete.scheduleClose();
        this.scheduleSignatureTipClose();
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
    this.editInput = document.createElement("input");
    this.editInput.style.cssText =
      "position:absolute;top:0;left:0;display:none;z-index:5;box-sizing:border-box;margin:0;padding:0 3px;border:2px solid #2563eb;outline:none;background:#fff;color:#111827;font:13px -apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;";
    this.spacer.append(this.canvas, this.editInput);
    this.stage.append(this.spacer);
    this.autocomplete = createAutocompletePopover({
      getFunctionNames: () => this.getFunctionNames(),
      onAccept: (input) => {
        input.dispatchEvent(new Event("input"));
      },
    });
    this.validationMenu = document.createElement("div");
    this.validationMenu.style.cssText =
      "position:fixed;z-index:1100;display:none;background:#fff;border:1px solid #d4d4d8;border-radius:6px;" +
      "box-shadow:0 8px 24px rgba(15,23,42,0.18);padding:4px;min-width:120px;max-height:240px;overflow:auto;" +
      "font:13px -apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;";
    document.body.append(this.validationMenu);
    this.signatureTip = document.createElement("div");
    this.signatureTip.style.cssText =
      "position:fixed;z-index:1099;display:none;background:#fff;border:1px solid #d4d4d8;border-radius:4px;" +
      "box-shadow:0 4px 12px rgba(15,23,42,0.12);padding:6px 10px;max-width:480px;" +
      "font:12px ui-monospace,SFMono-Regular,Menlo,monospace;color:#111827;";
    document.body.append(this.signatureTip);
    this.editInput.addEventListener("keydown", (ev) => this.onEditInputKeyDown(ev));
    this.editInput.addEventListener("input", () => {
      this.armPointMode(this.editInput);
      this.autocomplete.update(this.editInput);
      this.updateSignatureTip(this.editInput);
      if (this.validationOptions) {
        this.validationTyped = true;
        this.validationActive = 0;
        this.updateValidationMenu();
      }
      this.growEditInput();
      this.scheduleDraw();
    });
    this.editInput.addEventListener("keyup", () => this.updateSignatureTip(this.editInput));
    this.editInput.addEventListener("mousedown", () => {
      this.pointModeArmed = false;
    });
    this.editInput.addEventListener("click", () => this.updateSignatureTip(this.editInput));
    this.editInput.addEventListener("blur", () => {
      this.autocomplete.scheduleClose();
      this.scheduleSignatureTipClose();
      this.commitEdit(null);
    });
    if (this.editable) {
      this.formulaBox.addEventListener("input", () => {
        this.armPointMode(this.formulaBox);
        this.autocomplete.update(this.formulaBox);
        this.updateSignatureTip(this.formulaBox);
        this.scheduleDraw();
      });
      this.formulaBox.addEventListener("keyup", () => this.updateSignatureTip(this.formulaBox));
      this.formulaBox.addEventListener("mousedown", () => {
        this.pointModeArmed = false;
      });
      this.formulaBox.addEventListener("click", () => this.updateSignatureTip(this.formulaBox));
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
    this.hideEditOverlay();
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
    this.closeValidationMenu();
    this.hideSignatureTip();
    this.validationMenu.remove();
    this.signatureTip.remove();
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
    if (!active) return [];
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
    const baseHighlights = this.computeHighlights();
    this.highlights = this.pointHighlight
      ? [...baseHighlights, this.pointHighlight]
      : baseHighlights;
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
      onCopy: this.editable ? (sel, isCut) => this.handleCopy(sel, isCut) : undefined,
      onPaste: this.editable ? (target) => void this.handlePaste(target) : undefined,
      onFill: this.editable ? (source, target) => this.handleFill(source, target) : undefined,

      onClear: this.editable ? (sel) => this.handleClear(sel) : undefined,
      isPointModeActive: this.editable ? () => this.isPointModeActive() : undefined,
      onPointModeRef: this.editable ? (ref, o) => this.applyPointModeRef(ref, o) : undefined,
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

  private updateSignatureTip(input: HTMLInputElement): void {
    if (this.autocomplete.isOpen()) return this.hideSignatureTip();
    const caret = input.selectionStart;
    if (caret === null) return this.hideSignatureTip();
    const ctx = signatureAt(input.value, caret);
    if (!ctx) return this.hideSignatureTip();
    const sig = lookupSignature(ctx.name);
    if (!sig) return this.hideSignatureTip();
    this.signatureFor = input;
    this.renderSignatureTip(input, sig, ctx.argIndex);
  }

  private renderSignatureTip(
    input: HTMLInputElement,
    sig: { name: string; args: string[]; summary: string },
    argIndex: number,
  ): void {
    const tip = this.signatureTip;
    tip.replaceChildren();

    const sigLine = document.createElement("div");
    sigLine.style.cssText = "margin:0 0 6px 0;line-height:1.4;";

    const nameSpan = document.createElement("span");
    nameSpan.textContent = sig.name;
    nameSpan.style.fontWeight = "600";
    sigLine.append(nameSpan);

    const openParen = document.createElement("span");
    openParen.textContent = "(";
    sigLine.append(openParen);

    const highlightIndex =
      sig.args.length === 0
        ? -1
        : argIndex >= sig.args.length - 1 && sig.args[sig.args.length - 1] === "..."
          ? sig.args.length - 1
          : Math.min(argIndex, sig.args.length - 1);

    sig.args.forEach((arg, i) => {
      if (i > 0) {
        const comma = document.createElement("span");
        comma.textContent = ", ";
        sigLine.append(comma);
      }
      const argSpan = document.createElement("span");
      argSpan.textContent = arg;
      if (i === highlightIndex) {
        argSpan.style.cssText =
          "font-weight:700;background:#fef9c3;padding:0 2px;border-radius:2px;";
      }
      sigLine.append(argSpan);
    });

    const closeParen = document.createElement("span");
    closeParen.textContent = ")";
    sigLine.append(closeParen);
    tip.append(sigLine);

    const summaryLabel = document.createElement("div");
    summaryLabel.textContent = "Summary";
    summaryLabel.style.cssText =
      "font:600 11px -apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;color:#6b7280;margin:0 0 2px 0;";
    tip.append(summaryLabel);

    const summaryText = document.createElement("div");
    summaryText.textContent = sig.summary;
    summaryText.style.cssText =
      "font:12px -apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;color:#374151;line-height:1.35;";
    tip.append(summaryText);

    const rect = input.getBoundingClientRect();
    tip.style.left = `${rect.left}px`;
    tip.style.top = `${rect.bottom + 2}px`;
    tip.style.display = "block";
  }

  private hideSignatureTip(): void {
    this.signatureFor = null;
    this.signatureTip.style.display = "none";
    this.signatureTip.replaceChildren();
  }

  private scheduleSignatureTipClose(): void {
    if (this.signatureBlurTimer !== null) clearTimeout(this.signatureBlurTimer);
    this.signatureBlurTimer = setTimeout(() => this.hideSignatureTip(), 120);
  }

  private onFormulaBoxKeyDown(ev: KeyboardEvent): void {
    if (this.autocomplete.handleKey(ev)) return;
    if (this.handlePointKeyboardKey(ev)) return;
    this.resetPointSpanOnType(ev);
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

  private activeEditor(): HTMLInputElement | null {
    if (this.editCell) return this.editInput;
    if (
      this.editable &&
      document.activeElement === this.formulaBox &&
      this.formulaBox.value.startsWith("=")
    ) {
      return this.formulaBox;
    }
    return null;
  }

  private armPointMode(input: HTMLInputElement): void {
    const caret = input.selectionStart;
    this.pointModeArmed = caret !== null && caretAcceptsReference(input.value, caret);
  }

  private isPointModeActive(): boolean {
    const input = this.activeEditor();
    if (!input) return false;
    const caret = input.selectionStart;
    if (caret === null) return false;
    if (this.activeRefSpan && caret === this.activeRefSpan.end) return true;
    return caretAcceptsReference(input.value, caret);
  }

  private applyPointModeRef(ref: string, _opts: { extend: boolean }): void {
    const input = this.activeEditor();
    if (!input) return;
    const caret = input.selectionStart ?? input.value.length;
    const res = applyReferenceAtCaret(input.value, caret, ref, this.activeRefSpan);
    input.value = res.text;
    this.activeRefSpan = res.span;
    this.pointModeArmed = true;
    this.pointHighlight = parsePointHighlight(ref, HIGHLIGHT_PALETTE[0]!);
    input.focus({ preventScroll: true });
    input.setSelectionRange(res.caret, res.caret);
    this.autocomplete.close();
    this.updateSignatureTip(input);
    this.growEditInput();
    this.scheduleDraw();
  }

  private resetPointSpanOnType(ev: KeyboardEvent): void {
    if (ev.key.length === 1 || ev.key === "Backspace" || ev.key === "Delete") {
      this.activeRefSpan = null;
      this.pointHighlight = null;
      this.pointKeyAnchor = null;
      this.pointKeyCursor = null;
    }
  }

  private movePointKeyboard(dr: number, dc: number, extend: boolean): void {
    const state = this.currentState();
    const grid = buildGrid(this.getActiveSheet(), state.colOverrides, state.rowOverrides);
    const base = this.pointKeyCursor ?? this.editCell ?? state.activeCell ?? { r: 1, c: 1 };
    const cursor = {
      r: clamp(base.r + dr, 1, grid.maxRow),
      c: clamp(base.c + dc, 1, grid.maxCol),
    };
    const anchor = extend && this.pointKeyAnchor ? this.pointKeyAnchor : cursor;
    this.pointKeyCursor = cursor;
    this.pointKeyAnchor = anchor;
    const minR = Math.min(anchor.r, cursor.r);
    const maxR = Math.max(anchor.r, cursor.r);
    const minC = Math.min(anchor.c, cursor.c);
    const maxC = Math.max(anchor.c, cursor.c);
    const ref =
      minR === maxR && minC === maxC
        ? `${colLabel(minC)}${minR}`
        : `${colLabel(minC)}${minR}:${colLabel(maxC)}${maxR}`;
    this.applyPointModeRef(ref, { extend });
    this.scrollToCell(cursor.r, cursor.c);
  }

  private openValidationEdit(info: ValidationPickEvent): void {
    if (!this.editable) return;
    this.selectCell(info.r, info.c);
    setTimeout(() => {
      this.openEditOverlay({ r: info.r, c: info.c }, null);
      this.validationTyped = false;
      this.updateValidationMenu();
    }, 0);
  }

  private validationListFor(cell: { r: number; c: number }): string[] | null {
    const sheet = this.getActiveSheet();
    const dropdowns = sheet.validationDropdowns ?? [];
    const d = dropdowns.find((dd) => dd.r === cell.r && dd.c === cell.c);
    if (!d) return null;
    const lists = sheet.validationLists ?? [];
    return lists[d.list] ?? null;
  }

  private updateValidationMenu(): void {
    if (!this.validationOptions || !this.editCell) return this.closeValidationMenu();
    const q = this.validationTyped ? this.editInput.value.trim().toLowerCase() : "";
    const all = this.validationOptions;
    this.validationFiltered = q ? all.filter((o) => o.toLowerCase().includes(q)) : all.slice();
    if (this.validationFiltered.length === 0) return this.closeValidationMenu();
    if (this.validationActive >= this.validationFiltered.length) this.validationActive = 0;
    this.renderValidationMenu();
  }

  private renderValidationMenu(): void {
    const menu = this.validationMenu;
    menu.replaceChildren();
    this.validationFiltered.forEach((value, i) => {
      const item = document.createElement("div");
      item.textContent = value;
      const active = i === this.validationActive;
      item.style.cssText = `padding:4px 8px;cursor:pointer;border-radius:4px;white-space:nowrap;${
        active ? "background:#2563eb;color:#fff;" : "color:#111827;"
      }`;
      item.onmousedown = (ev) => {
        ev.preventDefault();
        this.acceptValidation(i);
      };
      item.onmouseenter = () => {
        this.validationActive = i;
        this.renderValidationMenu();
      };
      menu.append(item);
    });
    const rect = this.editInput.getBoundingClientRect();
    menu.style.left = `${rect.left}px`;
    menu.style.top = `${rect.bottom + 2}px`;
    menu.style.minWidth = `${Math.max(120, rect.width)}px`;
    menu.style.display = "block";
  }

  private closeValidationMenu(): void {
    this.validationActive = 0;
    this.validationFiltered = [];
    this.validationMenu.style.display = "none";
  }

  private isValidationMenuOpen(): boolean {
    return this.validationMenu.style.display !== "none";
  }

  private acceptValidation(index: number): void {
    const value = this.validationFiltered[index];
    if (value === undefined) return;
    this.editInput.value = value;
    this.commitEdit("down");
    this.canvas.focus({ preventScroll: true });
  }

  private handleValidationMenuKey(ev: KeyboardEvent): boolean {
    if (!this.isValidationMenuOpen()) return false;
    const n = this.validationFiltered.length;
    if (n === 0) return false;
    if (ev.key === "ArrowDown") {
      ev.preventDefault();
      this.validationActive = (this.validationActive + 1) % n;
      this.renderValidationMenu();
      return true;
    }
    if (ev.key === "ArrowUp") {
      ev.preventDefault();
      this.validationActive = (this.validationActive - 1 + n) % n;
      this.renderValidationMenu();
      return true;
    }
    if (ev.key === "Enter" || ev.key === "Tab") {
      ev.preventDefault();
      this.acceptValidation(this.validationActive);
      return true;
    }
    if (ev.key === "Escape") {
      ev.preventDefault();
      this.closeValidationMenu();
      return true;
    }
    return false;
  }

  private openEditOverlay(cell: { r: number; c: number }, initialText: string | null): void {
    if (!this.editable) return;
    const sheet = this.getActiveSheet();
    const state = this.currentState();
    this.scrollToCell(cell.r, cell.c);
    const grid = buildGrid(sheet, state.colOverrides, state.rowOverrides);
    const { topLeftOf } = buildMergeMaps(sheet);
    const rect = rectFor(sheet, grid, cell.r, cell.c, topLeftOf);
    const z = this.zoom;
    this.editCell = { r: cell.r, c: cell.c };
    this.editEnterMode = initialText !== null;
    this.activeRefSpan = null;
    this.pointKeyAnchor = null;
    this.pointKeyCursor = null;
    this.editBaseLeft = rect.x * z;
    this.editBaseWidth = Math.max(rect.w * z, 24);
    this.editInput.style.left = `${this.editBaseLeft}px`;
    this.editInput.style.top = `${rect.y * z}px`;
    this.editInput.style.width = `${this.editBaseWidth}px`;
    this.editInput.style.height = `${Math.max(rect.h * z, 16)}px`;
    this.editInput.style.whiteSpace = "nowrap";
    this.editInput.style.display = "block";
    this.editInput.value = initialText ?? formatFormulaBar(sheet, cell);
    this.editInput.focus({ preventScroll: true });
    const end = this.editInput.value.length;
    this.editInput.setSelectionRange(end, end);
    this.pointModeArmed = this.editEnterMode && caretAcceptsReference(this.editInput.value, end);
    this.growEditInput();
    this.validationOptions = this.validationListFor(cell);
    this.validationTyped = this.editEnterMode;
    this.validationActive = 0;
    if (this.validationOptions) this.updateValidationMenu();
    else this.closeValidationMenu();
  }

  private growEditInput(): void {
    if (!this.editCell) return;
    this.editInput.style.width = `${this.editBaseWidth}px`;
    const fit = this.editInput.scrollWidth + 2;
    const maxWidth = this.stage.scrollLeft + this.stage.clientWidth - this.editBaseLeft;
    const width = Math.min(Math.max(this.editBaseWidth, fit), Math.max(this.editBaseWidth, maxWidth));
    this.editInput.style.width = `${width}px`;
  }

  private hideEditOverlay(): void {
    this.autocomplete.close();
    this.closeValidationMenu();
    this.validationOptions = null;
    this.hideSignatureTip();
    this.activeRefSpan = null;
    this.pointHighlight = null;
    this.pointKeyAnchor = null;
    this.pointKeyCursor = null;
    this.pointModeArmed = false;
    if (!this.editCell) return;
    this.editCell = null;
    this.editInput.style.display = "none";
    this.editInput.style.width = "";
    this.editInput.value = "";
  }

  private commitEdit(commitMove: "down" | "right" | "up" | "left" | null): void {
    const cell = this.editCell;
    if (!cell) return;
    const input = balanceFormula(this.editInput.value);
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

  private handlePointKeyboardKey(ev: KeyboardEvent): boolean {
    if (
      ev.key !== "ArrowUp" &&
      ev.key !== "ArrowDown" &&
      ev.key !== "ArrowLeft" &&
      ev.key !== "ArrowRight"
    ) {
      return false;
    }
    if (!this.pointModeArmed || !this.isPointModeActive()) {
      this.pointModeArmed = false;
      return false;
    }
    ev.preventDefault();
    const dr = ev.key === "ArrowUp" ? -1 : ev.key === "ArrowDown" ? 1 : 0;
    const dc = ev.key === "ArrowLeft" ? -1 : ev.key === "ArrowRight" ? 1 : 0;
    this.movePointKeyboard(dr, dc, ev.shiftKey);
    return true;
  }

  private onEditInputKeyDown(ev: KeyboardEvent): void {
    if (this.autocomplete.handleKey(ev)) return;
    if (this.handleValidationMenuKey(ev)) return;
    if (this.handlePointKeyboardKey(ev)) return;
    this.resetPointSpanOnType(ev);
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
      this.commitEdit(null);
      this.canvas.focus({ preventScroll: true });
    } else if (
      ev.key === "ArrowUp" ||
      ev.key === "ArrowDown" ||
      ev.key === "ArrowLeft" ||
      ev.key === "ArrowRight"
    ) {
      if (!this.editEnterMode) return;
      if (this.editInput.value.startsWith("=")) return;
      if (this.isPointModeActive()) return;
      ev.preventDefault();
      const dir =
        ev.key === "ArrowUp"
          ? "up"
          : ev.key === "ArrowDown"
            ? "down"
            : ev.key === "ArrowLeft"
              ? "left"
              : "right";
      this.commitEdit(dir);
      this.canvas.focus({ preventScroll: true });
    }
  }
}


