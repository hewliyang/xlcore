import { expect, test } from "vitest";

import { anchorToRect, buildGrid, rectToAnchor } from "./grid.js";
import type { DrawingAnchor, Drawing, Sheet } from "./types.js";

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

const twoCellTemplate: DrawingAnchor = {
  anchorKind: "twoCell",
  fromCol: 0,
  fromColOffEmu: 0,
  fromRow: 0,
  fromRowOffEmu: 0,
  toCol: 1,
  toColOffEmu: 0,
  toRow: 1,
  toRowOffEmu: 0,
};

const absoluteTemplate: DrawingAnchor = {
  anchorKind: "absolute",
  fromCol: 0,
  fromColOffEmu: 0,
  fromRow: 0,
  fromRowOffEmu: 0,
  toCol: 0,
  toColOffEmu: 0,
  toRow: 0,
  toRowOffEmu: 0,
  extEmuCx: 9525,
  extEmuCy: 9525,
};

function rectFor(anchor: DrawingAnchor, grid: ReturnType<typeof buildGrid>) {
  const d: Drawing = {
    kind: "image",
    anchor,
    image: { src: "x" },
  } as unknown as Drawing;
  return anchorToRect(d, grid);
}

const grid = buildGrid(baseSheet);
const rects = [
  { x: grid.originX + 10, y: grid.originY + 5, w: 120, h: 60 },
  { x: grid.originX + 200, y: grid.originY + 150, w: 64, h: 40 },
  { x: grid.originX + 33, y: grid.originY + 77, w: 250, h: 130 },
  { x: grid.originX, y: grid.originY, w: 50, h: 20 },
];

for (const style of ["twoCell", "absolute"] as const) {
  const template = style === "twoCell" ? twoCellTemplate : absoluteTemplate;
  for (const r of rects) {
    test(`rectToAnchor round-trips ${style} ${r.x},${r.y},${r.w},${r.h}`, () => {
      const anchor = rectToAnchor(r, grid, template);
      const back = rectFor(anchor, grid);
      expect(back).not.toBeNull();
      expect(back!.x).toBeCloseTo(r.x, 0);
      expect(back!.y).toBeCloseTo(r.y, 0);
      expect(back!.w).toBeCloseTo(r.w, 0);
      expect(back!.h).toBeCloseTo(r.h, 0);
      if (style === "absolute") {
        expect(anchor.extEmuCx).toBeGreaterThan(0);
      } else {
        expect(anchor.extEmuCx).toBeUndefined();
      }
    });
  }
}

test("rectToAnchor clamps off-grid origin", () => {
  const anchor = rectToAnchor({ x: -50, y: -30, w: 100, h: 40 }, grid, twoCellTemplate);
  expect(anchor.fromCol).toBe(0);
  expect(anchor.fromRow).toBe(0);
  expect(anchor.fromColOffEmu).toBe(0);
  expect(anchor.fromRowOffEmu).toBe(0);
  const back = rectFor(anchor, grid);
  expect(back!.x).toBeCloseTo(grid.originX, 0);
  expect(back!.y).toBeCloseTo(grid.originY, 0);
});
