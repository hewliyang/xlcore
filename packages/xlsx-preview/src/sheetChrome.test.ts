import { describe, expect, it } from "vitest";
import {
  FILTER_ARROW_BOX_H,
  FILTER_ARROW_BOX_W,
  FILTER_ARROW_INSET_X,
  VALIDATION_ARROW_BOX,
  filterArrowRect,
  pivotFilterArrows,
  tableFilterArrows,
  validationArrowRect,
} from "./sheetChrome.js";
import type { Sheet } from "./types.js";

describe("filterArrowRect", () => {
  it("anchors the box to the right edge of the cell", () => {
    const box = filterArrowRect({ x: 100, y: 40, w: 80, h: 20 });
    expect(box.w).toBe(FILTER_ARROW_BOX_W);
    expect(box.h).toBe(FILTER_ARROW_BOX_H);
    expect(box.x).toBe(100 + 80 - FILTER_ARROW_BOX_W - FILTER_ARROW_INSET_X);
    expect(box.y).toBe(40 + (20 - FILTER_ARROW_BOX_H) / 2);
  });
});

describe("validationArrowRect", () => {
  it("places the button just outside the cell's right edge", () => {
    const box = validationArrowRect({ x: 100, y: 40, w: 80, h: 20 });
    expect(box.x).toBe(180);
    expect(box.w).toBe(VALIDATION_ARROW_BOX);
    expect(box.h).toBe(VALIDATION_ARROW_BOX);
    expect(box.y).toBe(40 + (20 - VALIDATION_ARROW_BOX) / 2);
  });

  it("caps the button height to short rows", () => {
    const box = validationArrowRect({ x: 0, y: 0, w: 50, h: 10 });
    expect(box.h).toBe(10);
    expect(box.y).toBe(0);
  });
});

describe("pivotFilterArrows", () => {
  it("flattens arrows with their pivot name", () => {
    const sheet = {
      pivots: [
        {
          name: "P1",
          range: { r1: 1, c1: 1, r2: 8, c2: 5 },
          filterArrowCells: [
            { r: 3, c: 2, field: "Region", axis: "row" },
            { r: 2, c: 4, field: "Product", axis: "column" },
          ],
        },
      ],
    } as unknown as Sheet;
    const arrows = pivotFilterArrows(sheet);
    expect(arrows).toEqual([
      { r: 3, c: 2, field: "Region", axis: "row", pivot: "P1" },
      { r: 2, c: 4, field: "Product", axis: "column", pivot: "P1" },
    ]);
  });

  it("returns empty for sheets without pivots", () => {
    expect(pivotFilterArrows({} as Sheet)).toEqual([]);
  });
});

describe("tableFilterArrows", () => {
  it("returns the sheet table filter arrow payloads", () => {
    const arrows = [
      { r: 1, c: 1, columnOffset: 0, columnName: "Region", rangeRef: "A1:C9" },
      { r: 1, c: 3, columnOffset: 2, columnName: "Amount", rangeRef: "A1:C9" },
    ];
    const sheet = { tableFilterArrows: arrows } as unknown as Sheet;
    expect(tableFilterArrows(sheet)).toEqual(arrows);
  });

  it("returns empty for sheets without table filter arrows", () => {
    expect(tableFilterArrows({} as Sheet)).toEqual([]);
  });
});
