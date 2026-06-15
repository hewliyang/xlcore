# Surface formula errors in the cell value

## Problem

Authoring a formula that errors (e.g. `=1/0`, `=NOTAFUNC()`) via the editing API
leaves the cell **blank** instead of showing `#DIV/0!` / `#NAME?`. The error
literal survives only in the recalc report's `fallback` field; it never reaches
the cell value, the doc XML, or the renderer.

Confirmed via `Workbook.create()` + `setFormula("=1/0")` + `recalculate()`:
`cell.value()` returns `{type:"blank"}`, report has
`fallback:{kind:"#DIV/0!"}`.

## Root cause (engine/value layer, NOT previewer)

The renderer already handles errors: `cellText.ts` case `"e"` renders the
centered error literal. Real `.xlsx` files look fine because Excel cached
`t="e" v="#DIV/0!"` and the renderer reads it straight from XML. The break is
only for cells we compute with no prior cache.

Three spots in `crates/xlcore-bridge/src/lib.rs`:

1. `evaluated_formula_value` — on a genuine engine error it always sets
   `*fallback = Some(...)` and returns `cached_value.unwrap_or(Blank)`,
   discarding the error literal.
2. `EngineCellValue` (`crates/xlcore-types/src/engine.rs`) has only
   `Blank | String | Number | Boolean` — no `Error` variant, so the recalc
   `value` cannot represent an error. (`ApiCellValue` already has `Error(String)`.)
3. `write_cached_formula_values` does `if update.fallback.is_some() { continue; }`,
   so even a non-blank value would not be written into the doc XML.

## Decision

Split engine errors by kind:

- **Genuine Excel errors** -> surface the literal as the cell value (this is what
  Excel shows): `#NULL! #DIV/0! #VALUE! #REF! #NAME? #NUM! #N/A #SPILL! #CALC! #CIRC!`.
- **Engine limitations** -> keep today's conservative fallback-to-cached behavior
  (our engine is the unreliable party; trust the file): `#N/IMPL`, and the
  synthesized `#ERROR!` ("Unevaluated formula" / `set_formula` failure).

Error kind strings come from `crates/ironcalc-base/src/expressions/token.rs`
(`Display for Error`) and `crates/xlcore-engine/src/lib.rs::formula_error`.

## Shipped

### Item 1 — plumb error literals through recalc + writeback

Added `Error(String)` to `EngineCellValue`; genuine errors surface as `value:
Error(kind)` with no fallback and are written as `t="e" v=kind` in the doc XML;
`t="e"` cells are now harvested as `Error` (not `String`); TS schema updated.

### Item 1b — key fallback off cached value, not error kind

Replaced `is_genuine_error` kind-list with a cached-value signal: if the file
holds a non-blank, non-error cached value trust it (set fallback, return `cv`)
otherwise surface the error literal as `CellValue::Error(kind)`. Restores
`preserves_cached_values_for_unsupported_formulas` to pre-Item-1 assertions.

### Item 2 — e2e dogfood (supervisor-verified)

Rebuilt wasm; `Workbook.create()` + `=1/0`/`=NOTAFUNC()`/`=1+"x"`/`=OFFSET(A1,-5,0)`
return `{type:"error", value}` from `cell.value()`, survive save/reopen, and
render as `#DIV/0!` / `#NAME?` / `#VALUE!` in PNG. Unsupported-function cells
with a cached value still fall back to it
(`preserves_cached_values_for_unsupported_formulas`).

## Out of scope

- previewer.ts / render changes (already correct).
