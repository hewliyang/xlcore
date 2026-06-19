# Parity: editor interaction triage

Four interactive-editor gaps in the previewer. Each is a small, self-contained
change in `interact.ts` / `previewer.ts` (+ example app wiring where noted).
Pattern to follow: recent commits `462d05e` (Delete drawing), `4138...` fill,
and the `celledit`/`rangepaste` event plumbing in `previewer.ts` +
`examples/xlsx-app.html`.

Verify: `pnpm --filter @<pkg> check` and `pnpm --filter @<pkg> test` from
`packages/xlsx-preview`. (`pnpm test` runs typecheck-free vitest after build.)

## Backlog

### 2. Allow clearing focus (no active cell)
`activeCell`/`selection` setters in `attachInteractivity` opts
(`previewer.ts:~699,707`) ignore null, so focus can never be cleared. Add an
Escape branch in `interact.ts` `onKeyDown` (when not editing and no drawing
selected) that clears active cell + selection. Make the setters accept null,
and make `render`/`draw` + name box + formula bar tolerate a null active cell
(skip drawing the active-cell box; blank name/formula boxes). Scope: ~30 lines
across `interact.ts`, `previewer.ts`, `render.ts`.

### 3. Edit overlay should overflow the cell like Excel
`openEditOverlay` (`previewer.ts:~1259`) hard-sizes `editInput` to the cell
width, clipping long content. Make the input auto-grow horizontally on `input`
(white-space:nowrap; grow width to fit text, clamped to the visible stage
viewport), so a long value/formula overflows past the cell like Excel. Reset
size on `hideEditOverlay`. Scope: ~20 lines in `previewer.ts`.

### 4. Delete/Backspace clears the selected cell range
`interact.ts` `onKeyDown` only handles Delete/Backspace for a **selected
drawing**; a normal cell selection does nothing. Add a Delete/Backspace branch
(when `selectedDrawing` is null) that fires a new `onClear?(selection)` opt with
the current selection (fallback to active cell). Wire it in `previewer.ts` to
dispatch a `cellclear` CustomEvent `{ sheetIndex, ref }`, and in
`examples/xlsx-app.html` route `cellclear` -> `recalcWorkbook.clearRange(...)`
(already exists in `editWorker.ts:235`). Scope: ~10 lines interact + handler +
example wiring.

## Shipped

- Esc commits the cell edit (commitEdit(null)) instead of discarding.
