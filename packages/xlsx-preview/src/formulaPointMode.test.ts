import { expect, test } from "vitest";

import { applyReferenceAtCaret, caretAcceptsReference } from "./formulaPointMode.js";

test("accepts after =", () => {
  expect(caretAcceptsReference("=", 1)).toBe(true);
});

test("accepts after operators and (", () => {
  for (const t of ["=A1+", "=A1*", "=A1-", "=A1/", "=A1^", "=A1&", "=A1<", "=A1>", "=SUM("]) {
    expect(caretAcceptsReference(t, t.length)).toBe(true);
  }
});

test("accepts after comma", () => {
  expect(caretAcceptsReference("=SUM(A1,", 8)).toBe(true);
});

test("accepts after space", () => {
  expect(caretAcceptsReference("=A1 ", 4)).toBe(true);
});

test("rejects after digit", () => {
  expect(caretAcceptsReference("=A1", 3)).toBe(false);
});

test("rejects after )", () => {
  expect(caretAcceptsReference("=SUM(A1)", 8)).toBe(false);
});

test("rejects inside string literal", () => {
  expect(caretAcceptsReference('="hi ', 5)).toBe(false);
});

test("rejects non-formula", () => {
  expect(caretAcceptsReference("A1+", 3)).toBe(false);
});

test("rejects caret 0", () => {
  expect(caretAcceptsReference("=A1", 0)).toBe(false);
});

test("insert ref at caret then replace in span", () => {
  const ins = applyReferenceAtCaret("=", 1, "A1", null);
  expect(ins.text).toBe("=A1");
  expect(ins.caret).toBe(3);
  expect(ins.span).toEqual({ start: 1, end: 3 });

  const rep = applyReferenceAtCaret(ins.text, ins.caret, "A1:B3", ins.span);
  expect(rep.text).toBe("=A1:B3");
  expect(rep.caret).toBe(6);
  expect(rep.span).toEqual({ start: 1, end: 6 });
});

test("insert when span end mismatches caret appends", () => {
  const r = applyReferenceAtCaret("=A1+", 4, "B2", { start: 1, end: 3 });
  expect(r.text).toBe("=A1+B2");
  expect(r.caret).toBe(6);
  expect(r.span).toEqual({ start: 4, end: 6 });
});

test("replace mid-formula in span", () => {
  const r = applyReferenceAtCaret("=SUM(A1,B2)", 10, "B2:C3", { start: 8, end: 10 });
  expect(r.text).toBe("=SUM(A1,B2:C3)");
  expect(r.span).toEqual({ start: 8, end: 13 });
});
