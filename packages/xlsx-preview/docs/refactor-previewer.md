# Refactor: split previewer.ts

`src/previewer.ts` is 1808 LoC and fails `check:loc` (limit 1500). Goal: extract
cohesive concerns into sibling modules, no behavioral change, all checks green.
Target end state: previewer.ts ~500-600 LoC (shell + public API + interactivity
wiring only).

Verify per item: `pnpm run typecheck && pnpm run lint && pnpm run knip &&
pnpm run check:loc` plus `pnpm test` (vitest). Each item ships its own commit.
New pure modules should get a `.test.ts`.

## Backlog

(empty)

## Shipped

8. Extracted `cellEditor.ts` (+ test): `CellEditor` owns the edit overlay `editInput` + all edit/point-mode state and methods; previewer delegates via `this.editor` through a `CellEditorHost` interface; composes the autocomplete/signature/validation widgets.

7. Extracted `validationDropdown.ts` (+ test): `createValidationDropdown` owns the menu element + options/filter state; previewer delegates via `this.validation` (open/refresh/handleKey/reset).

6. Extracted `signatureTip.ts` (+ test): `createSignatureTip` owns the tooltip element + blur timer; previewer delegates via `this.signature` with `isBlocked` = autocomplete open.

1. Extracted pure ref/location helpers into `previewerRefs.ts` (+ test); dropped local `colLabel`.
2. Added shared `mathUtils.ts` (`clamp`, + test); replaced duplicated local `clamp` in previewer/interact/highlights/selection.
3. Extracted `formulaText.ts` (+ test): `balanceFormula`, `formatFormulaBar`.
5. Extracted `autocompletePopover.ts` (+ test): `createAutocompletePopover` owns the menu element + state; previewer delegates via `this.autocomplete`.
4. Extracted `previewerChrome.ts` (+ test): `makeButton`/`makeTab`/`contrastingTextColor`/`virtualSize`/`normalizeSelection` + virtual extra const.
