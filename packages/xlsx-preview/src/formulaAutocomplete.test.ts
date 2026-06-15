import { expect, test } from "vitest";

import { autocompleteState } from "./formulaAutocomplete.js";

const NAMES = [
  "ABS",
  "AVERAGE",
  "SUM",
  "SUMIF",
  "SUMIFS",
  "SUMPRODUCT",
  "VLOOKUP",
];

test("caret after = matches prefix", () => {
  const text = "=SU";
  const s = autocompleteState(text, text.length, NAMES);
  expect(s).not.toBeNull();
  expect(s!.token).toBe("SU");
  expect(s!.start).toBe(1);
  expect(s!.end).toBe(3);
  expect(s!.matches).toEqual(["SUM", "SUMIF", "SUMIFS", "SUMPRODUCT"]);
});

test("caret after ( matches", () => {
  const text = "=IF(SU";
  const s = autocompleteState(text, text.length, NAMES);
  expect(s!.token).toBe("SU");
  expect(s!.start).toBe(4);
  expect(s!.end).toBe(6);
});

test("caret after , matches", () => {
  const text = "=IF(A1,SU";
  const s = autocompleteState(text, text.length, NAMES);
  expect(s!.token).toBe("SU");
  expect(s!.start).toBe(7);
});

test("after operator with replacement offsets", () => {
  const text = "=A1+SU";
  const s = autocompleteState(text, text.length, NAMES);
  expect(s!.token).toBe("SU");
  expect(s!.start).toBe(4);
  expect(s!.end).toBe(6);
});

test("mid-name caret", () => {
  const text = "=SUM";
  const s = autocompleteState(text, 3, NAMES);
  expect(s!.token).toBe("SU");
  expect(s!.start).toBe(1);
  expect(s!.end).toBe(3);
});

test("no match returns null", () => {
  const text = "=ZZZ";
  expect(autocompleteState(text, text.length, NAMES)).toBeNull();
});

test("inside string literal returns null", () => {
  const text = '="SU';
  expect(autocompleteState(text, text.length, NAMES)).toBeNull();
});

test("empty token returns null", () => {
  expect(autocompleteState("=", 1, NAMES)).toBeNull();
  expect(autocompleteState("=A1+", 4, NAMES)).toBeNull();
});

test("not a formula returns null", () => {
  expect(autocompleteState("SU", 2, NAMES)).toBeNull();
});

test("token preceded by name char (cell ref) returns null", () => {
  const text = "=A1SU";
  expect(autocompleteState(text, text.length, NAMES)).toBeNull();
});

test("caps match length", () => {
  const many = Array.from({ length: 30 }, (_, i) => `SUM${i}`);
  const s = autocompleteState("=SUM", 4, many);
  expect(s!.matches.length).toBe(12);
});
