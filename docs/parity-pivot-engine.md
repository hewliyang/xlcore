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

- One or more row fields, 0–1 column fields.
- Multiple data fields when there are **no** column fields (each data field
  becomes an extra value column with its own caption header).
- Multiple data fields **with** a single column field (the `-2` "values"
  marker axis): 3-row header, each column group expands into one sub-column
  per data field, grand-total group emits `Total <dataname>` per data field.
- Two column fields (nested headers) with a single data field.
- Unsupported shapes (>2 column fields, nested columns + multiple data
  fields, zero row fields) return no cells → renderer falls back to the previous empty-grid
  behavior; nothing breaks.

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

- [x] Bold the Grand Total row/column and the header rows.
- [x] Header fill (bold white on accent `4472C4`) to match the SpreadJS look.
- [x] Emit `style_index` on the engine cells (header / total-label /
      total-value roles). `register_styles` appends pivot fonts/fills/xfs to the
      workbook `Styles` once per extract (memoized via `pivot_style_memo` in
      `lib.rs`); ensures index-0 defaults exist first so authored workbooks with
      empty `cell_xfs` don't have their implicit-default cells styled.
- [ ] Label indentation for nested row fields (no-op for the current
      single-level tabular layout; revisit with outline/compact layouts).

### P1: Interactive live re-pivot (the big frontend win)

- [x] Expose `compute_cells` via WASM: `worksheet.pivots.preview(patch) →
      PivotGrid` (`pivotPreview` on the WASM handle / `pivot_preview` on
      `Workbook`) that aggregates **without** writing parts. Reuses
      `prepare_pivot` (validation + column build) + the existing
      `build_cache_definition`/`build_cache_records`/`build_pivot_definition`
      then runs `compute_cells` against a throwaway `Styles`. Number formats
      are skipped on this path. Units: `pivot_preview_aggregates_without_writing_parts`
      (rust), `smoke:api` (TS).
- [x] Grid DTO: `PivotGrid { rows, cols, cells: PivotGridCell[] }` with
      0-based `{ row, col, role, kind, value }`; `role` derived from the engine
      `style_index` (header / label / value / totalLabel / totalValue).
- [ ] Frontend: drag fields between row/column/filter/values, change
      aggregation, toggle filter items, recompute in-browser with no
      save/reopen round-trip.

### P1: Wider layout coverage

- [x] Multiple data fields in the no-column layout (extra value column +
      caption per data field).
- [x] Multiple data fields with a single column field (the `-2` "values"
      marker axis): expands each column group into one sub-column per data
      field with a 3-row header; verified vs SpreadJS. Unit:
      `computes_multiple_data_fields_with_column_field`.
- [x] More than one column field (nested column headers): two column fields
      with a single data field. Builds per-outer-group leaf columns + an
      `{outer} Total` subtotal column + grand-total column, with a 3-row header
      (caption/field-names, outer values + subtotal labels, inner values +
      row-field names). Verified vs SpreadJS. Unit:
      `computes_nested_column_fields`. >2 column fields, or 2 columns with
      multiple data fields, still fall back to empty.
- [ ] Multiple row fields beyond the current tabular side-by-side labels:
      outline/compact layouts and per-level subtotals.

### P2: Filtering + correctness

- [x] Honor page/filter field selection. Engine builds per-field hidden
      shared-item sets from `pivotFields/items[@h="1" @x=i]` and drops any
      record whose `FieldItem` value is hidden, before decode — so hidden items
      vanish from row/col keys *and* every total. Covers row/col filters and
      page fields expressed via hidden items (the common case). Unit:
      `hidden_items_excluded_from_keys_and_totals`.
- [x] `pageField/@item` single-select: resolves the selected `pivotField`
      item index → shared-item index and drops non-matching records before
      decode (combined with hidden-item filtering). Unit:
      `page_field_single_select_filters_records`.
- [ ] Honor stored `rowItems`/`colItems` order for imported pivots instead of
      recomputing from records (matters when the author set a manual sort or
      when not all field-item combinations appear in the data; Excel shows all
      items, we currently show only combinations present).
- [x] Data-field number formats: engine pipes the `dataField` `numFmtId` onto
      value + total cells (interned xfs, right-aligned); authoring accepts
      `numberFormat` on `PivotDataField` (resolved/interned into workbook
      styles), getter reverse-maps `numFmtId` → code. Verified vs SpreadJS.
- [x] Empty-records fallback: when `pivotCacheRecords` is empty, synthesize
      records from the `worksheetSource` range (same-document sheet lookup by
      name, resolves shared/inline strings, builds inline-literal records so no
      `sharedItems` needed). `synthesize_records` in `pivot_engine`'s sibling
      `pivots::extract`; engine path unchanged. Verified vs the populated cache:
      `tests/pivot_empty_records.rs` (fixture `pivot-empty-records.xlsx`).
      Note: hidden-item / page-field filtering by shared-item index is skipped
      on this path (records carry literal values, not `FieldItem` indices).

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
