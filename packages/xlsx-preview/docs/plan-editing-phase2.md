# Plan: Excel-grade formula editing UX (Phase 2)

Builds on `plan-editing.md` (Phase 1 shipped: `celledit` event, inline overlay,
formula bar, examples host with recalc loop). Phase 2 adds the three "feels like
Excel" interactions when authoring formulas:

1. **Precedent highlighting** — selecting/editing a formula cell draws colored
   boxes around the ranges it references (and tints the matching tokens).
2. **Function autocomplete** — a dropdown of function names as you type.
3. **Point mode** — clicking / shift-clicking / arrowing the grid while editing
   inserts or extends an A1 reference into the formula instead of moving the
   cursor.

No shortcuts: references and function names come from the real ironcalc parser /
catalog via wasm, not a JS regex guess. The previewer stays render-only over a
`WorkbookLayout`; the host owns the live `Workbook`. New capability the previewer
needs from the host is injected through a small `PreviewerEngine` adapter.

## Architecture

- Backend already exposes (wasm `WorkbookHandle` + TS `Workbook`/`Range`):
  `precedents`, `dependents`, `dependencies`, `setFormula`, `recalculate`,
  `layout`. Reference extraction lives in `crates/xlcore-api/src/dependencies.rs`
  (`collect_formula_references` → ironcalc `new_parser_english`).
- Missing backend: (a) parse references from an **uncommitted** formula string
  (for live highlight while typing), (b) the **function-name catalog** for
  autocomplete. Both added in P2.1.
- New previewer seam: `PreviewerOptions.engine?: PreviewerEngine` with
  `parseReferences(sheetName, anchorRef, formula): DependencyReference[]` and
  `functionNames(): string[]`. The examples host implements it over its
  `recalcWorkbook`. Highlighting/autocomplete/point-mode are no-ops when `engine`
  is absent.
- Same-sheet only for highlighting (cross-sheet refs are parsed but not drawn;
  documented limitation).

## Conventions (every item)

- Run `pnpm --filter @hewliyang/xlsx-preview check` before committing. Backend
  items also need `pnpm --filter @hewliyang/xlsx-preview build:wasm` then
  `pnpm --filter @hewliyang/xlsx-preview test` (the test script gates on
  `check-wasm-fresh`).
- No comments/docstrings (AGENTS.md). Conventional commits. Terse CHANGELOG entry.
- Prefer extracting pure logic into its own module so it is unit-testable without
  a DOM (no jsdom in this repo). Canvas/interaction glue stays thin.
- Update this doc (move the item to **Shipped**, terse) as part of each commit.

---

## Backlog

### P2.1 feat(api): uncommitted-formula reference parse + function catalog

Backend. `crates/xlcore-api`, `crates/xlcore-wasm`, TS `Workbook`, schema/api.

- `Workbook::parse_formula_references_in(sheet, anchor_ref, formula) ->
  Vec<DependencyReference>`: reuse `collect_formula_references` machinery in
  `dependencies.rs` but on an arbitrary `formula` string anchored at `anchor_ref`
  (no cell read). Refactor the existing `precedents` to share the same core.
- Function catalog: `Workbook::function_names() -> Vec<String>` (sorted, unique,
  English canonical names). Source from ironcalc. Cleanest is to add the list in
  `ironcalc-base` (e.g. a `pub fn english_function_names()` derived from the
  `impl_function_lookup!` table or the localized-name arms) with a coverage test
  vs the `Function` enum — pick whichever keeps it in sync without a hand list.
- Expose both via the `api_methods!` table in `crates/xlcore-wasm/src/lib.rs`
  (`parse_formula_references_in as "parseFormulaReferences"`, `function_names as
  "functionNames"`), add TS wrappers (`Workbook.parseFormulaReferences(...)`,
  `Workbook.functionNames()`), regen schema/api manifest, rebuild wasm.
- Verify (vitest, real wasm): `parseFormulaReferences("Sheet1","A1","=B1+SUM(C1:C3)")`
  → refs for `B1` and `C1:C3`; `functionNames()` includes `SUM`,`IF`,`XLOOKUP`,
  is uppercase + sorted, length > 300. Gotcha: must run `build:wasm` first or the
  freshness gate fails.

### P2.2 feat(previewer): highlight overlay in the renderer

Library. `src/render.ts` (+ `src/selection.ts` or a new `src/highlights.ts`),
`src/renderTypes.ts`.

- Add `highlights?: HighlightRange[]` to `RenderOptions` where
  `HighlightRange = { r1; c1; r2; c2; color: string }`.
- Pure builder: given grid + highlights → list of `{x,y,w,h,color}` rects (mirror
  `drawSelection` geometry using `colX/rowY`). Draw a 2px stroke in `color` + a
  faint fill (color at ~10% alpha) inside the existing pane clip loop, beneath
  `drawSelection` so the active selection stays on top.
- Unit-test the pure rect builder (geometry + clamping). Visual check: render a
  fixture layout to PNG with injected highlights via the CLI/`node.ts` path and
  eyeball.
- Verify: `pnpm --filter @hewliyang/xlsx-preview test` green; PNG shows boxes.

### P2.3 feat(previewer): live precedent highlighting wired to the engine

Library + examples. `src/previewer.ts`, `src/interact.ts` (read-only), examples.

- Add `PreviewerEngine` interface + `PreviewerOptions.engine?`. Methods used here:
  `parseReferences(sheetName, anchorRef, formula): DependencyReference[]`.
- Maintain a rotating palette (Excel-like ~7 colors). When a formula cell is the
  active cell, or while the inline/formula-bar editor holds a `=`-formula, compute
  references from the **current editor text** (or the committed `cell.formula` when
  not editing) via `engine.parseReferences`, map same-sheet refs → `highlights`
  with cycled colors, and redraw. Recompute on each keystroke in the editor
  (debounce a frame). Clear highlights when leaving edit / non-formula cell.
- Tint the matching ref tokens inside the editor with the same colors — only
  feasible in a styled overlay, not a bare `<input>`. MVP: skip token coloring
  (boxes only); note as a limitation / follow-up (needs contenteditable editor).
- Examples host: implement `engine` over `recalcWorkbook`
  (`parseReferences` → `wb.parseFormulaReferences(sheet, anchor, formula)`).
- Verify (browser-harness, see E2E): click a formula cell → boxes appear; type a
  formula referencing cells → boxes update live; commit/Esc → boxes clear.

### P2.4 feat(previewer): function autocomplete dropdown

Library + examples. new `src/formulaAutocomplete.ts`, `src/previewer.ts`.

- Pure model: `autocompleteState(text, caretIndex, names) -> { token, start, end,
  matches } | null` — find the function-name token under the caret (alpha run
  bounded by `=`,(`,`,`,operators,space), prefix-match (case-insensitive) against
  `names`, cap matches. Unit-test thoroughly (caret at start/end, after `(`, after
  `,`, no match, inside a string literal → null).
- UI: a positioned dropdown anchored to the active editor (reuse the popover
  anchoring style from `pivotFilterPopover.ts`/`tableFilterPopover.ts`). Keyboard:
  ArrowUp/Down move, Tab/Enter accept (insert `NAME(` replacing the token), Esc
  closes (and does not also cancel the edit on first press). Mouse click accepts.
  Hide when no matches / not a function position.
- `engine.functionNames()` feeds the list (fetched once, cached). No-op without
  engine.
- Verify (browser-harness): type `=SU` → dropdown with SUM/SUMIF/...; Down+Enter
  inserts `SUM(`; Esc closes.

### P2.5 feat(previewer): point mode (click/shift-select to insert refs)

Library + examples. new `src/formulaPointMode.ts`, `src/previewer.ts`,
`src/interact.ts`.

- Pure: `caretAcceptsReference(text, caretIndex) -> boolean` (caret right after
  `=`, an operator, `(`, or `,`, and not inside a string literal); and
  `applyReferenceAtCaret(text, caretIndex, ref) -> { text, caret }` that inserts a
  new ref or replaces the just-inserted ref (track an "active ref span" so dragging
  / shift-extending rewrites it rather than appending). Unit-test both.
- Wire: while an edit is active and the caret accepts a reference, route grid
  interactions through point mode instead of moving the active cell —
  - pointer click on a cell → insert that cell's A1 ref; drag → replace with the
    swept range; shift+click → extend to a range.
  - arrow keys → start/extend a "marching" reference; typing any non-nav key exits
    point mode and resumes normal text editing.
  Add the needed callbacks/flags to `InteractOptions` (e.g. `isPointModeActive()`
  + `onPointModeRef(rangeRef)`), keeping non-edit behavior unchanged.
- Examples: works through the same `editable`/`engine` wiring (no host change
  expected beyond Phase 1).
- Verify (browser-harness): F2 on a cell, type `=`, click another cell → its ref
  appears; drag → range ref; arrows → ref marches; Enter commits the formula and
  it recalculates.

---

## E2E harness (browser)

1. `cd packages/xlsx-preview && pnpm build:ts` (and `node scripts/make-recalc-fixture.mjs`
   if `examples/recalc-demo.xlsx` is missing).
2. `PORT=8765 node scripts/preview.mjs` — app at
   `/packages/xlsx-preview/examples/xlsx-app.html`.
3. Use the browser-harness skill (CDP) to load `recalc-demo.xlsx` and exercise the
   item's scenario; screenshot each step.

Pure-logic items (`*.test.ts`) are verified by `pnpm test`. Visual-only render
checks use a CLI PNG render + eyeball. Interactive items use the browser harness.

## Known limitations (document, don't fix now)

- Cross-sheet precedents parsed but not highlighted (only active sheet drawn).
- Token coloring inside the editor deferred (needs a contenteditable editor to
  replace the `<input>`); boxes-only for now.
- No structured-table / spilled-array ref highlighting beyond what the parser
  returns as plain areas.

## Shipped

_(move items here as they land, terse)_
