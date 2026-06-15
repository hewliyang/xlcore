import { describe, expect, it } from "vitest";
import { absoluteAnchor, anchorA1 } from "./api-refs.js";

describe("anchorA1", () => {
  it("converts 1-based A1 range to 0-based half-open anchor", () => {
    expect(anchorA1("A1:E15")).toEqual({ fromColumn: 0, fromRow: 0, toColumn: 5, toRow: 15 });
  });
  it("handles a single-cell range", () => {
    expect(anchorA1("B2:B2")).toEqual({ fromColumn: 1, fromRow: 1, toColumn: 2, toRow: 2 });
  });
  it("strips a sheet prefix", () => {
    expect(anchorA1("Sheet1!C3:F20")).toEqual({
      fromColumn: 2,
      fromRow: 2,
      toColumn: 6,
      toRow: 20,
    });
  });
  it("throws on a non-range", () => {
    expect(() => anchorA1("A1")).toThrow();
  });
});

describe("absoluteAnchor", () => {
  const EMU = 9525n;
  it("converts a px rect on the default 64\u00d720 grid", () => {
    expect(absoluteAnchor(0, 0, 320, 100)).toEqual({
      fromColumn: 0,
      fromRow: 0,
      toColumn: 5,
      toRow: 5,
      fromColumnOffsetEmu: 0n,
      fromRowOffsetEmu: 0n,
      toColumnOffsetEmu: 0n,
      toRowOffsetEmu: 0n,
    });
  });
  it("emits in-cell EMU offsets for non-aligned rects", () => {
    expect(absoluteAnchor(70, 25, 100, 30)).toEqual({
      fromColumn: 1,
      fromRow: 1,
      toColumn: 2,
      toRow: 2,
      fromColumnOffsetEmu: 6n * EMU,
      fromRowOffsetEmu: 5n * EMU,
      toColumnOffsetEmu: 42n * EMU,
      toRowOffsetEmu: 15n * EMU,
    });
  });
  it("offsets never exceed the referenced cell", () => {
    for (const [x, y, w, h] of [
      [0, 0, 64, 20],
      [63.9, 19.9, 0.2, 0.2],
      [1, 1, 1000, 500],
      [127, 39, 65, 21],
    ] as const) {
      const a = absoluteAnchor(x, y, w, h);
      expect(a.fromColumnOffsetEmu! < 64n * EMU).toBe(true);
      expect(a.fromRowOffsetEmu! < 20n * EMU).toBe(true);
      expect(a.toColumnOffsetEmu! < 64n * EMU).toBe(true);
      expect(a.toRowOffsetEmu! < 20n * EMU).toBe(true);
    }
  });
  it("honors colWidthPx / rowHeightPx overrides", () => {
    expect(absoluteAnchor(100, 60, 200, 60, { colWidthPx: 50, rowHeightPx: 30 })).toEqual({
      fromColumn: 2,
      fromRow: 2,
      toColumn: 6,
      toRow: 4,
      fromColumnOffsetEmu: 0n,
      fromRowOffsetEmu: 0n,
      toColumnOffsetEmu: 0n,
      toRowOffsetEmu: 0n,
    });
  });
  it("round-trips px through EMU within 1 EMU", () => {
    const a = absoluteAnchor(123.4, 56.7, 89.1, 23.4);
    const px = (cell: number, offEmu: bigint, size: number) => cell * size + Number(offEmu) / 9525;
    expect(px(a.fromColumn, a.fromColumnOffsetEmu!, 64)).toBeCloseTo(123.4, 3);
    expect(px(a.fromRow, a.fromRowOffsetEmu!, 20)).toBeCloseTo(56.7, 3);
    expect(px(a.toColumn, a.toColumnOffsetEmu!, 64)).toBeCloseTo(123.4 + 89.1, 3);
    expect(px(a.toRow, a.toRowOffsetEmu!, 20)).toBeCloseTo(56.7 + 23.4, 3);
  });
  it("rejects invalid input", () => {
    expect(() => absoluteAnchor(-1, 0, 10, 10)).toThrow(/x must be/);
    expect(() => absoluteAnchor(0, -1, 10, 10)).toThrow(/y must be/);
    expect(() => absoluteAnchor(0, 0, 0, 10)).toThrow(/w must be/);
    expect(() => absoluteAnchor(0, 0, 10, Number.NaN)).toThrow(/h must be/);
    expect(() => absoluteAnchor(0, 0, 10, 10, { colWidthPx: 0 })).toThrow(/colWidthPx/);
    expect(() => absoluteAnchor(0, 0, 10, 10, { rowHeightPx: -5 })).toThrow(/rowHeightPx/);
  });
});
