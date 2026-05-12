# numfmt triage (2026-05-09)

Built four narrow fixtures to map exactly which number-format families
break in `packages/xlsx-preview/src/render.ts::formatNumber`. Each `.xlsx` ships
with both its `*.hsx.png` ground-truth screenshot and a current
`*.ours.png` for side-by-side review. Rerun the visual diff with the
workflow in [`TESTING.md`](../../../docs/TESTING.md).

## Status (2026-05-09, after evaluator rewrite)

| Fixture                          | Before     | After     | Notes                                                                                                                                  |
| -------------------------------- | ---------- | --------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| `date-time-formats.xlsx`         | ❌ all 13  | ✅ 13/13  | Full date/time renderer (date tokens, AM/PM, `[h]`/`[mm]`/`[ss]` elapsed, `mm:ss.0` sub-second). Built-in IDs 45–49 added to renderer. |
| `currency-locale.xlsx`           | 🟡 4/10    | ✅ 10/10  | `[$sym-loc]` parsed; `_x` width tokens emit a space; `*x` fill tokens collapse to nothing (no cell-width plumbing yet).                 |
| `custom-section-conditions.xlsx` | ❌ all 12  | ✅ 12/12  | `[Red]` / `[Color12]` propagate to text color; `[>0]` / `[<10]` conditional gates respected; trailing-comma scaling stripped from lits. |
| `fraction-and-scientific.xlsx`   | ❌ all 10  | ✅ 10/10  | Stern–Brocot for variable-denom fractions; fixed denom (`?/8`, `?/16`); engineering-shift scientific (`##0.0E+0` → `123.5E+5`).        |

All four fixtures now match hsx output to the character. Kitchensink
verified non-regressed.

## What changed

Replaced the regex-based stub in `packages/xlsx-preview/src/render.ts::formatNumber`
with a real evaluator in `packages/xlsx-preview/src/numfmt.ts` (~700 LOC). Pipeline:

```
formatValue(value, fmt)
  → splitTopLevel(fmt, ";")           // section split, brace-aware
  → parseSection(raw)                  // tokenize + classify flavor + cache counts
  → pickSection(sections, value)       // [cond] gates first, else sign-based
  → renderSection(value, sec)          // dispatch by flavor
      → renderNumber / renderDate / renderFraction / renderScientific
```

Key design choices, documented inline:

- `,` and `/` are tokenized as **literals**, not dedicated tokens. Their
  meaning depends on the surrounding flavor: a `,` between digit
  placeholders is grouping; trailing `,` chars after the last placeholder
  are scale-by-1000 markers; a `,` inside a date section is just a comma.
  Same for `/`.
- The renderer walks digit placeholders right-to-left for the integer
  side and left-to-right for the fractional side; the leftmost
  placeholder gets all overflow so `#0` for value 12345 still emits
  `12345` not `5`.
- `[Color12]` indexed colors map to a small palette matching Excel's
  legacy color table; the resolved CSS color rides out on
  `FormatResult.color` and the renderer threads it into the cell's text
  spans as a font-color override.
- Excel built-ins **45–49** (`mm:ss`, `[h]:mm:ss`, `mm:ss.0`,
  `##0.0E+0`, `@`) were missing from `BUILTIN_NUMFMT`. Added the full
  ECMA-376 §18.8.30 table.

## Known small gaps (intentional, low priority)

- **Padding / fill**: ~~`*x` emits nothing~~ **DONE** — `*x` tokens now
  emit a `FILL_SENTINEL` (`\u0001`) in `FormatResult.text` plus a parallel
  `fills: string[]` carrying the fill char; the textRenderer measures the
  cell-primary span at `ownRect.w - padX*2` and substitutes N copies of
  the fill char per sentinel so accounting `_("$"* #,##0_)` packs as
  `$    80,539 ` flush against both cell edges. `_x` still emits a single
  space instead of measuring `x`'s glyph width — visually equivalent for
  the parens / space chars accounting formats actually use.
- **Per-format memoization**: `parseFormat` runs per cell. With dedupe
  it's cheap; a `Map<string, Section[]>` cache is a trivial follow-up.
- **Locale separator (`.` vs `,`)**: hardcoded en-US.
- **Pre-1900-03-01 dates**: serial < 60 is off by one day (Excel's bogus
  1900 leap year).
- **Asian / lunar calendar tokens** (`g`, `e`, `b1`, `b2`).
- ~~**Accounting-zero `??`**~~: **DONE.** Format `_("$"* "-"??_)` for
  value 0 now emits `"-  "` (dash + two blanks) to match Excel. Fix in
  `renderIntegerTokens` (`numfmtNumberParts.ts`): when the value's
  integer part is exactly `0` AND the format provides no `0`
  placeholder, treat `intDigits` as empty so `?` placeholders fall
  through to spaces and `#` placeholders emit nothing. Tests in
  `numfmt.test.ts` cover the accounting third section + bare `??` /
  `???` / `#` and verify `0.00` / `#,##0` still anchor a literal zero
  for value 0.

## Architectural followup (not blocking)

Move the **parser** (not the renderer) into the Rust extractor: parse
format codes once per workbook, ship a `Section` AST on
`WorkbookLayout.styles.numFmts[i].ast`, and have the TS evaluator walk
it. Cell-width-aware tokens (`_x`, `*x`, `###` overflow) stay on the
renderer side. Single grammar, single test surface, lighter JS bundle.
No urgency — the v0 surface fits.

---

## Original triage (preserved for reference)

## Root causes (current `formatNumber`, render.ts:144)

```ts
function formatNumber(value: number, fmt: string | undefined): string {
  const f = (fmt ?? "").trim();
  if (!f || f === "General") return formatGeneral(value);
  const stripped = f.replace(/\[[^\]]*\]/g, "");      // (1)
  const section = stripped.split(";")[0] ?? stripped; // (2)
  const decimals = decimalsIn(section);
  if (section.includes("%")) return (value * 100).toFixed(decimals) + "%";
  if (section.includes("$")) return "$" + withGrouping(value, decimals);  // (3)
  if (section.includes(",")) return withGrouping(value, decimals);
  if (section.includes("0") || section.includes("#")) return value.toFixed(decimals);
  return formatGeneral(value);
}
```

1. **`[..]` stripper is destructive**: `[$€-407]` → gone. Currency tags
   need to be parsed, not stripped — they carry the symbol + locale id.
2. **Section selection is value-blind**: positive→[0], negative→[1],
   zero→[2], text→[3], modulated by leading `[<n]` / `[>n]` conditions.
   Today everything uses [0] regardless of sign. (Also: when only one
   section is present, negative should auto-prefix `-`; we currently
   render `1234.50` for a value of `-1234.50` under format
   `#,##0.00;(#,##0.00);"-"` — but section[0] alone _without_ a negative
   section should still emit `-1,234.50`.)
3. **No date detection**: format codes containing `y/m/d/h/s` go through
   the `0`/`#` branch which `toFixed`'s the serial number. Excel uses the
   1900-based serial (1.0 = 1900-01-01, fractional = time of day).
   Implement date conversion + token-by-token expansion (`yyyy`, `yy`,
   `mmmm`, `mmm`, `mm`, `m`, `dddd`, `ddd`, `dd`, `d`, `hh`, `h`, `mm`
   — context-sensitive! — `ss`, `AM/PM`, `[h]` for elapsed-time clamp).
4. **No fraction synthesis** (`# ?/?`, `# ??/??`, `# ?/8`). Two flavors:
   variable-denominator (Stern–Brocot or simple continued-fraction
   search bounded by the `?` count) and fixed-denominator (`/8`, `/16`).
5. **No scientific** (`0.00E+00`). Standard JS `toExponential(decimals)`
   gets close but the integer-part width matters: `##0.0E+0` shifts the
   mantissa to make the exponent a multiple of 3 (engineering).
6. **Trailing-comma scaling**: each `,` _inside_ the `0`/`#` block but
   _not_ part of the grouping triple divides by 1000. `0.0,"K"` →
   `1500` renders as `1.5K`, not `1500.0K`. Easy to detect: count
   trailing commas after the last `0`/`#`.

## Recommended next step

Replace `formatNumber` with a real format-section evaluator. PARITY.md
already lists this as quick-win #4. Sketch:

```ts
type Section = {
  cond?: { op: ">" | "<" | ">=" | "<=" | "=" | "<>"; value: number };
  color?: string;
  tokens: Token[];   // literal | digit (0/#/?) | comma | dot | percent | currency | dateField | exponent | fraction
  scale: number;     // 10^(-3 * trailing-commas)
  isText: boolean;   // contains @
};
```

1. Tokenize the format string respecting `"…"`, `\x` escapes, and `[..]`
   tags (color names, condition `[op N]`, currency `[$sym-xxx]`).
2. Split on top-level `;` into 1–4 sections.
3. Pick a section based on (a) explicit `[cond]` if present, otherwise
   sign-based slot.
4. Render: digit grouping for `#,##0`, fixed denom or
   continued-fraction search for `?`, date-field expansion for date
   tokens, `toExponential` for `E+`, scale by trailing commas.

LibreOffice `sc/source/core/tool/zforfind.cxx` + `zformat.cxx` is the
spec-grade reference (the parser/runtime live there, ~5k LOC; a JS port
of the runtime alone is ~600 LOC).

## Files

```
date-time-formats.xlsx          # 13 date/time formats × one sample each
currency-locale.xlsx            # 10 currency / accounting / locale rows
custom-section-conditions.xlsx  # 4 multi-section formats × 3 sign cases
fraction-and-scientific.xlsx    # 5 fraction + 5 scientific rows
*.hsx.png                       # ground-truth screenshots (SpreadJS)
build-*.sh                      # reproducible builders (hsx-driven)
```

Rerun all builders:

```bash
for s in tests/fixtures/numfmt/build-*.sh; do bash "$s"; done
```
