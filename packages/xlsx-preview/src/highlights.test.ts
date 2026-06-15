import { expect, test } from "vitest";

import { buildHighlightRects } from "./highlights.js";
import type { Grid } from "./grid.js";

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
