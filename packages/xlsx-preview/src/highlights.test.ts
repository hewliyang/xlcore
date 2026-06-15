import { expect, test } from "vitest";

import { buildHighlightRects, referencesToHighlights } from "./highlights.js";
import type { Grid } from "./grid.js";
import type { DependencyReference } from "./api-schema/DependencyReference.js";

function ref(
  sheet: string,
  startRow: number,
  startColumn: number,
  endRow: number,
  endColumn: number,
): DependencyReference {
  return {
    sheet,
    reference: "",
    startRow,
    startColumn,
    endRow,
    endColumn,
    rows: endRow - startRow + 1,
    columns: endColumn - startColumn + 1,
  };
}

const PALETTE = ["#aaa000", "#bbb111"];

function makeGrid(): Grid {
  const colX = [0, 0, 50, 100, 150, 200];
  const rowY = [0, 0, 20, 40, 60, 80];
  return {
    colX,
    colW: [0, 50, 50, 50, 50],
    rowY,
    rowH: [0, 20, 20, 20, 20],
    totalW: 200,
    totalH: 80,
    maxCol: 4,
    maxRow: 4,
    rowGutterW: 0,
    colGutterH: 0,
    originX: 0,
    originY: 0,
    rowOutlineDepth: 0,
    colOutlineDepth: 0,
  };
}

test("single-cell highlight produces correct rect", () => {
  const rects = buildHighlightRects(makeGrid(), [{ r1: 2, c1: 2, r2: 2, c2: 2, color: "#ff0000" }]);
  expect(rects).toEqual([{ x: 50, y: 20, w: 50, h: 20, color: "#ff0000" }]);
});

test("multi-cell range produces correct rect", () => {
  const rects = buildHighlightRects(makeGrid(), [{ r1: 1, c1: 1, r2: 3, c2: 3, color: "#00ff00" }]);
  expect(rects).toEqual([{ x: 0, y: 0, w: 150, h: 60, color: "#00ff00" }]);
});

test("out-of-range clamps to grid extents", () => {
  const rects = buildHighlightRects(makeGrid(), [
    { r1: 1, c1: 1, r2: 99, c2: 99, color: "#0000ff" },
  ]);
  expect(rects).toEqual([{ x: 0, y: 0, w: 200, h: 80, color: "#0000ff" }]);
});

test("normalizes reversed corners", () => {
  const rects = buildHighlightRects(makeGrid(), [{ r1: 3, c1: 3, r2: 1, c2: 1, color: "#abcdef" }]);
  expect(rects).toEqual([{ x: 0, y: 0, w: 150, h: 60, color: "#abcdef" }]);
});

test("color passthrough", () => {
  const rects = buildHighlightRects(makeGrid(), [{ r1: 2, c1: 2, r2: 2, c2: 2, color: "#123456" }]);
  expect(rects[0]?.color).toBe("#123456");
});

test("same-sheet refs map with cycled colors", () => {
  const out = referencesToHighlights(
    [ref("S1", 1, 1, 1, 1), ref("S1", 2, 2, 3, 4), ref("S1", 5, 5, 5, 5)],
    "S1",
    PALETTE,
  );
  expect(out).toEqual([
    { r1: 1, c1: 1, r2: 1, c2: 1, color: "#aaa000" },
    { r1: 2, c1: 2, r2: 3, c2: 4, color: "#bbb111" },
    { r1: 5, c1: 5, r2: 5, c2: 5, color: "#aaa000" },
  ]);
});

test("cross-sheet refs dropped, colors cycle over kept only", () => {
  const out = referencesToHighlights(
    [ref("Other", 1, 1, 1, 1), ref("S1", 2, 2, 2, 2), ref("S1", 3, 3, 3, 3)],
    "S1",
    PALETTE,
  );
  expect(out).toEqual([
    { r1: 2, c1: 2, r2: 2, c2: 2, color: "#aaa000" },
    { r1: 3, c1: 3, r2: 3, c2: 3, color: "#bbb111" },
  ]);
});

test("empty input yields empty output", () => {
  expect(referencesToHighlights([], "S1", PALETTE)).toEqual([]);
});

test("empty palette yields empty output", () => {
  expect(referencesToHighlights([ref("S1", 1, 1, 1, 1)], "S1", [])).toEqual([]);
});
