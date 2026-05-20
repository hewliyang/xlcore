import { expect, test } from "vitest";

import { drawingHyperlinkAt } from "./drawingHits.js";
import { buildGrid } from "./grid.js";
import type { Drawing, Sheet } from "./types.js";

test("drawingHyperlinkAt returns shape hyperlink when point is inside bbox", () => {
  const emu = 9525;
  const drawing: Drawing = {
    kind: "shape",
    anchor: {
      anchorKind: "twoCell",
      fromCol: 0,
      fromColOffEmu: emu,
      fromRow: 0,
      fromRowOffEmu: emu,
      toCol: 0,
      toColOffEmu: 0,
      toRow: 0,
      toRowOffEmu: 0,
      extEmuCx: 10 * emu,
      extEmuCy: 8 * emu,
    },
    hyperlink: { target: "https://example.com/test", tooltip: "Go" },
    shape: { nodes: [{ relX: 0, relY: 0, relW: 1, relH: 1, paragraphs: [] }] },
  };
  const sheet = {
    index: 0,
    name: "S",
    maxRow: 10,
    maxCol: 10,
    defaultColWidthPx: 64,
    defaultRowHeightPx: 20,
    cols: [],
    merges: [],
    freeze: null,
    showGridLines: true,
    cells: [],
    drawings: [drawing],
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
  const grid = buildGrid(sheet);
  const insideX = grid.originX + 5;
  const insideY = grid.originY + 4;
  const link = drawingHyperlinkAt(sheet, grid, insideX, insideY);
  expect(link?.target).toBe("https://example.com/test");
  expect(link?.tooltip).toBe("Go");
  expect(drawingHyperlinkAt(sheet, grid, 0, 0)).toBeUndefined();
});
