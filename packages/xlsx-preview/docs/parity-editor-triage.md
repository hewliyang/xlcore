# Parity: editor interaction triage

Four interactive-editor gaps in the previewer. Each is a small, self-contained
change in `interact.ts` / `previewer.ts` (+ example app wiring where noted).
Pattern to follow: recent commits `462d05e` (Delete drawing), `4138...` fill,
and the `celledit`/`rangepaste` event plumbing in `previewer.ts` +
`examples/xlsx-app.html`.

Verify: `pnpm --filter @<pkg> check` and `pnpm --filter @<pkg> test` from
`packages/xlsx-preview`. (`pnpm test` runs typecheck-free vitest after build.)

## Backlog

## Shipped

- Esc on the grid (not editing, no drawing selected) clears active cell + selection.

- Esc commits the cell edit (commitEdit(null)) instead of discarding.
- Delete/Backspace on a cell selection fires `cellclear` -> `clearRange`.
- Edit overlay auto-grows horizontally to overflow the cell like Excel.
