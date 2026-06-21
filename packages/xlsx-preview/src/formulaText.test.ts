import { describe, expect, it, vi } from "vitest";
import type { Cell, Sheet } from "./types.js";

const cells = new Map<string, Cell | undefined>();

vi.mock("./columnar.js", () => ({
  findCell: (_sheet: Sheet, r: number, c: number) => cells.get(`${r},${c}`),
}));

const { balanceFormula, formatFormulaBar } = await import("./formulaText.js");

const sheet = {} as Sheet;

function setCell(cell: Record<string, unknown> | undefined): void {
  cells.clear();
  if (cell) cells.set("0,0", cell as unknown as Cell);
}

describe("balanceFormula", () => {
  it("passes through non-formula text", () => {
    expect(balanceFormula("hello")).toBe("hello");
    expect(balanceFormula("(unclosed")).toBe("(unclosed");
  });

  it("appends missing closing parens", () => {
    expect(balanceFormula("=SUM(A1")).toBe("=SUM(A1)");
    expect(balanceFormula("=SUM(A1,MIN(B1")).toBe("=SUM(A1,MIN(B1))");
  });

  it("leaves balanced formulas untouched", () => {
    expect(balanceFormula("=SUM(A1)")).toBe("=SUM(A1)");
  });

  it("ignores parens inside double-quoted strings", () => {
    expect(balanceFormula('=CONCAT("(")')).toBe('=CONCAT("(")');
    expect(balanceFormula('=A&"text )"')).toBe('=A&"text )"');
  });

  it("ignores parens inside single-quoted refs", () => {
    expect(balanceFormula("='Sheet (1)'!A1")).toBe("='Sheet (1)'!A1");
  });

  it("handles escaped quotes", () => {
    expect(balanceFormula('=CONCAT("a""(b"')).toBe('=CONCAT("a""(b")');
  });
});

describe("formatFormulaBar", () => {
  it("returns empty for missing cell", () => {
    setCell(undefined);
    expect(formatFormulaBar(sheet, { r: 0, c: 0 })).toBe("");
  });

  it("prefixes formulas with = when missing", () => {
    setCell({ formula: "SUM(A1)" });
    expect(formatFormulaBar(sheet, { r: 0, c: 0 })).toBe("=SUM(A1)");
  });

  it("keeps existing leading =", () => {
    setCell({ formula: "=SUM(A1)" });
    expect(formatFormulaBar(sheet, { r: 0, c: 0 })).toBe("=SUM(A1)");
  });

  it("stringifies plain values", () => {
    setCell({ value: 42 });
    expect(formatFormulaBar(sheet, { r: 0, c: 0 })).toBe("42");
  });

  it("joins rich runs", () => {
    setCell({ runs: [{ text: "a" }, { text: "b" }] });
    expect(formatFormulaBar(sheet, { r: 0, c: 0 })).toBe("ab");
  });
});
