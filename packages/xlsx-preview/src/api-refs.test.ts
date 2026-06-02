import { describe, expect, it } from "vitest";
import { anchorA1, rangeDims, validateMatrixShape } from "./api-refs.js";

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

describe("rangeDims", () => {
  it("single cell", () => {
    expect(rangeDims("A1")).toEqual({ rows: 1, cols: 1 });
    expect(rangeDims("$AA$10")).toEqual({ rows: 1, cols: 1 });
  });
  it("rectangular range", () => {
    expect(rangeDims("A1:C3")).toEqual({ rows: 3, cols: 3 });
    expect(rangeDims("B2:D2")).toEqual({ rows: 1, cols: 3 });
    expect(rangeDims("A1:A5")).toEqual({ rows: 5, cols: 1 });
  });
  it("whole columns / rows", () => {
    expect(rangeDims("A:C")).toEqual({ rows: null, cols: 3 });
    expect(rangeDims("2:4")).toEqual({ rows: 3, cols: null });
  });
  it("strips sheet prefix", () => {
    expect(rangeDims("Sheet1!A1:B2")).toEqual({ rows: 2, cols: 2 });
  });
  it("returns null for unparseable", () => {
    expect(rangeDims("A1:B2:C3")).toBeNull();
    expect(rangeDims("foo")).toBeNull();
  });
});

describe("validateMatrixShape", () => {
  it("accepts matching matrix", () => {
    expect(() =>
      validateMatrixShape("setValues", "A1:B2", [
        [1, 2],
        [3, 4],
      ]),
    ).not.toThrow();
  });
  it("rejects jagged rows", () => {
    expect(() =>
      validateMatrixShape("setValues", "A1:C2", [
        [1, 2, 3],
        [4, 5],
      ]),
    ).toThrow(/jagged/);
  });
  it("rejects row-count mismatch", () => {
    expect(() => validateMatrixShape("setValues", "A1:B2", [[1, 2]])).toThrow(/expects 2 row/);
  });
  it("rejects col-count mismatch", () => {
    expect(() => validateMatrixShape("setValues", "A1:B2", [[1], [2]])).toThrow(/expects 2 column/);
  });
  it("rejects empty matrix", () => {
    expect(() => validateMatrixShape("setValues", "A1", [])).toThrow(/at least one row/);
    expect(() => validateMatrixShape("setValues", "A1", [[]])).toThrow(/at least one column/);
  });
  it("allows any shape for unparseable ref", () => {
    expect(() =>
      validateMatrixShape("setValues", "weird-ref", [
        [1, 2],
        [3, 4],
      ]),
    ).not.toThrow();
  });
  it("only checks cols for whole-column ranges", () => {
    expect(() =>
      validateMatrixShape("setValues", "A:B", [
        [1, 2],
        [3, 4],
        [5, 6],
      ]),
    ).not.toThrow();
    expect(() => validateMatrixShape("setValues", "A:B", [[1]])).toThrow(/expects 2 column/);
  });
});
