# Editing UX fixes — backlog

In-cell editing in `packages/xlsx-preview/src/previewer.ts` is finnicky. Three
fixes, all in the edit-overlay keyboard / point-mode path. Verified e2e via
`pnpm --filter @hewliyang/xlsx-preview preview` (loads `examples/recalc-demo.xlsx`,
`editable:true`, engine wired).

Shared facts (from triage):
- Edit overlay = `this.editInput`; opened by `openEditOverlay(cell, initialText)`.
  `initialText` is the typed char in "enter mode" (started by typing) and `null`
  for F2 / double-click ("edit mode").
- `onEditInputKeyDown` currently handles Enter/Tab/Escape only; arrows fall
  through to the input → caret trapped.
- `commitEdit(move)` already supports `"down"|"up"|"left"|"right"` and the example
  app moves the selection accordingly.
- Point mode: `isPointModeActive()` (caret after `=`,`(`,`,`,operator), `applyPointModeRef(ref,{extend})`
  inserts/replaces the A1 ref at the caret via `applyReferenceAtCaret` + `activeRefSpan`.
- Live highlight: `computeHighlights()` calls `engine.parseReferences(...)`. It
  returns `[]` when the formula can't parse (e.g. unbalanced `=SUM(A4:A10`), which
  is exactly the mid-point-drag state → no candidate highlight.
- Helpers already in file: `colLabel`, `colNameToIndex`, `clamp`, `buildGrid`.

## TODO

### 3. Keyboard-driven point-mode selection
While editing a formula at a ref-accepting caret, arrows should pick/move a
reference (and Shift+arrow extends), like the mouse point drag.
- Add `pointKeyAnchor`/`pointKeyCursor` state. On arrow in `onEditInputKeyDown`
  (and `onFormulaBoxKeyDown`) when `isPointModeActive()`: move a cursor from
  `editCell ?? activeCell` (clamped to grid bounds), Shift keeps the anchor to
  extend, otherwise collapse; build the A1 ref and call `applyPointModeRef`.
  `scrollToCell` to follow. Reset the anchor/cursor on type/commit/edit-start.
- Verify: edit a cell, type `=SUM(`, press Down a few times then Shift+Down —
  the ref text grows (e.g. `=SUM(E5:E8`) and the candidate box (item 2) tracks it.

## Shipped
- Arrow keys commit + navigate the selection when typing a value in enter mode.
- Candidate range box drawn during point-mode selection (before the formula parses).
