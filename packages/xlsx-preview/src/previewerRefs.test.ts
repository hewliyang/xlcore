import { describe, expect, it } from "vitest";
import {
  colNameToIndex,
  findUnquotedBang,
  parsePointHighlight,
  resolveWorkbookLocation,
  unquoteSheetName,
} from "./previewerRefs.js";
import type { WorkbookLayout } from "./types.js";

function layoutWith(definedNames: WorkbookLayout["definedNames"]): WorkbookLayout {
  return {
    sheets: [{ name: "Sheet1" }, { name: "Data Sheet" }],
    definedNames,
  } as unknown as WorkbookLayout;
}

describe("colNameToIndex", () => {
  it("maps single and multi letter columns", () => {
    expect(colNameToIndex("A")).toBe(1);
    expect(colNameToIndex("Z")).toBe(26);
    expect(colNameToIndex("AA")).toBe(27);
    expect(colNameToIndex("aa")).toBe(27);
    expect(colNameToIndex("XFD")).toBe(16384);
  });
});

describe("findUnquotedBang / unquoteSheetName", () => {
  it("finds the bang outside quotes", () => {
    expect(findUnquotedBang("Sheet1!A1")).toBe(6);
    expect(findUnquotedBang("'A!B'!C1")).toBe(5);
    expect(findUnquotedBang("NoBangHere")).toBe(-1);
  });

  it("handles escaped quotes", () => {
    expect(findUnquotedBang("'O''Brien'!A1")).toBe(10);
  });

  it("unquotes sheet names", () => {
    expect(unquoteSheetName("Sheet1")).toBe("Sheet1");
    expect(unquoteSheetName("'Data Sheet'")).toBe("Data Sheet");
    expect(unquoteSheetName("'O''Brien'")).toBe("O'Brien");
    expect(unquoteSheetName("  Sheet1  ")).toBe("Sheet1");
  });
});

describe("resolveWorkbookLocation", () => {
  it("resolves a named range to a sheet cell", () => {
    const layout = layoutWith([
      { name: "MyName", formula: "'Data Sheet'!$C$5", localSheetId: undefined },
    ]);
    expect(resolveWorkbookLocation(layout, "#MyName", 0)).toEqual({
      sheetIndex: 1,
      r: 5,
      c: 3,
    });
  });

  it("resolves a direct reference", () => {
    const layout = layoutWith([]);
    expect(resolveWorkbookLocation(layout, "Sheet1!B2", 0)).toEqual({
      sheetIndex: 0,
      r: 2,
      c: 2,
    });
  });
});

describe("parsePointHighlight", () => {
  it("parses a single cell", () => {
    expect(parsePointHighlight("B3", "#fff")).toEqual({
      r1: 3,
      c1: 2,
      r2: 3,
      c2: 2,
      color: "#fff",
    });
  });

  it("parses a range and normalizes order", () => {
    expect(parsePointHighlight("C5:A2", "#000")).toEqual({
      r1: 2,
      c1: 1,
      r2: 5,
      c2: 3,
      color: "#000",
    });
  });

  it("returns null for invalid refs", () => {
    expect(parsePointHighlight("not-a-ref", "#fff")).toBeNull();
  });
});
