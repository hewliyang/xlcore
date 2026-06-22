import { clamp } from "./mathUtils.js";
import { balanceFormula, formatFormulaBar } from "./formulaText.js";
import { buildGrid } from "./render.js";
import { colLabel } from "./grid.js";
import { buildMergeMaps, rectFor } from "./geometry.js";
import { frozenDims } from "./panes.js";
import { parsePointHighlight } from "./previewerRefs.js";
import {
  applyReferenceAtCaret,
  caretAcceptsReference,
  type RefSpan,
} from "./formulaPointMode.js";
import type { HighlightRange } from "./renderTypes.js";
import type { AutocompletePopoverHandle } from "./autocompletePopover.js";
import type { SignatureTipHandle } from "./signatureTip.js";
import type { ValidationDropdownHandle } from "./validationDropdown.js";
import type { Sheet } from "./types.js";

const POINT_HIGHLIGHT_COLOR = "#2563eb";

export interface CellEditDetail {
  sheetIndex: number;
  r: number;
  c: number;
  input: string;
  commitMove: "down" | "right" | "up" | "left" | null;
}

export interface CellEditorHost {
  readonly editable: boolean;
  getSheet(): Sheet;
  getColOverrides(): Map<number, number>;
  getRowOverrides(): Map<number, number>;
  getActiveCellState(): { r: number; c: number } | null;
  getZoom(): number;
  getActiveSheetIndex(): number;
  getFormulaBox(): HTMLInputElement;
  getStageScrollLeft(): number;
  getStageScrollTop(): number;
  getStageClientWidth(): number;
  scrollToCell(r: number, c: number): void;
  scheduleDraw(): void;
  focusCanvas(): void;
  emitCellEdit(detail: CellEditDetail): void;
}

export interface CellEditorDeps {
  host: CellEditorHost;
  autocomplete: AutocompletePopoverHandle;
  signature: SignatureTipHandle;
  validation: ValidationDropdownHandle;
}

export class CellEditor {
  private readonly host: CellEditorHost;
  private readonly autocomplete: AutocompletePopoverHandle;
  private readonly signature: SignatureTipHandle;
  private readonly validation: ValidationDropdownHandle;
  private readonly editInput: HTMLInputElement;

  private editCell: { r: number; c: number } | null = null;
  private editEnterMode = false;
  private pointModeArmed = false;
  private editBaseLeft = 0;
  private editBaseWidth = 0;
  private pointKeyAnchor: { r: number; c: number } | null = null;
  private pointKeyCursor: { r: number; c: number } | null = null;
  private activeRefSpan: RefSpan | null = null;
  private pointHighlight: HighlightRange | null = null;

  constructor(deps: CellEditorDeps) {
    this.host = deps.host;
    this.autocomplete = deps.autocomplete;
    this.signature = deps.signature;
    this.validation = deps.validation;

    this.editInput = document.createElement("input");
    this.editInput.style.cssText =
      "position:absolute;top:0;left:0;display:none;z-index:5;box-sizing:border-box;margin:0;padding:0 3px;border:2px solid #2563eb;outline:none;background:#fff;color:#111827;font:13px -apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;";
    this.editInput.addEventListener("keydown", (ev) => this.onEditInputKeyDown(ev));
    this.editInput.addEventListener("input", () => {
      this.armPointMode(this.editInput);
      this.autocomplete.update(this.editInput);
      this.signature.update(this.editInput);
      if (this.validation.hasOptions()) this.validation.refresh(true);
      this.growEditInput();
      this.host.scheduleDraw();
    });
    this.editInput.addEventListener("keyup", () => this.signature.update(this.editInput));
    this.editInput.addEventListener("mousedown", () => {
      this.pointModeArmed = false;
    });
    this.editInput.addEventListener("click", () => this.signature.update(this.editInput));
    this.editInput.addEventListener("blur", () => {
      this.autocomplete.scheduleClose();
      this.signature.scheduleClose();
      this.commitEdit(null);
    });
  }

  getEditInput(): HTMLInputElement {
    return this.editInput;
  }

  getEditCell(): { r: number; c: number } | null {
    return this.editCell;
  }

  getEditText(): string {
    return this.editInput.value;
  }

  setEditText(value: string): void {
    this.editInput.value = value;
  }

  getPointHighlight(): HighlightRange | null {
    return this.pointHighlight;
  }

  disarmPointMode(): void {
    this.pointModeArmed = false;
  }

  private activeEditor(): HTMLInputElement | null {
    if (this.editCell) return this.editInput;
    const formulaBox = this.host.getFormulaBox();
    if (
      this.host.editable &&
      document.activeElement === formulaBox &&
      formulaBox.value.startsWith("=")
    ) {
      return formulaBox;
    }
    return null;
  }

  armPointMode(input: HTMLInputElement): void {
    const caret = input.selectionStart;
    this.pointModeArmed = caret !== null && caretAcceptsReference(input.value, caret);
  }

  isPointModeActive(): boolean {
    const input = this.activeEditor();
    if (!input) return false;
    const caret = input.selectionStart;
    if (caret === null) return false;
    if (this.activeRefSpan && caret === this.activeRefSpan.end) return true;
    return caretAcceptsReference(input.value, caret);
  }

  applyPointModeRef(ref: string, _opts: { extend: boolean }): void {
    const input = this.activeEditor();
    if (!input) return;
    const caret = input.selectionStart ?? input.value.length;
    const res = applyReferenceAtCaret(input.value, caret, ref, this.activeRefSpan);
    input.value = res.text;
    this.activeRefSpan = res.span;
    this.pointModeArmed = true;
    this.pointHighlight = parsePointHighlight(ref, POINT_HIGHLIGHT_COLOR);
    input.focus({ preventScroll: true });
    input.setSelectionRange(res.caret, res.caret);
    this.autocomplete.close();
    this.signature.update(input);
    this.growEditInput();
    this.host.scheduleDraw();
  }

  resetPointSpanOnType(ev: KeyboardEvent): void {
    if (ev.key.length === 1 || ev.key === "Backspace" || ev.key === "Delete") {
      this.activeRefSpan = null;
      this.pointHighlight = null;
      this.pointKeyAnchor = null;
      this.pointKeyCursor = null;
    }
  }

  private movePointKeyboard(dr: number, dc: number, extend: boolean): void {
    const grid = buildGrid(
      this.host.getSheet(),
      this.host.getColOverrides(),
      this.host.getRowOverrides(),
    );
    const base =
      this.pointKeyCursor ?? this.editCell ?? this.host.getActiveCellState() ?? { r: 1, c: 1 };
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
    this.host.scrollToCell(cursor.r, cursor.c);
  }

  openEditOverlay(cell: { r: number; c: number }, initialText: string | null): void {
    if (!this.host.editable) return;
    const sheet = this.host.getSheet();
    const colOverrides = this.host.getColOverrides();
    const rowOverrides = this.host.getRowOverrides();
    this.host.scrollToCell(cell.r, cell.c);
    const grid = buildGrid(sheet, colOverrides, rowOverrides);
    const { topLeftOf } = buildMergeMaps(sheet);
    const rect = rectFor(sheet, grid, cell.r, cell.c, topLeftOf);
    const z = this.host.getZoom();
    this.editCell = { r: cell.r, c: cell.c };
    this.editEnterMode = initialText !== null;
    this.activeRefSpan = null;
    this.pointKeyAnchor = null;
    this.pointKeyCursor = null;
    const { splitX, splitY } = frozenDims(sheet, grid);
    const offX = cell.c < splitX ? this.host.getStageScrollLeft() : 0;
    const offY = cell.r < splitY ? this.host.getStageScrollTop() : 0;
    this.editBaseLeft = rect.x * z + offX;
    this.editBaseWidth = Math.max(rect.w * z, 24);
    this.editInput.style.left = `${this.editBaseLeft}px`;
    this.editInput.style.top = `${rect.y * z + offY}px`;
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
    this.validation.open(cell, { typed: this.editEnterMode });
  }

  private growEditInput(): void {
    if (!this.editCell) return;
    this.editInput.style.width = `${this.editBaseWidth}px`;
    const fit = this.editInput.scrollWidth + 2;
    const maxWidth =
      this.host.getStageScrollLeft() + this.host.getStageClientWidth() - this.editBaseLeft;
    const width = Math.min(
      Math.max(this.editBaseWidth, fit),
      Math.max(this.editBaseWidth, maxWidth),
    );
    this.editInput.style.width = `${width}px`;
  }

  hideEditOverlay(): void {
    this.autocomplete.close();
    this.validation.reset();
    this.signature.hide();
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

  commitEdit(commitMove: "down" | "right" | "up" | "left" | null): void {
    const cell = this.editCell;
    if (!cell) return;
    const input = balanceFormula(this.editInput.value);
    this.hideEditOverlay();
    this.host.emitCellEdit({
      sheetIndex: this.host.getActiveSheetIndex(),
      r: cell.r,
      c: cell.c,
      input,
      commitMove,
    });
  }

  handlePointKeyboardKey(ev: KeyboardEvent): boolean {
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
    if (this.validation.handleKey(ev)) return;
    if (this.handlePointKeyboardKey(ev)) return;
    this.resetPointSpanOnType(ev);
    if (ev.key === "Enter") {
      ev.preventDefault();
      this.commitEdit(ev.shiftKey ? "up" : "down");
      this.host.focusCanvas();
    } else if (ev.key === "Tab") {
      ev.preventDefault();
      this.commitEdit(ev.shiftKey ? "left" : "right");
      this.host.focusCanvas();
    } else if (ev.key === "Escape") {
      ev.preventDefault();
      this.commitEdit(null);
      this.host.focusCanvas();
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
      this.host.focusCanvas();
    }
  }
}
