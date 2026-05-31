# Engine Parity Hillclimb

This is the working checklist for getting xlcore-engine plus our vendored
IronCalc fork from a thin facade to an Excel-compatible recalc layer that can
feed xlcore-export, chart refs, conditional formatting, sparklines, and the
future agent mutation API.

## Current State

Implemented:

- crates/xlcore-engine wraps ironcalc_base behind WorkbookEngine.
- ironcalc_base is vendored in crates/ironcalc-base and resolved as a workspace
  path dependency.
- Same-sheet formulas, ranges, cross-sheet refs, and formula prefix handling are
  covered by Rust unit tests.
- SUMPRODUCT is implemented in the vendored fork and covered by both IronCalc
  internal tests and xlcore-engine tests.
- crates/xlcore-bridge has the first OOXML harvest -> engine evaluate ->
  recalculated formula value API, covered by tests/fixtures/engine/basic-formulas.xlsx.
- crates/xlcore-bridge can write recalculated scalar formula values back into
  cached <v> nodes and extract WorkbookLayout from the recalculated document,
  covered by tests/fixtures/engine/stale-formulas.xlsx.
- Shared formula groups (<f t="shared">) are expanded into per-cell formulas
  for ordinary A1-style refs, then written back per cell. Covered by
  tests/fixtures/engine/shared-formulas.xlsx.
- Unsupported formulas that surface as #NAME?, #N/IMPL, or #ERROR! now preserve
  their source cached <v> values and report a per-cell fallback diagnostic.
  Covered by tests/fixtures/engine/unsupported-formulas.xlsx.
- A narrow compatibility shim rewrites top-level scalar LET(...) formulas into
  ordinary formulas before IronCalc sees them. This is a proof of concept, not
  the long-term implementation.

Not implemented yet:

- Array formulas, dynamic spill ranges, tables, structured refs, and workbook
  defined-name evaluation beyond what IronCalc already handles.
- Most missing modern functions and Excel edge semantics.

## Parity Ladder

### P0: Bridge And Recalc Basics

Goal: open an .xlsx, evaluate existing formulas, and write computed values back
without harming round-trip fidelity.

- Harvest workbook sheets into WorkbookEngine. Basic scalar harvest is started.
- Preserve sheet order and names exactly enough for formulas. Started.
- Load literals: numbers, strings, booleans, errors, blanks. Basic scalar load is started.
- Load formulas from <f> and keep cached <v> as fallback on errors. Formula load is started.
- Expand shared formulas before handing them to the engine. Started for
  ordinary A1-style refs, including $ row/column anchors and simple ranges.
- Evaluate dependencies across sheets.
- Write evaluated scalar values back into <v> and layout cell values. Started
  for ordinary scalar formula cells.
- Preserve formulas, styles, comments, charts, tables, drawings, and unknown XML.
- Handle circular refs, parse errors, and unsupported functions deterministically.
  Unsupported-function fallback is started for scalar formula cells.

Acceptance: a basic formula fixture mutates inputs, recalculates totals, and the
exported layout plus rewritten workbook show the new values while unrelated OOXML
round-trips unchanged.

### P1: Functions Blocking Existing Fixtures

Goal: unblock the formulas already present in kitchensink.xlsx and normal
agent-generated spreadsheets.

- scalar LET
- SUBTOTAL behavior for tables and hidden rows
- text/date/number coercion edge cases used by formatting and CF
- XMATCH if we start accepting workbooks that pair it with XLOOKUP

Acceptance: focused unit corpus plus hsx-authored .xlsx fixtures match
Excel/SpreadJS cached values for scalar formulas.

### P1: Dynamic Arrays

Goal: modern formulas that return spill ranges.

- SEQUENCE
- FILTER
- SORT, SORTBY
- UNIQUE
- HSTACK, VSTACK
- TAKE, DROP, CHOOSECOLS, CHOOSEROWS
- TOCOL, TOROW
- #SPILL!, blocked spill ranges, and anchor-cell metadata

Acceptance: anchor formula plus spilled cells are represented in engine output,
written to layout JSON, and rendered correctly. This likely requires changes
inside IronCalc because Model::set_cell_value currently degrades arrays into a
not-implemented error.

### P2: LAMBDA Family

Goal: user-defined calculations and higher-order formulas.

- LAMBDA
- named LAMBDA through workbook defined names
- MAP, BYROW, BYCOL
- REDUCE, SCAN
- MAKEARRAY
- recursion limits and error messages

Acceptance: named and inline lambdas match Excel for scalar and array-returning
cases. This is not a string rewrite problem; it needs lexical environments,
callable values, and array-aware evaluation.

### P2: Long Tail

- AGGREGATE
- advanced financial/statistical functions not already in IronCalc
- structured refs and table totals semantics
- formula-driven conditional formatting
- chart and sparkline formula-only source cells
- data validation formulas
- autoFilter re-evaluation

## Testing Strategy

Use three layers. Keep each failure small enough to diagnose.

### 1. Rust Unit Corpus

Use cargo test -p xlcore-engine for parser, shim, and scalar engine behavior.
These tests should not need .xlsx files.

Pattern:

~~~rust
let mut engine = WorkbookEngine::new("case").unwrap();
engine.set_input(0, 1, 1, "10").unwrap();
engine.set_formula(0, 2, 1, "SUM(A1:A1)").unwrap();
engine.evaluate();
assert_eq!(engine.cell_value(0, 2, 1).unwrap(), CellValue::Number(10.0));
~~~

Use this layer for:

- arithmetic and references
- function semantics
- error propagation
- circular refs
- formula rewrite/shim behavior
- type coercion

### 2. OOXML Bridge Fixtures

Add tests/fixtures/engine/*.xlsx with companion builders. Use hsx to create
normal workbooks because it produces realistic SpreadsheetML quickly.

Example builder shape:

~~~bash
#!/usr/bin/env bash
set -euo pipefail

F="tests/fixtures/engine/basic-formulas.xlsx"
mkdir -p "$(dirname "$F")"
hsx create "$F" >/dev/null
hsx set "$F" "Sheet1!A1:C4" '[
  [{"value":"Q1"},{"value":"Q2"},{"value":"Total"}],
  [{"value":10},{"value":15},{"formula":"=SUM(A2:B2)"}],
  [{"value":20},{"value":25},{"formula":"=SUM(A3:B3)"}],
  [{"formula":"=SUM(A2:A3)"},{"formula":"=SUM(B2:B3)"},{"formula":"=SUM(C2:C3)"}]
]'
~~~

For each fixture, store expected values as JSON next to the workbook once the
bridge exists:

~~~json
[
  { "sheet": "Sheet1", "cell": "C2", "value": 25 },
  { "sheet": "Sheet1", "cell": "C4", "value": 70 }
]
~~~

Use this layer for:

- reading formulas from OOXML
- shared formula expansion
- cached value fallback
- writeback into <v>
- sheet names with spaces and quotes
- defined names
- table refs
- spill metadata

hsx is useful for authoring and screenshots. When hsx cannot calculate a new
Excel function, use it only to write the workbook, then capture expected values
from Excel desktop or LibreOffice and commit those expected values.

### 3. Renderer And Workflow Fixtures

Once recalc changes layout values, use existing preview checks:

~~~bash
cargo build --release
pnpm --filter @hewliyang/xlsx-preview build
node packages/xlsx-preview/dist/cli.js tests/fixtures/engine/basic-formulas.xlsx -o /tmp/ours.png --scale 2
hsx screenshot tests/fixtures/engine/basic-formulas.xlsx -o /tmp/hsx.png
~~~

Use this layer only for formula behavior that affects visuals:

- formula-driven conditional formatting
- chart source ranges
- sparkline source ranges
- spilled cells visible in the grid
- table totals

## Fixture Matrix

Start with these cases, in this order:

| Fixture | Purpose | Oracle |
| --- | --- | --- |
| basic-formulas.xlsx | arithmetic, ranges, SUMPRODUCT | hsx/Excel |
| shared-formulas.xlsx | <f t="shared"> expansion and writeback | Excel |
| unsupported-formulas.xlsx | unsupported function fallback and reporting | Excel cache |
| errors.xlsx | #DIV/0!, #VALUE!, #NAME?, circular refs | Excel |
| coercion.xlsx | text/number/bool coercion | Excel |
| defined-names.xlsx | workbook and sheet-local names | Excel |
| sumproduct-let.xlsx | first missing modern scalar functions | Excel/hsx where supported |
| dynamic-arrays.xlsx | SEQUENCE, FILTER, SORT, UNIQUE | Excel |
| lambda.xlsx | inline and named LAMBDA | Excel |
| cf-expression.xlsx | formula conditional formatting | Excel visual + JSON |
| chart-sources.xlsx | chart refs backed only by formulas | hsx screenshot |

## Extending IronCalc

There are two tracks.

### Compatibility Shims

Use shims only when a formula can be transformed into formulas IronCalc already
supports. The current PoC is prepare_formula_for_ironcalc() in
crates/xlcore-engine/src/formula.rs.

Good shim candidates:

- simple scalar LET
- aliases or _xlfn. cleanup
- small formula normalizations at import boundaries

Bad shim candidates:

- LAMBDA
- dynamic arrays
- spill behavior
- functions needing lazy evaluation, ranges as first-class values, or workbook
  mutation side effects

### Proper Fork Changes

For real function support, patch the vendored ironcalc_base fork:

1. Add a variant to src/functions/mod.rs::Function.
2. Add the function name to lookup. For broad localized support, add it to
   src/language/mod.rs::Functions, src/language/language.json, the generated
   language.bin, and the impl_function_lookup! macro. For a narrow Excel-canonical
   name, a direct lookup special case is acceptable.
3. Implement the function in the right module, or add a new module such as
   dynamic_arrays.rs.
4. Add a match arm in Model::evaluate_function.
5. Use Node args directly for functions that need lazy evaluation. Do not
   eagerly evaluate arguments for IF, LET, LAMBDA, FILTER, or higher-order array
   functions.
6. Add tests under IronCalc's own src/test/ corpus and mirror the important
   cases in crates/xlcore-engine.

Relevant IronCalc 0.7.1 paths:

- crates/ironcalc-base: vendored fork root.
- src/expressions/parser/mod.rs: function names become FunctionKind, unknown
  names become InvalidFunctionKind.
- src/functions/mod.rs: function enum, lookup macro, and dispatch.
- src/model.rs: node evaluation, error handling, and formula result writeback.
- src/language/language.json: localized function names.

LET should become a real evaluator that creates a lexical binding frame and
evaluates the final expression in that frame. LAMBDA needs callable values,
argument binding, named-function integration, and recursion limits. Dynamic
array functions need CalcResult::Array to spill into cells instead of becoming
#NIMPL!.

## Definition Of Done

A formula feature is done only when all of these are true:

- Rust unit tests cover scalar behavior and error cases.
- At least one .xlsx fixture proves import and writeback.
- Expected values are captured from Excel/SpreadJS/LibreOffice, with divergence
  documented.
- Layout JSON exposes recalculated values.
- Renderer behavior is checked when the feature affects visuals.
- Unsupported or failed formulas preserve source formulas and cached values.
