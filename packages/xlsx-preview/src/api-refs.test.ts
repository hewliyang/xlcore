import { describe, expect, it } from "vitest";
import { rangeDims, validateMatrixShape } from "./api-refs.js";

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
