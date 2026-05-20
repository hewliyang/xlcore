import { expect, test } from "vitest";

import { anchorToRect, buildGrid } from "./grid.js";
import type { Drawing, Sheet } from "./types.js";

const baseSheet = {
  index: 0,
  name: "Sheet1",
  maxRow: 20,
  maxCol: 10,
  defaultColWidthPx: 64,
  defaultRowHeightPx: 20,
  cols: [],
  merges: [],
  freeze: null,
  showGridLines: true,
  cells: [],
  drawings: [],
  hyperlinks: [],
  comments: [],
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

test("absoluteAnchor resolves from sheet origin plus EMU offsets", () => {
  const emu = 9525;
  const drawing: Drawing = {
    kind: "shape",
    anchor: {
      anchorKind: "absolute",
      fromCol: 0,
      fromColOffEmu: 2 * emu,
      fromRow: 0,
      fromRowOffEmu: 3 * emu,
      toCol: 0,
      toColOffEmu: 0,
      toRow: 0,
      toRowOffEmu: 0,
      extEmuCx: 8 * emu,
      extEmuCy: 6 * emu,
    },
    shape: { nodes: [{ relX: 0, relY: 0, relW: 1, relH: 1, paragraphs: [] }] },
  };
  const sheet = { ...baseSheet, drawings: [drawing] };
  const grid = buildGrid(sheet);
  const rect = anchorToRect(drawing, grid);
  expect(rect).not.toBeNull();
  expect(rect!.x).toBeCloseTo(grid.originX + 2, 1);
  expect(rect!.y).toBeCloseTo(grid.originY + 3, 1);
  expect(rect!.w).toBeCloseTo(8, 1);
  expect(rect!.h).toBeCloseTo(6, 1);
});
