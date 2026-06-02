# Pivot Engine Parity

Working checklist + handoff for pivot tables: authoring the OOXML parts and
the self-contained aggregation engine that materializes the value grid for the
`xlsx-preview` renderer.

This is **independent of `parity-engine.md`** (the IronCalc formula recalc
layer). A pivot is a group-by + aggregate over `pivotCacheRecords`, which Excel
stores fully materialized, so none of the formula engine is required.

## Current State

### Done

Authoring (`crates/xlcore-api/src/pivots.rs`):

- `set_pivot` / `pivots` / `remove_pivot` on `Workbook`; DTOs in
  `xlcore-types` (`PivotPatch`, `PivotInfo`, `PivotDataField`,
  `PivotAggregation`), `InvalidPivot` error code.
- WASM (`setPivot`/`pivots`/`removePivot`) + TS (`worksheet.pivots`,
  `workbook.allPivots`) wrappers.
- Writes three parts (`pivotCacheDefinition`, `pivotCacheRecords`,
  `pivotTable`) + the workbook `<pivotCaches>` edit, mutually consistent:
  enumerated `sharedItems` for every field, index-based records,
  materialized `rowItems`/`colItems`.
- Field XML matches Excel/ECMA-376 (verified via `./ecma-376` search CLI):
  keep the `t="default"` total item with `defaultSubtotal` on (not the
  contradictory `defaultSubtotal="0"`), data field `axis="axisValues"`.
  Opens in Excel without the repair dialog.

Aggregation/render engine (`crates/xlcore-export/src/pivot_engine.rs`):

- Decodes cache definition + records into typed values (resolves
  `<x v=i>` against `sharedItems`; handles literal Number/String/Boolean
  records and `MissingItem`).
- Group-by over row/column field tuples; folds each bucket with all 11
  aggregations (sum/count/avg/max/min/product/countNums/stdDev/stdDevP/var/varP)
  plus row, column, and grand totals.
- Excel-style axis sort (numbers numerically, text case-insensitive
  alphabetical).
- Materializes the computed grid into sheet cells (1-based, matching the rest
  of the export) via `merge_pivot_cells` in `lib.rs`, before columnar packing.
  Renderer needs zero changes.
- Verified value + label parity against SpreadJS (`hsx`) for both an authored
  workbook and the imported fixture `tests/fixtures/pivot/pivot-simple.xlsx`.

### Supported shapes (engine)

- One or more row fields, 0–1 column fields, exactly one data field.
- Unsupported shapes (multi data field, >1 column field, zero row fields)
  return no cells → renderer falls back to the previous empty-grid behavior;
  nothing breaks.

## Architecture

```
xlsx → xlcore-io (SpreadsheetDocument)
     → xlcore-export::pivots::extract
         ├─ Pivot metadata { name, range, filter_arrow_cells }
         └─ pivot_engine::compute_cells(definition, cacheDef, records) → Vec<Cell>
     → merge_pivot_cells(sheet, cells)        # lib.rs, pre-compactify
     → columnar::compactify                   # packs rows → ColumnarCells blob
     → WorkbookLayout (JSON via WASM extract_xlsx)
     → render.ts / cli.js                     # draws cells, unchanged
```

The same group-by logic conceptually backs the author path (which materializes
`rowItems`/`colItems` for Excel/SpreadJS) and the export path (which
materializes actual cell values for our renderer).

## Action List

### P0: Styling parity

- [ ] Bold the Grand Total row/column and the header rows.
- [ ] Header fill + label indentation to match the SpreadJS look.
- [ ] Extend the `Pivot` schema struct (or emit `style_index` on the cells)
      with cell roles (header / row-label / value / total) so the renderer can
      style them. Currently cells carry no `style_index`.

### P1: Interactive live re-pivot (the big frontend win)

- [ ] Expose `compute_cells` (or a thin grid form) via WASM:
      `workbook.pivotPreview(patch) → PivotGrid` that aggregates **without**
      writing parts.
- [ ] Frontend: drag fields between row/column/filter/values, change
      aggregation, toggle filter items, recompute in-browser with no
      save/reopen round-trip.
- [ ] Decide grid DTO shape (header cells + row labels + value matrix) vs.
      reusing `Cell[]`.

### P1: Wider layout coverage

- [ ] Multiple data fields (the `-2` "values" marker axis); expand along
      columns under each column group, with the data-field caption row.
- [ ] More than one column field (nested column headers).
- [ ] Multiple row fields beyond the current tabular side-by-side labels:
      outline/compact layouts and per-level subtotals.

### P2: Filtering + correctness

- [ ] Honor page/filter field selection (currently all items always included).
- [ ] Honor stored `rowItems`/`colItems` order for imported pivots instead of
      recomputing from records (matters when the author set a manual sort or
      when not all field-item combinations appear in the data; Excel shows all
      items, we currently show only combinations present).
- [ ] Data-field number formats (pipe `numFmt` onto the value cells / the
      `dataField`) so currency/percent render correctly.
- [ ] Empty-records fallback: aggregate from the `worksheetSource` range when
      `pivotCacheRecords` is empty + `refreshOnLoad="1"`.

### P2: Author/export robustness

- [ ] Relocate cache parts from `/pivotCache` to the conventional
      `/xl/pivotCache` (ooxmlsdk `PATH_PREFIX = "../pivotCache"`); valid today
      via absolute rels and Excel-clean, but non-standard.
- [ ] `update()` for pivots (mirror the chart remove+re-set merge pattern).
- [ ] Author-time cell write: optionally write the computed grid into real
      worksheet `<c>` cells on save so non-SpreadJS tools see values too.

### Out of scope (preserve-only)

- Slicers, timelines, calculated fields, date/numeric grouping.

## Key Files

- `crates/xlcore-api/src/pivots.rs` — authoring (parts + workbook edit).
- `crates/xlcore-types/src/lib.rs` — pivot DTOs + `InvalidPivot`.
- `crates/xlcore-export/src/pivot_engine.rs` — decode + aggregate + layout.
- `crates/xlcore-export/src/pivots.rs` — reads parts, calls the engine.
- `crates/xlcore-export/src/lib.rs` — `merge_pivot_cells`, pipeline wiring.
- `crates/xlcore-wasm/src/lib.rs` — WASM bindings.
- `packages/xlsx-preview/src/api-collections.ts`,
  `api-worksheet.ts`, `api.ts` — TS surface.
- `tests/fixtures/pivot/pivot-simple.xlsx` — Excel-authored reference fixture.

## Testing

- Rust unit: `cargo test -p xlcore-export pivot` (aggregation math, sort,
  full two-axis grid with totals) and `cargo test -p xlcore-api pivot`
  (author → save → reopen round-trip, validation errors).
- TS smoke: `pnpm --filter @hewliyang/xlsx-preview smoke:api` (author + reopen).
- Visual parity loop:
  - `hsx --no-daemon screenshot <file> '<sheet>!A1:G10' -o /tmp/hsx.png`
    (SpreadJS reference)
  - `node packages/xlsx-preview/dist/cli.js <file> --sheet <sheet> -o /tmp/xp.png`
    (our renderer; rebuild wasm first: `pnpm --filter @hewliyang/xlsx-preview build:wasm`
    then copy `pkg/xlcore_wasm.js` + `.wasm` into `src/`).
- Spec check: `./ecma-376/ecma search "<term>"` / `ecma show <id>` to verify
  pivot XML against ECMA-376 Part 1 (e.g. `ST_ItemType`, `ST_Axis`,
  `pivotField`).

## Known Risks / Gotchas

- Export cells are **1-based** (`xlcore_io::parse_a1` returns `(1,1)` for `A1`);
  the engine and `max_row`/`max_col` follow that. Don't mix with 0-based
  conventions used elsewhere (e.g. `ChartAnchor`).
- The `/pivotCache` part location is non-standard but valid; SpreadJS and
  Excel both accept it. `refreshOnLoad="1"` means Excel rebuilds the cache on
  open regardless.
- When wiring more part reads in `pivots::extract`, note `root_element(&mut P)`
  mutably borrows the doc while `child_part(&P)` is immutable — clone the
  decoded structs out before the next mutable read (see the existing
  `cache` closure).
