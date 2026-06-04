# Workbook API Parity Plan

This tracks the workbook manipulation API we want for Rust, WASM, and TypeScript.
SpreadJS is the breadth reference; EPPlus is the object-model sanity check; xlcore
owns the implementation and OOXML preservation semantics.

Source references:

- SpreadJS declaration dump: `gc.spread.sheets.d.ts`
- Current Rust facade: `crates/xlcore-api`
- Shared DTOs and TS bindings: `crates/xlcore-types` -> `packages/xlsx-preview/src/api-schema`
- WASM wrapper: `crates/xlcore-wasm::WorkbookHandle`
- TS wrapper: `packages/xlsx-preview/src/api.ts`

## Principles

- Rust is the source of truth. WASM and TypeScript wrap the same mutation path.
- Public references use A1 strings. Internal indexes can remain OOXML/native.
- Preserve unrelated OOXML parts by default: charts, drawings, styles, tables,
  comments, relationships, extensions, workbook metadata.
- Recalc stays explicit. Mutations mark caches stale; callers choose when to
  recalculate and whether save should write calculated values.
- API errors are structured and stable enough for agents to recover from.
- Do not clone SpreadJS UI. We only mirror workbook manipulation concepts that
  matter for creating, editing, testing, rendering, and saving files.

## Current Surface

Implemented:

- `Workbook::new/open_bytes/open_path/save_bytes/save_path/load_report`
- Sheet list/create/rename/delete
- A1 cell refs with quoted sheet names and absolute markers
- Cell get/set value/set formula/clear
- Scalar values: blank, string, number, boolean, error
- Formula writes preserve source text and mark caches stale
- Recalc/writeback through `xlcore-bridge`
- Layout extraction through `xlcore-export`
- WASM `WorkbookHandle` and TS `Workbook` wrapper
- Shared Rust/TS DTOs through `xlcore-types`
- API smoke test: create -> mutate -> recalc -> layout -> save -> reopen

Known gaps:

- Style write surface limited to font/fill/border/alignment/number format
  patch; themes, named styles, and table-style authoring still preserve-only
- Chart authoring (column/bar/line/pie/area/scatter/bubble/doughnut with
  title/legend/categories ref + series, per-series solid color, axis titles,
  stacking for bar/column/line/area) is wired through `set_chart` / `charts`
  / `remove_chart`. Authored charts open clean in Excel desktop (no repair
  dialog) — fresh `drawing<n>.xml` and `chart<n>.xml` parts emit the
  `<?xml ... standalone="yes"?>` header plus xdr/a/r/c xmlns declarations
  on the root. Charts use the legacy `c:chartSpace` schema only — no
  companion `chartStyle`/`colorStyle` parts — so Excel falls back to its
  Excel-2007-era painter (flat fills, default accent palette, basic axis
  fonts). SpreadJS applies its own theme so authored charts render more
  modernly there. Chart-level data labels now author/read for all chart
  kinds (show value / category / series / percent / legend-key, position,
  separator). Per-series data labels also author/round-trip via
  `ChartSeriesPatch.data_labels` (overrides chart-level when set; chart-level
  dl no longer duplicates onto every series). Richer chart features (combo,
  dual axis, chartEx, marker/line styling, per-point data labels, modern
  chartStyle/colorStyle companion parts) remain preserve-only. Image authoring now covers
  position, rotation, flips, and crop. Shape authoring (`shapes` / `set_shape`
  / `remove_shape`) creates DrawingML preset shapes via a two-cell anchor: any
  of the 187 `prstGeom` presets (validated through `ShapeTypeValues::from_str`),
  solid fill, solid outline (color + width), multiline text (color/size/bold/
  italic/underline + horizontal `align` and vertical `verticalAlign`), line
  arrowheads (`headEnd`/`tailEnd` `{type,w,len}`), rotation, and flips. Rotated/
  flipped shapes now emit `a:off`/`a:ext` derived from the anchor grid geometry,
  so `rot` is honored by both Excel and the preview. v0 carve-outs: gradient/
  pattern/blip fills, effects/shadows, per-run rich text, groups, connectors with
  connection sites, and custom geometry stay preserve-only. Pivot tables now author a single
  worksheet-source pivot (`set_pivot` / `pivots` / `remove_pivot`): row/column/
  filter fields plus sum/count/avg/max/min/product/countNums/stdDev/var data
  fields. The cache enumerates shared items for every field and records use
  index references; rowItems/colItems are materialized (subtotals disabled) so
  Excel and SpreadJS compute the value grid from the cache. The
  `pivotCacheDefinition`/`pivotCacheRecords` parts land at `/pivotCache`
  (SDK `PATH_PREFIX` relative to `xl/workbook.xml`) and are wired with absolute
  relationship targets; the `pivotTable` part gets its own cacheDefinition
  relationship via `create_relationship_to_part`. On extract, a self-contained
  aggregation engine (`xlcore-export/src/pivot_engine.rs`) groups
  `pivotCacheRecords` and folds each bucket (sum/count/avg/max/min/product/
  countNums/stdDev/var) with row/column/grand totals, then materializes the
  computed value grid into the sheet cells so the preview renderer shows pivot
  values without Excel/SpreadJS. This is independent of the formula recalc
  engine; it covers one+ row fields, 0-1 column fields, and one data field.
  Slicers/timelines, calculated fields, grouping, multi-level subtotals, and
  multi data field / multi column field layouts remain out of scope.
  Sparkline
  groups now author/list/remove (line/column/win-loss stacked, per-cell
  location + dataRef, markers/high/low/first/last/negative/displayXAxis
  flags, per-color palette, axis types + manual min/max, line weight).
  Sparklines live in worksheet `extLst` under the
  `{05C60535-1F16-4fd2-B633-F4F36F0B64E0}` x14 extension; round-trips
  through hsx/Excel cleanly. Conditional formatting now authors classic rules
  plus color scales / data bars / icon sets; `<extLst>` x14 extensions
  (custom icons, multi-color data bars, etc.) remain preserve-only.
- Threaded notes (modern `<threadedComment>`) author/list/reply/remove
  shipped through `xlcore-api`, with legacy `tc=<guid>`-authored classic
  comment shadow now mirrored into `xl/comments<n>.xml` so older Excel
  viewers see the thread. VML drawing indicators are synthesized on
  comment authoring when no `legacyDrawing` already exists (preserves
  user-supplied VML on round-trip).
- Batch returns a diagnostic envelope (`BatchOutcome { value, warnings, error }`)
  and a workbook-level warnings buffer is exposed via `warnings()` / `take_warnings()`
  (Rust, WASM, TS). Real warning emitters (lossy normalizations, unsupported
  formula/object surfaces) are added per-feature as needed; rollback/transactional
  semantics are still out of scope.
- Defined names round-trip, but engine-side resolution of structured/table
  refs and most modern function names is still missing (see
  `docs/parity-engine.md`)

## Done: ooxmlsdk 0.7.0 Migration

Upgraded to ooxmlsdk 0.7.0. Breaking schema rename pass touched
`xlcore-io`, `xlcore-export`, `xlcore-bridge`, `xlcore-api`, and
`xlcore-tabular`. Key structural shifts handled:

- Variant/field prefixes (`X` / `Xdr` / `C` / `Cx` / `A` and matching
  `x_` / `xdr_` / `c_` / `cx_` / `a_`) dropped across the schema.
- `Font` / `RunProperties` are now choice particles
  (`Vec<FontChoice>` / `Vec<RunPropertiesChoice>`). Added
  `xlcore-export/src/font_flat.rs` for read paths and a retain/push
  pattern for write paths (`xlcore-api/src/styles.rs`).
- `BooleanValue` is an enum (no longer `bool` alias); converted with
  `bool::from` / `BooleanValue::from_bool`.
- `CoordinateValue` / `Coordinate32Value` / `DrawingmlPercentageValue` /
  `TextBulletSizeValue` are wrapper enums; converted via `to_emu()` /
  `as_drawingml_percent()`.
- `<x:t>`, `<x:v>`, `<x:author>`, `<x:formula>`, `<x:totalsRowFormula>`,
  header/footer text nodes are now tuple structs wrapping `XstringType`
  / `TableFormulaType`.
- `Worksheet.sequence_of_references` / `DataValidation.sequence_of_references`
  are now `Vec<String>` / `Option<Vec<String>>` directly (list-value
  wrapper gone).
- `GraphicData.xml_children` is now
  `graphic_data_choice: Vec<GraphicDataChoice>`; chart-frame relationship
  lookup walks `GraphicDataChoice::ChartReference`.

Leveraged the upgrade:

- Lifted the blank-only restriction on `AutoFilterCriteria::Values` and
  emit one `<x:filter/>` per value
  (`crates/xlcore-api/src/auto_filter.rs`).
- Added `auto_filter_column_values_multi_value_round_trip` regression
  test.

1184 tests passing across the workspace.

## API Parity Table

Status key:

- Done: implemented and tested through at least Rust or TS smoke coverage
- Partial: some support exists, but not enough for the target surface
- P0: next workbook editing layer
- P1: agent workflow layer
- P2: report-generation/object-model breadth
- Later: explicitly out of the current hillclimb

| Area | SpreadJS reference | xlcore target | Status | Test approach |
| --- | --- | --- | --- | --- |
| Open/create/save | `Workbook`, JSON/file flows | Open bytes/path, create blank workbook, save bytes/path, preserve unrelated OOXML | Done | Rust API + save/reopen |
| Shared DTOs | SpreadJS `.d.ts` surface | Rust DTOs generated to TS from `xlcore-types` | Done | `scripts/regen-api-schema.sh` |
| Workbook metadata | `Workbook.options`, `docProps`, workbook name | Read/write core properties, active sheet, calc properties | Done | Rust API + save/reopen |
| Sheet collection | `Workbook.getSheet/addSheet/removeSheet/setSheet` | List/create/rename/delete/move/hide/show/active sheet | Done | Rust API + TS smoke |
| Cell scalar IO | `Worksheet.getValue/setValue`, `Range.value` | Get/set scalar values and errors by A1 ref | Done | Rust API + TS smoke |
| Cell formulas | `getFormula/setFormula`, calc APIs | Set formula text, preserve formula, explicit recalc/writeback | Partial | xlcore-engine + Excel fixtures |
| Range values | `Worksheet.getArray/setArray`, `Range.value` | Get/set rectangular matrices with shape validation | Done | Rust API + TS smoke |
| Range formulas | `Range.formula`, copy/fill APIs | Set formula matrices, copy formulas with relative refs | Done | Rust API + TS smoke |
| Clear modes | `clear`, `ClearPendingChangeType` | Clear values, formulas, styles, comments, or all | Done (values/formulas/styles/all; comments later) | Rust API + TS smoke |
| Style subset | `Style`, `Range.backColor/font/border/formatter` | Number format, font, fill, alignment, wrap, simple borders | Done | Rust API + layout |
| Full styles | Themes, named styles, table styles | Preserve existing styles; author broader style model later | P2 | OOXML diff + preview |
| Row/column size | `setRowHeight/setColumnWidth`, auto fit | Resize rows/columns; optional auto-fit later | Done | Rust API + save/reopen |
| Row/column visibility | `setRowVisible/setColumnVisible` | Hide/show rows and columns | Done | Rust API + save/reopen |
| Insert/delete rows/cols | `addRows/deleteRows/addColumns/deleteColumns` | Structural edits with cell/formula/reference movement | Done | Rust API + save/reopen |
| Freeze panes | `frozenRowCount/frozenColumnCount` | Freeze/unfreeze panes | Done | Rust API + save/reopen |
| Merges | `addSpan/removeSpan/getSpans` | Merge/unmerge/list merged ranges | Done | Rust API + TS smoke |
| Selections/view state | `setSelection/getSelections/showCell` | Persist active cell/selection only if OOXML-backed and useful | Later | OOXML inspection |
| Search | `Worksheet.search` | Search values/formulas across sheets | Done | Rust API + TS smoke |
| Copy/paste/fill | `copyTo`, fill APIs | Copy ranges, fill down/right, translate relative formulas | Done | Rust API + TS smoke |
| Dependencies | Calc engine refs/deps behavior | Precedents/dependents from formula graph | Done | Rust API + TS smoke |
| Defined names | `Workbook.names`, `NameInfo` | List/create/update/delete workbook and sheet names | Done (engine resolution still missing) | Rust API + TS smoke |
| Comments/notes | `Comments.CommentManager` | Add/edit/delete/list comments and threaded notes when present | Done (classic comments + threaded notes add/reply/list/remove + `tc=<guid>` classic shadow for legacy Excel + synthesized VML legacy drawing indicators on fresh files) | Rust API + save/reopen |
| Hyperlinks | Worksheet hyperlink APIs | Add/edit/delete/list cell hyperlinks | Done | Rust API + save/reopen |
| Tables | `Tables.TableManager`, `Table` | Create table from range, headers/totals, resize, style name | Done | Rust API + save/reopen |
| AutoFilter | Table/filter APIs | Preserve filters first; author simple filters later | Done (range + per-column Top10/Custom/multi-value Values with optional blank) | Rust API + smoke |
| Data validation | `DataValidation` APIs | Add/edit/delete/list list and scalar validations | Done | Rust API + save/reopen |
| Conditional formatting | `ConditionalFormatting` APIs | Preserve existing; author basic rules once style writes exist | Done (cellIs/expression/text/blanks/errors/top10/aboveAverage/duplicate/unique rules + dxf font/fill/border/numFmt/alignment authoring; 2/3-stop color scales, min/max data bars, 3/4/5-icon icon sets with cfvo round-trip; extLst x14 extensions still preserve-only) | Rust API + save/reopen |
| Charts | `Charts`, `Shapes` | Preserve; create/edit chart types already rendered | Done (authoring column/bar/line/pie/area/scatter/bubble/doughnut with title/legend/categories ref/series literal-or-ref names + values_ref; scatter/bubble take xValuesRef + yVal; bubble takes bubbleSizesRef; per-series solid `color` (`RRGGBB` or `AARRGGBB`, alpha stripped) authored + read for all kinds incl. bar/column/line; `categoryAxisTitle`/`valueAxisTitle`; chart-level `dataLabels` with show flags, position, separator; per-series `dataLabels` override; preserves richer existing chart XML) | Rust API + save/reopen |
| Images | `Shapes.Picture`, shape collection | Insert image, position, size, crop/rotation later | Done (insert/list/remove PNG/JPEG/GIF/BMP/TIFF/WEBP/SVG with two-cell anchor; sniff or explicit `format`; crop/rotation still preserve-only) | Rust API + save/reopen |
| Shapes | `Shapes.ShapeCollection` | Preserve first; author basic shape/text later | Done (insert/list/remove preset shapes via two-cell anchor: any of 187 `prstGeom` presets, solid fill, solid outline color+width, multiline text with color/size/bold/italic/underline + horizontal/vertical alignment, line arrowheads, rotation, flips; rotation/flip emit anchor-derived `a:off`/`a:ext` so `rot` renders in Excel + preview; gradient/pattern/blip fills, effects, rich-text runs, groups, connectors, custom geometry preserve-only) | Rust API + save/reopen + preview-render |
| Sparklines | `Sparklines` APIs | Preserve; author simple line/column/win-loss | Done (author/list/remove line/column/stacked groups with per-entry location + dataRef, markers/high/low/first/last/negative/displayXAxis flags, axis min/max kinds + manual values, line weight, full color palette) | Rust API + save/reopen |
| Pivot tables | `PivotTableManager`, slicers/timelines | Preserve; refresh/create only after aggregation model exists | Partial (author single worksheet-source pivot: row/column/filter fields + sum/count/avg/max/min/product/countNums/stdDev/var data fields; enumerated cache + materialized rowItems/colItems so Excel/SpreadJS compute values; subtotals disabled, single-level fine, no slicers/timelines/calculated fields/grouping; cache part emits at `/pivotCache` via absolute rels; layout extractor aggregates `pivotCacheRecords` and materializes the value grid so the preview renderer shows values without SpreadJS/Excel) | Rust API + SpreadJS load/compute + save/reopen + preview-render value parity |
| Slicers/timelines | `Slicers`, pivot slicers | Preserve; authoring deferred | Later | OOXML diff |
| Protection | Sheet/workbook protection APIs | Sheet/workbook protection metadata | Done | Rust API + save/reopen |
| Print/page setup | Print, page setup, headers/footers | Page setup, print areas, headers/footers | Done (orientation/scale/fit/margins/print options/header+footer; print areas via defined names) | Rust API + save/reopen |
| JSON import/export | `toJSON/fromJSON` | Optional xlcore JSON for app state, not SpreadJS-compatible by default | Later | Round-trip tests |
| Undo/redo/commands | Command manager, undo manager | Optional command journal/replay log | Later | Unit tests |
| Events/UI options | Events, context menu, scrollbars, hit testing | Out of scope unless needed by previewer harness | Later | Browser harness only |
| Diagnostics | SpreadJS often throws or mutates silently | Stable `ApiError`, `ApiWarning`, `BatchOutcome` envelope with workbook warnings buffer | Done (envelope + buffer; real emitters added per feature) | Rust unit tests + TS surface |
| Browser harness | SpreadJS UI runtime | Minimal local page to mutate/recalc/render via TS wrapper | Done (`examples/xlsx-playground.html`, site `/playground`) | Playwright/screenshot |

## Target TypeScript Shape

Keep the public wrapper small and explicit. Add fluent helpers only after the
Rust operations exist.

~~~ts
const workbook = await Workbook.open(bytes);

workbook.createSheet("Inputs");
workbook.setValue("Inputs!A1", "Units");
workbook.setValue("Inputs!B1", 100);
workbook.setFormula("Inputs!C1", "=B1*1.08");

workbook.setRangeValues("Inputs!A2:B4", [
  ["North", 10],
  ["South", 20],
  ["West", 30],
]);

workbook.setStyle("Inputs!A1:C1", {
  font: { bold: true },
  fill: { color: "#E2F0D9" },
});

const recalc = workbook.recalculate();
const layout = workbook.layout({ sheetName: "Inputs" });
const out = workbook.save();
workbook.dispose();
~~~

## Testing Strategy

Every feature should be proven through the lowest useful layer and at least once
through the public TS/WASM surface when exposed there.

| Layer | Purpose | Tools |
| --- | --- | --- |
| Rust unit tests | Parsing, validation, DTO behavior, mutation invariants | `cargo test -p xlcore-api`, `cargo test -p xlcore-types` |
| OOXML round trips | Save/reopen and preservation of unrelated parts | committed fixtures, targeted XML inspection |
| TS/WASM tests | Public wrapper, generated types, save/reopen smoke | Vitest or package smoke scripts |
| Renderer/browser | Visual layout integration and WASM lifetime issues | existing renderer, browser harness, screenshots |

Fixture builders:

- Use `openpyxl` for simple mechanical workbooks. It has no formula engine, but
  it is sufficient for sheet structure, values, styles, merges, comments, and
  tables.
- Use `hsx`/SpreadJS when behavior is the thing under test: copy/fill,
  insert/delete movement, dependency tracing, and screenshot comparisons.
- Do not require byte-for-byte parity with SpreadJS. Compare semantic workbook
  state, rendered output, object inventory, and selected OOXML invariants.

## Manual Validation With `hsx`

For any structural edit (`insert_rows`, `delete_columns`, `copy_range`,
`fill_range`, merges, freeze, defined-name shifts), the quickest sanity check is
to apply the op through `xlcore-api`, save the workbook, and read it back
through `hsx`. `hsx` (SpreadJS) is the authority on what an Excel-side consumer
actually sees.

Loop:

1. Write a throwaway driver under `crates/xlcore-api/examples/`
   (`cargo run -p xlcore-api --example <name>`) that builds the workbook,
   applies the op, and `save_path`s to `/tmp/<scenario>.xlsx`. Don't commit it.
2. Inspect with `hsx`:
   - `hsx --no-daemon get <file> <Sheet!Range> --formulas` — values + formulas
     as SpreadJS reads them.
   - `hsx --no-daemon info <file>` — sheet list + used range.
3. For OOXML facts `hsx` doesn't surface (merges, panes, defined names),
   `unzip -p <file> xl/worksheets/sheetN.xml | rg -o '<mergeCell[^/]*/>'`
   etc. is fine.
4. If the output disagrees with what Excel would show, the bug is in xlcore;
   fix and re-run.

This is a local-only loop. `hsx` is not a CI dependency and never imported from
crates or packages.

## Diagnostics Contract

Current errors are generated as `ApiError` and exposed to TS as `ApiError`.
Keep codes stable and add only when behavior needs caller recovery.

| Code | Meaning |
| --- | --- |
| `invalid_ref` | A1 reference cannot be parsed or is outside supported bounds |
| `missing_sheet` | Referenced sheet does not exist |
| `duplicate_sheet` | Create/rename would duplicate a sheet name |
| `invalid_sheet_name` | Sheet name violates Excel rules |
| `cannot_delete_last_sheet` | Deleting would leave no worksheets |
| `shape_mismatch` | Range matrix dimensions do not match the target range |
| `unsupported_formula` | Formula could not be evaluated; source/cache preserved where possible |
| `unsupported_style` | Style patch contains values we cannot serialize (e.g. invalid color) |
| `merge_overlap` | Requested merge overlaps an existing merge on the sheet |
| `invalid_hyperlink` | Hyperlink patch is missing both target and location, or has an empty target |
| `invalid_comment` | Comment patch has empty text |
| `invalid_threaded_note` | Threaded-note patch has empty text or reply parent id not found |
| `invalid_search_query` | Search query is empty or contains an invalid regex/wildcard pattern |
| `invalid_data_validation` | Data validation patch is missing required formulas/operator or mixes incompatible fields |
| `invalid_defined_name` | Defined name violates Excel naming rules or has empty formula |
| `invalid_property` | Workbook property patch has an unparseable value (e.g. non-ISO timestamp) |
| `invalid_table` | Table patch is missing a range when creating, has an invalid name, overlaps an existing table, or uses incompatible geometry |
| `duplicate_table` | Reserved for future use; table upsert currently treats duplicate names as updates |
| `invalid_protection` | Protection patch has a non-hex password, an empty hash/salt/algorithm string, or other malformed credential field |
| `invalid_page_setup` | Page setup patch has an out-of-range scale, zero copies, or a negative/non-finite margin |
| `invalid_auto_filter` | Auto-filter column patch references a non-existent filter, an out-of-range column offset, or an empty/unsupported criteria shape |
| `invalid_conditional_format` | Conditional format rule patch is missing required formula/operator/text for the rule kind, or has a non-positive priority |
| `invalid_chart` | Chart patch has no series, or a series `values_ref` is empty |
| `invalid_image` | Image patch has empty bytes, an unrecognized format that wasn't explicitly specified, or a non-finite rotation/crop value |
| `invalid_shape` | Shape patch has an unknown `prstGeom` preset or a non-finite rotation value |
| `invalid_sparkline_group` | Sparkline group patch has no entries, an invalid location/dataRef, or a non-RRGGBB color |
| `invalid_pivot` | Pivot patch has no data field, no row/column field, a source with no data rows, or references a field missing from the source header |
| `unsupported_formula` | Formula could not be evaluated; source/cache preserved where possible |
| `unsupported_object` | Requested chart/table/drawing/pivot operation is not implemented |
| `lossy_operation` | Operation completed but normalized/discarded unsupported details |
| `ooxml_write_error` | Writer could not serialize a valid workbook |

`Workbook::batch` returns a `BatchOutcome<T> { value, warnings, error }`:
warnings accumulated during the closure are always surfaced, and the first
fatal error short-circuits but does not roll back already-applied mutations.
Outside of `batch`, callers can collect ambient warnings with `warnings()` /
`take_warnings()` (TS: `workbook.warnings()` / `workbook.takeWarnings()`).
Transactional/rollback semantics remain out of scope.

## Definition Of Done

An API feature is done when:

- Rust facade exposes it.
- WASM/TypeScript expose it or explicitly document why it is Rust-only.
- Tests cover mutation, save, reopen, and diagnostics.
- Layout extraction reflects the mutation when visible.
- Recalc behavior is explicit when formulas are involved.
- Existing unrelated OOXML survives the operation.
