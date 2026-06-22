// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest";
import { CellEditor, type CellEditDetail, type CellEditorHost } from "./cellEditor.js";
import { createAutocompletePopover } from "./autocompletePopover.js";
import { createSignatureTip } from "./signatureTip.js";
import { createValidationDropdown } from "./validationDropdown.js";
import type { Sheet } from "./types.js";

function makeSheet(): Sheet {
  return {
    index: 0,
    name: "Sheet1",
    maxRow: 30,
    maxCol: 14,
    defaultColWidthPx: 50,
    defaultRowHeightPx: 20,
    cols: [],
    merges: [],
    freeze: null,
    showGridLines: true,
    cells: [],
    decodedCells: {
      count: 0,
      r: new Uint32Array(0),
      c: new Uint32Array(0),
      kind: new Uint8Array(0),
      valueIdx: new Int32Array(0),
      formulaIdx: new Int32Array(0),
      styleIdx: new Int32Array(0),
      runsIdx: new Int32Array(0),
      rowPtr: new Uint32Array(0),
    },
    decodedRowMeta: {
      count: 0,
      index: new Uint32Array(0),
      heightPx: new Float32Array(0),
      styleIdx: new Int32Array(0),
      hidden: new Uint8Array(0),
      outlineLevel: new Uint8Array(0),
      byIndex: new Map(),
    },
  } as unknown as Sheet;
}

function makeEditor(overrides: Partial<CellEditorHost> = {}) {
  const sheet = makeSheet();
  const emitted: CellEditDetail[] = [];
  const formulaBox = document.createElement("input");
  document.body.append(formulaBox);
  const host: CellEditorHost = {
    editable: true,
    getSheet: () => sheet,
    getColOverrides: () => new Map(),
    getRowOverrides: () => new Map(),
    getActiveCellState: () => ({ r: 1, c: 1 }),
    getZoom: () => 1,
    getActiveSheetIndex: () => 0,
    getFormulaBox: () => formulaBox,
    getStageScrollLeft: () => 0,
    getStageScrollTop: () => 0,
    getStageClientWidth: () => 800,
    scrollToCell: vi.fn(),
    scheduleDraw: vi.fn(),
    focusCanvas: vi.fn(),
    emitCellEdit: (detail) => emitted.push(detail),
    ...overrides,
  };
  const autocomplete = createAutocompletePopover({
    getFunctionNames: () => [],
    onAccept: () => {},
  });
  const signature = createSignatureTip({ isBlocked: () => false });
  const validation = createValidationDropdown({
    getEditInput: () => editor.getEditInput(),
    getSheet: () => sheet,
    onAccept: () => {},
  });
  const editor = new CellEditor({ host, autocomplete, signature, validation });
  document.body.append(editor.getEditInput());
  return { editor, host, emitted, formulaBox, autocomplete, signature, validation };
}

afterEach(() => {
  document.body.replaceChildren();
});

describe("CellEditor", () => {
  it("openEditOverlay shows the input and seeds initial text", () => {
    const { editor } = makeEditor();
    editor.openEditOverlay({ r: 2, c: 3 }, "=A1");
    const input = editor.getEditInput();
    expect(input.style.display).toBe("block");
    expect(input.value).toBe("=A1");
    expect(editor.getEditCell()).toEqual({ r: 2, c: 3 });
  });

  it("commitEdit emits balanced input and clears the overlay", () => {
    const { editor, emitted } = makeEditor();
    editor.openEditOverlay({ r: 4, c: 5 }, "=SUM(A1");
    editor.commitEdit("down");
    expect(emitted).toHaveLength(1);
    expect(emitted[0]).toEqual({
      sheetIndex: 0,
      r: 4,
      c: 5,
      input: "=SUM(A1)",
      commitMove: "down",
    });
    expect(editor.getEditCell()).toBeNull();
    expect(editor.getEditInput().style.display).toBe("none");
  });

  it("hideEditOverlay resets point-mode highlight", () => {
    const { editor } = makeEditor();
    editor.openEditOverlay({ r: 1, c: 1 }, "=");
    editor.applyPointModeRef("B2", { extend: false });
    expect(editor.getPointHighlight()).not.toBeNull();
    editor.hideEditOverlay();
    expect(editor.getPointHighlight()).toBeNull();
  });

  it("applyPointModeRef inserts the ref at the caret", () => {
    const { editor } = makeEditor();
    editor.openEditOverlay({ r: 1, c: 1 }, "=");
    editor.applyPointModeRef("C3", { extend: false });
    expect(editor.getEditInput().value).toBe("=C3");
  });

  it("movePointKeyboard builds an extended range ref via arrow keys", () => {
    const { editor } = makeEditor();
    editor.openEditOverlay({ r: 1, c: 1 }, "=");
    editor.applyPointModeRef("A1", { extend: false });
    const down = new KeyboardEvent("keydown", { key: "ArrowDown" });
    expect(editor.handlePointKeyboardKey(down)).toBe(true);
    expect(editor.getEditInput().value).toBe("=A2");
    const shiftDown = new KeyboardEvent("keydown", { key: "ArrowDown", shiftKey: true });
    expect(editor.handlePointKeyboardKey(shiftDown)).toBe(true);
    expect(editor.getEditInput().value).toBe("=A2:A3");
  });
});
