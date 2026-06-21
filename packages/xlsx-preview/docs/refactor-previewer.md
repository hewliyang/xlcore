# Refactor: split previewer.ts

`src/previewer.ts` is 1808 LoC and fails `check:loc` (limit 1500). Goal: extract
cohesive concerns into sibling modules, no behavioral change, all checks green.
Target end state: previewer.ts ~500-600 LoC (shell + public API + interactivity
wiring only).

Verify per item: `pnpm run typecheck && pnpm run lint && pnpm run knip &&
pnpm run check:loc` plus `pnpm test` (vitest). Each item ships its own commit.
New pure modules should get a `.test.ts`.

## Backlog

3. Extract `formulaText.ts` (+ test): `balanceFormula`, `formatFormulaBar`.

4. Extract `previewerChrome.ts`: `makeButton`, `makeTab`, `contrastingTextColor`,
   `virtualSize`, `normalizeSelection` (DOM/geometry builders).

5. Extract `autocompletePopover.ts` (+ test): own the `autocompleteMenu` element
   and state; methods update/render/close/scheduleClose/isOpen/accept/handleKey.
   Host keeps a field and delegates. Model after pivotFilterPopover.ts.

6. Extract `signatureTip.ts`: own `signatureTip` element; update/render/hide/
   scheduleClose.

7. Extract `validationDropdown.ts` (+ test): own `validationMenu` element + filter
   state; update/render/close/isOpen/accept/handleKey + validationListFor.

8. Extract `cellEditor.ts` (+ test): the edit overlay + point mode. Owns
   `editInput` and all edit/point state; methods openEditOverlay, growEditInput,
   hideEditOverlay, commitEdit, onEditInputKeyDown, armPointMode, isPointModeActive,
   applyPointModeRef, resetPointSpanOnType, movePointKeyboard, handlePointKeyboardKey.
   Composes the three widgets from items 5-7. Host wires callbacks (scrollToCell,
   getActiveSheet, emit celledit, scheduleDraw).

## Shipped

1. Extracted pure ref/location helpers into `previewerRefs.ts` (+ test); dropped local `colLabel`.
2. Added shared `mathUtils.ts` (`clamp`, + test); replaced duplicated local `clamp` in previewer/interact/highlights/selection.
