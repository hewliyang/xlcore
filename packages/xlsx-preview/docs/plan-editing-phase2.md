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

(empty)

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
- Point mode: arrow-key "marching" ref insertion was skipped; refs come from
  pointer click / drag / shift-click only.

## Shipped

- P2.5 point mode (pure `caretAcceptsReference`/`applyReferenceAtCaret` with an active-ref span in `src/formulaPointMode.ts`; `InteractOptions.isPointModeActive`/`onPointModeRef`). While editing a `=`-formula with the caret accepting a reference, grid click inserts an A1 ref, drag sweeps a range, shift+click extends from the prior anchor, and typing/commit resets the span. Pointer interactions refocus the editor instead of the canvas. Arrow-key marching skipped (limitation).

- P2.4 function autocomplete dropdown (pure `autocompleteState` in `src/formulaAutocomplete.ts`; positioned listbox in `src/previewer.ts` anchored under the active editor, keyboard nav, Enter/Tab insert `NAME(`, Esc closes dropdown only; `engine.functionNames()` cached). No-op without engine.

- P2.3 live precedent highlighting wired to a `PreviewerEngine` (`PreviewerOptions.engine?`, rotating 7-color palette, pure `referencesToHighlights` mapping same-sheet refs in `src/highlights.ts`, recompute on edit/formula-bar/selection draw; examples host implements `engine` over `recalcWorkbook`). Boxes only; token coloring deferred.

- P2.2 highlight overlay in the renderer (`HighlightRange[]` on `RenderOptions`, pure `buildHighlightRects` builder in `src/highlights.ts`, drawn beneath `drawSelection`).
- P2.1 `Workbook.parseFormulaReferences` (refs from an uncommitted formula via shared `references_for_formula` core) + `Workbook.functionNames` (English catalog from `ironcalc_base::english_function_names`, enum-synced via `Function::into_iter`).
