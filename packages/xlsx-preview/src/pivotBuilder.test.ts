import { describe, expect, it } from "vitest";
import { applyMove, headerRange, parseRef } from "./pivotBuilder.js";

const empty = { rows: [], columns: [], filters: [], values: [] };

describe("parseRef", () => {
  it("splits sheet and a1", () => {
    expect(parseRef("Sheet1!A1:D10")).toEqual({ sheet: "Sheet1", a1: "A1:D10" });
  });
  it("unquotes sheet names", () => {
    expect(parseRef("'My Sheet'!A1:B2")).toEqual({ sheet: "My Sheet", a1: "A1:B2" });
  });
  it("handles bare ranges", () => {
    expect(parseRef("A1:B2")).toEqual({ a1: "A1:B2" });
  });
});

describe("headerRange", () => {
  it("collapses to the first row", () => {
    expect(headerRange("A1:D100")).toBe("A1:D1");
    expect(headerRange("B2:F50")).toBe("B2:F2");
  });
});

describe("applyMove", () => {
  it("moves a field into rows", () => {
    expect(applyMove(empty, "Region", "available", "rows").rows).toEqual(["Region"]);
  });
  it("wraps a field into a value with default aggregation", () => {
    expect(applyMove(empty, "Amount", "available", "values").values).toEqual([
      { field: "Amount", aggregation: "sum" },
    ]);
  });
  it("removes from the source bucket on move", () => {
    const start = { ...empty, rows: ["Region"] };
    const next = applyMove(start, "Region", "rows", "columns");
    expect(next.rows).toEqual([]);
    expect(next.columns).toEqual(["Region"]);
  });
  it("preserves aggregation when reordering within values", () => {
    const start = { ...empty, values: [{ field: "Amt", aggregation: "average" as const }] };
    expect(applyMove(start, "Amt", "values", "values")).toBe(start);
  });
  it("is a no-op when from equals to", () => {
    const start = { ...empty, rows: ["Region"] };
    expect(applyMove(start, "Region", "rows", "rows")).toBe(start);
  });
});
