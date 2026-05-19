import { expect, test } from "vitest";

import { buildGrid } from "./grid.js";
import type { Sheet } from "./types.js";

test("viewport grid preserves merged-cell extents beyond the visible columns", () => {
  const sheet = {
    index: 0,
    name: "Sheet1",
    maxRow: 30,
    maxCol: 14,
    defaultColWidthPx: 50,
    defaultRowHeightPx: 20,
    cols: [],
    merges: [{ r1: 28, c1: 2, r2: 28, c2: 14 }],
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

  const grid = buildGrid(sheet, undefined, undefined, 500, 600);

  expect(grid.maxCol).toBeGreaterThanOrEqual(14);
  expect(grid.colX[15]).toBeGreaterThan(grid.colX[2]!);
});
