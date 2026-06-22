# Backlog: filter-controller ergonomics

Goal: make table/pivot filter dropdowns "just work" when editing is on, by
exposing memoized lazy controller getters on `WorkerWorkbook` (mirroring the
existing `get engine()`), surfacing a discoverable warning when a filter is
clicked but no controller is wired, and deleting the copy-paste controller
builders from the example page.

Files of interest:
- `packages/xlsx-preview/src/worker.ts` — `WorkerWorkbook`, has `get engine()`,
  `distinctValues`, `tableSetFilter`, `tableSetSort`, `pivotMetas`,
  `updatePivot`.
- `packages/xlsx-preview/src/previewer.ts` — `onTableFilter`/`onPivotFilter`
  silently no-op when controller absent.
- `packages/xlsx-preview/src/tableFilterPopover.ts` /
  `pivotFilterPopover.ts` — `TableFilterController` / `PivotFilterController`
  interfaces.
- `packages/xlsx-preview/examples/xlsx-app.html` — `buildPivotController` /
  `buildTableController` to delete; usage at load.

Verify: `pnpm --filter @hewliyang/xlsx-preview check` and `... test`. E2e: load
an xlsx with a table + pivot via the example app / xlsx-preview cli and confirm
filter dropdowns open and apply.

## TODO

### ~~T1 — Memoized lazy controllers on WorkerWorkbook~~ (moved to Shipped)
Add `get tableController(): TableFilterController` and
`get pivotController(): PivotFilterController` to `WorkerWorkbook`, memoized on
the instance (instantiate once, cache — NOT fresh-per-call like `engine`,
because they hold mutable `kept`/`hidden` state). Port the logic from
`buildTableController`/`buildPivotController` in `xlsx-app.html`:
- table: `kept` Map keyed by columnOffset; `items`/`activeValues` via
  `distinctValues`; `setFilter` updates `kept` (delete when all/none selected)
  then `tableSetFilter`; `setSort` via `tableSetSort`.
- pivot: drop the async-precompute/null pattern. Resolve `pivotMetas()` lazily
  inside `items`/`setHidden` (cache the name→meta map after first fetch). Always
  return a controller (no-pivot case handled by the filter button never
  rendering). `hidden` Map keyed by `pivot\0field`; `setHidden` recomputes
  hiddenItems for the pivot then `updatePivot`.
Reset/clear the memoized controllers + their caches in any place the shadow is
re-synced if filter state would otherwise go stale (check `syncShadow`); if not
needed, leave a note. Keep within LOC budget.

### T2 — Discoverable warning in previewer
In `previewer.ts`, when a `tablefilter`/`pivotfilter` interaction fires while
`editable` is true but the matching controller is absent, `console.warn` ONCE
(guard with a per-instance flag per kind) telling the dev to pass
`recalcWorkbook.tableController` / `.pivotController`. No backend coupling — it's
purely "I have no controller". Don't warn when not editable.

### T3 — Delete copy-paste from example page
Remove `buildPivotController`/`buildTableController` from `xlsx-app.html` and use
`recalcWorkbook.tableController` / `recalcWorkbook.pivotController` directly at
the `createWorkbookPreviewerFromFile` call (guard for `recalcWorkbook` null like
`engine` already is). Confirm any docs/README snippet showing the old builders is
updated.

## Shipped

- T1: memoized lazy `get tableController` / `get pivotController` on `WorkerWorkbook`; pivot meta cache reset in `syncShadow` (kept/hidden state preserved across resync).
