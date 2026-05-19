import { expect, test } from "vitest";
import { FILL_SENTINEL, formatValue } from "./numfmt";

const ACC = '_("$"* #,##0.00_);_("$"* (#,##0.00);_("$"* "-"??_);_(@_)';

test("accounting positive: $ + sentinel + digits + trailing-space", () => {
  const r = formatValue(80539, ACC);

  expect(r.text).toBe(` $${FILL_SENTINEL}80,539.00 `);
  expect(r.fills).toEqual([" "]);
});

test("accounting negative: routes to the second section + sentinel inside parens", () => {
  const r = formatValue(-1234.5, ACC);

  expect(r.text).toBe(` $${FILL_SENTINEL}(1,234.50)`);
  expect(r.fills).toEqual([" "]);
});

test("accounting zero: routes to the third section with placeholder dash", () => {
  const r = formatValue(0, ACC);

  expect(r.text).toBe(` $${FILL_SENTINEL}-   `);
  expect(r.fills).toEqual([" "]);
});

test("format with `0` placeholder still emits literal zero for value 0", () => {
  expect(formatValue(0, "0.00").text).toBe("0.00");
  expect(formatValue(0, "#,##0").text).toBe("0");
});

test("`?`-only int side emits blanks for zero, not a literal digit", () => {
  expect(formatValue(0, "??").text).toBe("  ");
  expect(formatValue(0, "???").text).toBe("   ");

  expect(formatValue(0, "#").text).toBe("");
});

test("non-accounting format has no fills array", () => {
  const r = formatValue(1234.5, "$#,##0.00");
  expect(r.text).toBe("$1,234.50");
  expect(r.fills).toBeUndefined();
  expect(r.text.includes(FILL_SENTINEL)).toBe(false);
});

test("custom fill char (not space) round-trips through fills[]", () => {
  const r = formatValue(42, "0*-");
  expect(r.text).toBe(`42${FILL_SENTINEL}`);
  expect(r.fills).toEqual(["-"]);
});

test("multiple fills in one section both surface in fills[]", () => {
  const r = formatValue(7, "*-0*=");
  expect(r.text).toBe(`${FILL_SENTINEL}7${FILL_SENTINEL}`);
  expect(r.fills).toEqual(["-", "="]);
});

test("General keyword splices formatGeneral output, with literal suffix", () => {
  expect(formatValue(45473, "General\\E").text).toBe("45473E");
  expect(formatValue(1.5, "General\\A").text).toBe("1.5A");

  expect(formatValue(42, "GENERAL\\F").text).toBe("42F");
});

test("negative-slot sections receive |value|", () => {
  expect(formatValue(-5, "0.0;General").text).toBe("5");
  expect(formatValue(-5, "General;General").text).toBe("5");

  expect(formatValue(-1234.5, "#,##0").text).toBe("-1,235");

  expect(formatValue(-5, "0.0;-0.0").text).toBe("-5.0");
  expect(formatValue(-1234.5, "#,##0;(#,##0)").text).toBe("(1,235)");
});
