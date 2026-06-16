import { expect, test } from "vitest";

import { lookupSignature, signatureAt } from "./formulaSignature.js";

test("=SUM( -> argIndex 0", () => {
  const text = "=SUM(";
  expect(signatureAt(text, text.length)).toEqual({ name: "SUM", argIndex: 0 });
});

test("=SUM(A1, -> argIndex 1", () => {
  const text = "=SUM(A1,";
  expect(signatureAt(text, text.length)).toEqual({ name: "SUM", argIndex: 1 });
});

test("=XIRR(A1:A3, B1:B3, -> name XIRR argIndex 2", () => {
  const text = "=XIRR(A1:A3, B1:B3,";
  expect(signatureAt(text, text.length)).toEqual({ name: "XIRR", argIndex: 2 });
});

test("nested IF not SUM after comma", () => {
  const text = "=IF(SUM(A1,A2)>0,";
  expect(signatureAt(text, text.length)).toEqual({ name: "IF", argIndex: 1 });
});

test("comma inside string does not advance argIndex", () => {
  const text = '=IF("a,b",';
  expect(signatureAt(text, text.length)).toEqual({ name: "IF", argIndex: 1 });
});

test("unterminated string does not crash", () => {
  const text = '=IF("a,';
  expect(signatureAt(text, text.length)).toEqual({ name: "IF", argIndex: 0 });
});

test("not inside call returns null", () => {
  expect(signatureAt("=SUM", 4)).toBeNull();
  expect(signatureAt("hello", 3)).toBeNull();
});

test("lookupSignature is case-insensitive", () => {
  expect(lookupSignature("sum")?.name).toBe("SUM");
  expect(lookupSignature("Xirr")?.args).toEqual(["values", "dates", "guess"]);
  expect(lookupSignature("UNKNOWN")).toBeNull();
});
