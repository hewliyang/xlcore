# Plan: cell editing in the previewer

Goal: let users edit cell values/formulas in the canvas previewer, recalc (auto or
manual), and re-render. The previewer stays render-only over a `WorkbookLayout`; the
**host app owns the live `Workbook` WASM handle** and applies mutations. The previewer
only surfaces an edit intent via a new `celledit` event.

## Architecture recap

- `createWorkbookPreviewerFromFile` decodes via a worker into a `WorkbookLayout` and
  renders it. It does **not** mutate anything.
- The example app (`examples/xlsx-app.html`) separately opens the same `.xlsx` into a
  main-thread `Workbook` (`recalcWorkbook`) which exposes the mutation + recalc API:
  - `wb.sheet(name).range(addr).setValue(input)` / `.setFormula("=...")`
  - `wb.recalculate()` (mutates in place) and `wb.layout()` (fresh layout)
- After any mutation the app calls `previewer.replaceLayout(wb.layout())`. `replaceLayout`
  preserves per-sheet `activeCell`/`selection`/scroll, so the cursor survives.
- Editing only applies to `.xlsx` (CSV/parquet have no `recalcWorkbook`). Gate edit UI on
  `recalcWorkbook != null`.

## Contract: the `celledit` event

The previewer emits a `celledit` `CustomEvent` whose `detail` is:

```ts
{ sheetIndex: number; r: number; c: number; input: string; commitMove?: "down" | "right" | "up" | "left" | null }
```

`input` is the raw committed string (leading `=` ⇒ formula). The host coerces and applies
it. `commitMove` hints how the active cell should advance after commit (Enter ⇒ down,
Tab ⇒ right, Shift variants reverse). Add `"celledit"` to `PreviewerEventName`.

A new option `PreviewerOptions.editable?: boolean` (default `false`) turns the behavior on.

---

## Backlog

Work top-to-bottom. Each item is independently shippable + committable. Run
`pnpm --filter @hewliyang/xlsx-preview check` (typecheck/lint/knip/schema/api) before
committing. No comments/docstrings in code (see AGENTS.md). Conventional commits. Add a
terse CHANGELOG.md entry per item.

### 1. feat(previewer): editable formula bar + `celledit` event

Library only. `src/previewer.ts`.

- Add `editable?: boolean` to `PreviewerOptions`; store `this.editable`.
- Add `"celledit"` to `PreviewerEventName`.
- When `editable`, make the built-in formula bar input writable:
  - drop `this.formulaBox.readOnly = true` (keep readOnly when not editable).
  - On focus, seed with the active cell's current formula/value (already done by
    `formatFormulaBar` in `draw()`; stop overwriting `formulaBox.value` while it is the
    active element / being edited).
  - On `Enter`: emit `celledit` with `{ sheetIndex, r, c, input: formulaBox.value, commitMove: "down" }`, blur.
  - On `Escape`: restore from `formatFormulaBar(...)`, blur.
- Emit via the existing `dispatchEvent(new CustomEvent("celledit", { detail }))` pattern;
  do **not** mutate the layout here — the host does that and calls `replaceLayout`.
- Guard `draw()` so it doesn't clobber `formulaBox.value` while the user is typing in it
  (`document.activeElement === this.formulaBox`).

Verify: a host listener receives `celledit` with the typed string when pressing Enter in
the formula bar. (Wire-up + e2e happens in item 3, but a unit-ish DOM check is fine.)

### 2. feat(previewer): inline cell editing overlay

Library. `src/previewer.ts` + `src/interact.ts`.

- Previewer creates a hidden overlay `<input>` once, appended to `this.stage`:
  - wrapper `<div>` `position:sticky;top:0;left:0;width:0;height:0;z-index:5;overflow:visible`
    (mirrors how `this.canvas` is sticky so it pins to the visible top-left of the scroll
    viewport). The inner `<input>` is `position:absolute` and moved with
    `transform: translate(x px, y px)`.
  - To position over the active cell, compute on-screen CSS px from the grid the same way
    `draw()`/render do:
    `buildGrid(sheet, colOverrides, rowOverrides)` → `cellRect(grid, r, c)` gives
    `{x,y,w,h}` in logical units, then
    `screenX = (rect.x - viewport.x) * zoom`, `screenY = (rect.y - viewport.y) * zoom`,
    width `rect.w * zoom`, height `rect.h * zoom`. Use `this.viewport` (already recomputed
    in `draw()`).
  - Before showing, call `scrollToCell(r, c)` so the cell is in the main viewport (MVP:
    skip frozen-pane-aware placement — documented limitation; frozen cells get scrolled
    into the unfrozen area first).
- Edit triggers (only when `editable`), routed from `interact.ts` via two new optional
  callbacks added to `InteractOptions`:
  - `onEditStart?(cell: { r; c }, initialText: string | null): void` — `initialText === null`
    means "open with current cell content" (F2 / double-click), a string means "replace
    with this typed character" (printable key).
  - In `interact.ts` `onKeyDown`: add `case "F2"` → `opts.onEditStart?.(cur, null)`; and a
    `default` branch that, for a single printable character (`ev.key.length === 1 && !ctrl/meta`),
    calls `opts.onEditStart?.(cur, ev.key)` and `ev.preventDefault()` (instead of the current
    bare `return`). Keep arrow/Tab/Enter navigation intact.
  - Add a `dblclick` listener on the canvas that hit-tests to a cell and calls
    `opts.onEditStart?.(cell, null)`. Reuse existing `hitTest`/cell resolution used by
    `onPointerDown` (the body cell branch ~line 575+).
- Previewer wires `onEditStart` in `attachInteractivity`: position + show the overlay input,
  set its value (`initialText ?? formatFormulaBar(...)`), focus, place caret at end.
- Commit/cancel on the overlay input:
  - `Enter` ⇒ emit `celledit` `{ ..., input, commitMove: shift ? "up" : "down" }`, hide.
  - `Tab` ⇒ emit with `commitMove: shift ? "left" : "right"`, hide, `preventDefault`.
  - `Escape` ⇒ hide without emit.
  - `blur` ⇒ commit with `commitMove: null` (or cancel — pick commit-on-blur to match Excel).
- Reposition/hide the overlay on scroll and zoom while open (simplest: hide on scroll/zoom
  to avoid drift), and hide it inside `replaceLayout`/`attachInteractivity`/`setActiveSheet`.

Verify: F2/double-click/typing opens an input over the active cell; Enter emits `celledit`
and advances down; Escape cancels.

### 3. feat(examples): wire cell editing + recalc modes

App only. `examples/xlsx-app.html`.

- Pass `editable: true` to `createWorkbookPreviewerFromFile`.
- Add an `applyEdit({ sheetIndex, r, c, input, commitMove })` helper:
  - bail if `recalcWorkbook == null`.
  - `addr = formatCell(r, c)`; `name = previewer.layout.sheets[sheetIndex].name`.
  - `const ws = recalcWorkbook.sheet(name);`
  - if `input.startsWith("=")` ⇒ `ws.range(addr).setFormula(input)`
    else ⇒ `ws.range(addr).setValue(coerceInput(input))` where `coerceInput` maps:
    `""`→`null`; `/^-?\d+(\.\d+)?$/`→Number; `true/false`→Boolean; else string.
  - if auto-recalc on ⇒ `recalcWorkbook.recalculate()`.
  - `previewer.replaceLayout(recalcWorkbook.layout())`.
  - advance active cell per `commitMove` via `previewer.selectCell(...)` (clamp ≥ 1), else
    keep current; then `renderState()` + `setStatus("edited " + addr)`.
- `previewer.on("celledit", (e) => applyEdit(e.detail))`.
- Recalc modes: add an "Auto recalc" checkbox (default on) in the Viewport section next to
  the existing Recalc button. When auto is on, the manual button is redundant but keep it.
  Add a global `F9` hotkey → manual recalc (same body as the recalc button click).
- The existing `#formula-box` is a `<span>`; leave it read-only (mirrors active cell). The
  editable formula bar from item 1 lives inside the previewer (currently hidden via
  `#preview .xlcore-formula-bar { display:none }`). For this example, **un-hide** the
  previewer formula bar (remove/relax that CSS rule) so item-1 editing is reachable, OR
  document that inline editing (item 2) is the primary path here. Prefer un-hiding so both
  paths are testable.

Verify e2e in the browser (see below).

---

## E2E test (after item 3)

1. `cd packages/xlsx-preview && PORT=8765 node scripts/preview.mjs` (serves repo root;
    app at `/packages/xlsx-preview/examples/xlsx-app.html`). Ensure
    `examples/recalc-demo.xlsx` exists (else `pnpm build:ts && node scripts/make-recalc-fixture.mjs`).
2. Browser harness: open the app, load `recalc-demo.xlsx`.
3. Double-click a cell (or press F2), type a value/formula, press Enter → cell updates and,
   for formulas, the computed value shows after recalc.
4. Toggle Auto recalc off, edit a `=RAND()`-dependent cell, confirm no recalc until F9 /
   Recalc button.
5. Screenshot each step to confirm.

## Known limitations (document, don't fix now)

- Frozen-pane-aware overlay placement (MVP scrolls cell into the main pane first).
- No undo/redo; mutations are in-memory (use `wb.save()` later for export/download).
- Full-layout re-decode per edit (`replaceLayout`) — fine for small/medium workbooks.
- Editing gated to `.xlsx`.
