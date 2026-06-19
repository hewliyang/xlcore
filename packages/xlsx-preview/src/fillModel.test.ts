import { describe, expect, it } from "vitest";
import { projectFill } from "./fillModel.js";

describe("projectFill", () => {
  it("extends down by tiling rows", () => {
    const src = [["a"], ["b"]];
    const out = projectFill(src, { r1: 1, c1: 1, r2: 6, c2: 1 }, { r1: 1, c1: 1, r2: 2, c2: 1 });
    expect(out).toEqual([["a"], ["b"], ["a"], ["b"], ["a"], ["b"]]);
  });

  it("extends right by tiling cols", () => {
    const src = [["x", "y"]];
    const out = projectFill(src, { r1: 1, c1: 1, r2: 1, c2: 5 }, { r1: 1, c1: 1, r2: 1, c2: 2 });
    expect(out).toEqual([["x", "y", "x", "y", "x"]]);
  });

  it("repeats a single-cell source", () => {
    const src = [["z"]];
    const out = projectFill(src, { r1: 1, c1: 1, r2: 3, c2: 1 }, { r1: 1, c1: 1, r2: 1, c2: 1 });
    expect(out).toEqual([["z"], ["z"], ["z"]]);
  });

  it("tiles a 2x2 block", () => {
    const src = [
      ["1", "2"],
      ["3", "4"],
    ];
    const out = projectFill(src, { r1: 1, c1: 1, r2: 4, c2: 4 }, { r1: 1, c1: 1, r2: 2, c2: 2 });
    expect(out).toEqual([
      ["1", "2", "1", "2"],
      ["3", "4", "3", "4"],
      ["1", "2", "1", "2"],
      ["3", "4", "3", "4"],
    ]);
  });
});
