import { expect, test } from "vitest";
import { FILL_SENTINEL, formatValue } from "./numfmt";

// Accounting `*x` fill: numfmt can't size the gap (no cell width here),
// so it emits a sentinel char per fill token plus a `fills[]` array.
// The textRenderer is what actually expands them. Here we lock down the
// section/sign routing + sentinel placement.

const ACC = '_("$"* #,##0.00_);_("$"* (#,##0.00);_("$"* "-"??_);_(@_)';

test("accounting positive: $ + sentinel + digits + trailing-space", () => {
  const r = formatValue(80539, ACC);
  // Layout per OOXML `_("$"* #,##0.00_)`:
  //   _(    → space placeholder (literal one-space pad)
  //   "$"   → literal $
  //   * <sp>→ FILL_SENTINEL with fill char = ' '
  //         (the `*` token consumes the next char as its fill char,
  //          so there's no separate space literal between sentinel
  //          and digits)
  //   #,##0.00 → 80,539.00
  //   _)    → trailing space
  expect(r.text).toBe(` $${FILL_SENTINEL}80,539.00 `);
  expect(r.fills).toEqual([" "]);
});

test("accounting negative: routes to the second section + sentinel inside parens", () => {
  const r = formatValue(-1234.5, ACC);
  // Negative section `_("$"* (#,##0.00)` — sentinel sits between
  // `$` and `(`, with no literal space (the `*` consumed the space).
  expect(r.text).toBe(` $${FILL_SENTINEL}(1,234.50)`);
  expect(r.fills).toEqual([" "]);
});

test("accounting zero: routes to the third section with placeholder dash", () => {
  const r = formatValue(0, ACC);
  // `_("$"* "-"??_)` — "-" literal then two `?` digit placeholders. The
  // `??` int side has no `0` placeholder to anchor a literal zero, so
  // Excel renders absolute zero as `"-  "` (dash + two blanks). The
  // trailing space comes from the `_)` literal.
  expect(r.text).toBe(` $${FILL_SENTINEL}-   `);
  expect(r.fills).toEqual([" "]);
});

test("format with `0` placeholder still emits literal zero for value 0", () => {
  // `0` placeholder anchors a digit even when the value is zero; only
  // `?`/`#`-only int sides trigger the suppression.
  expect(formatValue(0, "0.00").text).toBe("0.00");
  expect(formatValue(0, "#,##0").text).toBe("0");
});

test("`?`-only int side emits blanks for zero, not a literal digit", () => {
  expect(formatValue(0, "??").text).toBe("  ");
  expect(formatValue(0, "???").text).toBe("   ");
  // `#`-only emits nothing.
  expect(formatValue(0, "#").text).toBe("");
});

test("non-accounting format has no fills array", () => {
  const r = formatValue(1234.5, "$#,##0.00");
  expect(r.text).toBe("$1,234.50");
  expect(r.fills).toBeUndefined();
  expect(r.text.includes(FILL_SENTINEL)).toBe(false);
});

test("custom fill char (not space) round-trips through fills[]", () => {
  // `0*-` means: render 0, then fill with `-` to cell edge.
  const r = formatValue(42, "0*-");
  expect(r.text).toBe(`42${FILL_SENTINEL}`);
  expect(r.fills).toEqual(["-"]);
});

test("multiple fills in one section both surface in fills[]", () => {
  // Synthetic format with two `*` fills bracketing the number.
  const r = formatValue(7, "*-0*=");
  expect(r.text).toBe(`${FILL_SENTINEL}7${FILL_SENTINEL}`);
  expect(r.fills).toEqual(["-", "="]);
});
