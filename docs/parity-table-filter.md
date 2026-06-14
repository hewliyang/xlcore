# Table / AutoFilter Engine Parity

Working checklist + handoff for **data table filtering & sorting**: turning the
already-authored `autoFilter` criteria into an actual visible-row view, and
wiring the interactive header dropdowns (mirror of the shipped pivot filter
popover).

This is the table analog of [`parity-pivot-engine.md`](parity-pivot-engine.md).
Same three-layer responsibility split:

| Layer | Pivot (shipped) | Table (this doc) |
|---|---|---|
| **Engine** | `compute_cells(def, cache)` → value grid | `apply_view(range, criteria, sortState)` → hidden-row set + row order |
| **Previewer** | paints arrows, renders dropdown, swaps layout | same, on the autofilter header buttons |
| **`.recalculate()`** | not involved | not involved (formula layer is separate) |

The filter **criteria + sortState live in the definition** (`autoFilter`
element), never in a cache. Unlike pivots there is **no separate cache** — the
table *is* the live worksheet cells. So the engine produces a *view* over those
cells, not a new materialized grid.

## Current State

### Done (authoring only — criteria are inert)

- `xlcore-api/src/auto_filter.rs`: `set_auto_filter` / `set_auto_filter_column`
  (values / top10 / custom) / `remove_auto_filter_column` write `filterColumn`
  criteria into the `autoFilter` element. TS surface in `api-collections.ts`
  (`worksheet.autoFilter.setColumnValues/Top10/Custom`).
- Export reads `auto_filter_range` (`schema.rs`) and the renderer **already
  draws** filter arrows on the autofilter header row and on `table.hasAutoFilter`
  tables (`sheetChrome.ts` `computeTableState`, `drawFilterArrows`).
- Renderer honors `row.hidden` (`grid.ts` sets height 0).

### The gap

1. **Criteria don't filter anything.** Nothing computes row visibility from
   `filterColumn`. The export only reads pre-baked `row.hidden="1"` flags. The
   moment a filter changes interactively those flags are stale.
2. **No sort.** `sortState` is neither authored nor applied.
3. **Arrows are inert.** They render but carry no column identity and
   `interact.ts` only hit-tests *pivot* arrows. No dropdown, no controller.

## Architecture

```
worksheet range (live cells)  ──┐
autoFilter { filterColumn[], sortState }  (definition)
                                ▼
   table_engine::apply_view  →  { hidden_rows: Set<u32>, row_order: Vec<u32> }
                                ▼
   authored path: write row.hidden flags + physically reorder data rows
                                ▼
   re-extract WorkbookLayout  →  renderer (unchanged; honors row.hidden + order)
```

- **Filtering is non-destructive** (a `row.hidden` mask, reversible).
- **Sort matches Excel: physically reorders the data rows** in the worksheet
  (destructive in the workbook model, which is what Excel persists). Re-extract
  → renderer renders in-place, no renderer change. (Non-destructive row-order
  *preview* is a P2 nicety.)
- The interactive path mirrors pivots exactly: controller returns a fresh
  `WorkbookLayout` (via `wb.layout()`), previewer swaps it with `replaceLayout()`.

## Action List

### P0: Engine + authored filtering

- [x] **`table_engine` module** (`crates/xlcore-export/src/table_engine/mod.rs`):
      pure `compute_hidden_rows(first_data_row, rows, &[(col_offset, &AutoFilterCriteria)])
      -> BTreeSet<u32>` (1-based) — Values+blank, Custom 1–2 (`=,<>,>,>=,<,<=`,
      numeric-or-string compare, `*`/`?` wildcards on `=`/`<>`), Top10
      (top/bottom, count/percent), multi-column AND. Unit tests per kind + AND.
- [x] **Authored filter application** in `xlcore-api/src/auto_filter.rs`:
      after `set_auto_filter_column` / `remove_auto_filter_column`, recompute
      the union of hidden rows across all `filterColumn`s and write
      `row.hidden` (set for filtered-out data rows, clear for visible ones;
      never touch the header row or rows outside the filter range). Round-trip
      test: author values filter → reopen → hidden rows match.
- [x] **Sort authoring + physical reorder**:
      `set_auto_filter_sort(sheet, column_offset, descending)` writes
      `sortState` AND reorders the data rows in place (stable sort, numbers
      numerically / text case-insensitive, blanks last). `remove_auto_filter_sort`
      removes it (leaves current order). TS: `autoFilter.setSort/clearSort`,
      wasm `setAutoFilterSort/removeAutoFilterSort`. Unit test covers reorder +
      state round-trip.

### P1: Interactive header dropdowns

- [x] **Arrow column identity** in export: emit sheet-level
      `TableFilterArrow { r, c, column_offset, column_name, range_ref }`
      (`schema.rs`, derived in `sheet.rs`/`tables.rs` from `auto_filter_range`
      + table autofilter). Regenerate TS schema
      (`cargo test --release -p xlcore-export export_bindings`). Keep the
      existing `drawFilterArrows`; this just adds the hit-test payload.
- [ ] **Hit-test in `interact.ts`**: mirror `pivotArrowAt` / `firePivotFilter`
      → `tableArrowAt` / `fireTableFilter`, emit `onTableFilter({ field,
      columnOffset, rangeRef, rect })`. Cursor pointer on hover. Unit:
      `sheetChrome.test.ts` arrow geometry/lookup for table arrows.
- [ ] **`tableFilterPopover.ts`** (mirror `pivotFilterPopover.ts`, vanilla DOM):
      header (column name) + **Sort A→Z / Sort Z→A** buttons + **value
      checklist** + "Clear filter". `TableFilterController { items(ctx),
      activeValues(ctx), sort(ctx), setFilter(ctx & {values}), setSort(ctx &
      {descending|null}) }` each returning `WorkbookLayout | void`. Wire into
      `previewer.ts` (mirror `pivotController`/`onPivotFilter`): swap returned
      layout via `replaceLayout()`, also emit a low-level `tablefilter` event.
- [ ] **Example wiring** in `examples/react-vite/src/App.tsx` +
      `examples/xlsx-app.html`: build a `TableFilterController` from
      `distinctValuesFor` (reuse `pivotSource.ts`) +
      `wb.autoFilter.setColumnValues` / new `setSort` + `wb.layout()`.

### P2: Polish (optional)

- [ ] Non-destructive sort: emit a `row_order` view in the layout so preview
      sorting doesn't mutate cells; only materialize on save/commit.
- [ ] `SUBTOTAL` / `AGGREGATE` honor the filtered-out rows during recalc
      (the filter engine feeds the formula engine a per-row hidden signal).
- [ ] "Select all" / search box in the value dropdown.

### Out of scope (preserve-only)

- Color filters, icon filters, dynamic (relative-date) filters, slicers.

## Key Files

- `crates/xlcore-export/src/table_engine/mod.rs` — **new** filter/sort engine.
- `crates/xlcore-api/src/auto_filter.rs` — authoring + apply hidden flags + sort.
- `crates/xlcore-export/src/{schema.rs,sheet.rs,tables.rs}` — `TableFilterArrow`.
- `packages/xlsx-preview/src/{interact.ts,sheetChrome.ts}` — hit-test + draw.
- `packages/xlsx-preview/src/tableFilterPopover.ts` — **new** dropdown.
- `packages/xlsx-preview/src/previewer.ts` — controller wiring.
- `packages/xlsx-preview/src/pivotSource.ts` — `distinctValuesFor` (reuse).
- `packages/xlsx-preview/examples/{react-vite,xlsx-app.html}` — host wiring.

## Testing

- Rust unit: `cargo test -p xlcore-export table` (filter semantics, sort),
  `cargo test -p xlcore-api auto_filter` (author → apply hidden → reopen).
- TS: `pnpm --filter @hewliyang/xlsx-preview check` + `sheetChrome.test.ts`.
- Visual parity loop (Excel-authored fixture with an autofilter):
  - `hsx --no-daemon screenshot <file> '<sheet>!A1:G20' -o /tmp/hsx.png`
  - `node packages/xlsx-preview/dist/cli.js <file> --sheet <sheet> -o /tmp/xp.png`
    (rebuild wasm first: `pnpm --filter @hewliyang/xlsx-preview build:wasm`).
- **E2E browser**: open the react-vite example, load a filtered workbook, click
  a header arrow → uncheck values → rows hide in place; Sort Z→A reorders.

## Known Risks / Gotchas

- Export cells are **1-based** (`parse_a1` → `(1,1)` for `A1`); keep the engine
  1-based to match `merge_pivot_cells` conventions. Don't mix with 0-based
  `ChartAnchor`.
- `auto_filter_range` includes the header row; only data rows (r1+1..=r2) are
  filtered/sorted.
- `root_element(&mut P)` mutably borrows the doc while `child_part(&P)` is
  immutable — clone decoded structs out before the next mutable read (same
  gotcha as `pivots::extract`).
- Tables can carry their own `<autoFilter>` inside the table part; a sheet can
  also have a bare `worksheet/autoFilter`. Handle both arrow sources.
- Physical sort reorder moves cells (incl. their style) but does **not**
  fix up formula references pointing into the sorted rows, nor merges that
  span data rows — out of scope; callers should sort value-only ranges.
</content>
</invoke>
