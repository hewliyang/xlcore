import { describe, expect, it } from "vitest";
import { headerRange, parseRef } from "./pivotSource.js";

describe("parseRef", () => {
  it("splits sheet and a1", () => {
    expect(parseRef("Sheet1!A1:D10")).toEqual({ sheet: "Sheet1", a1: "A1:D10" });
  });
  it("unquotes quoted sheet names", () => {
    expect(parseRef("'My Sheet'!A1:B2")).toEqual({ sheet: "My Sheet", a1: "A1:B2" });
  });
  it("handles a bare reference", () => {
    expect(parseRef("A1:B2")).toEqual({ a1: "A1:B2" });
  });
});

describe("headerRange", () => {
  it("collapses a range to its first (header) row", () => {
    expect(headerRange("A1:D100")).toBe("A1:D1");
    expect(headerRange("B2:F50")).toBe("B2:F2");
  });
});
