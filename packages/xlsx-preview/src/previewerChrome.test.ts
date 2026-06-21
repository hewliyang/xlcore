import { describe, expect, it } from "vitest";
import { contrastingTextColor, normalizeSelection } from "./previewerChrome.js";

describe("contrastingTextColor", () => {
  it("returns dark text on light backgrounds", () => {
    expect(contrastingTextColor("#ffffff")).toBe("#111827");
  });
  it("returns light text on dark backgrounds", () => {
    expect(contrastingTextColor("#000000")).toBe("#ffffff");
  });
  it("falls back to dark text for invalid input", () => {
    expect(contrastingTextColor("red")).toBe("#111827");
  });
});

describe("normalizeSelection", () => {
  it("orders corners and clamps to bounds", () => {
    expect(normalizeSelection({ r1: 5, c1: 4, r2: 2, c2: 1 }, 10, 10)).toEqual({
      r1: 2,
      c1: 1,
      r2: 5,
      c2: 4,
    });
  });
  it("clamps out-of-range values", () => {
    expect(normalizeSelection({ r1: 0, c1: 99, r2: 99, c2: 0 }, 8, 8)).toEqual({
      r1: 1,
      c1: 1,
      r2: 8,
      c2: 8,
    });
  });
});
