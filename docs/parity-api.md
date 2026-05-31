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

- Style write surface limited to font/fill/border/alignment/number format patch
- No row/column structural edits
- No merges
- No comments, hyperlinks, tables, names, validation, or object authoring
- Batch is a simple Rust closure, not a diagnostic/transaction envelope

## API Parity Table

Status key:

- Done: implemented and tested through at least Rust or TS smoke coverage
- Partial: some support exists, but not enough for the target surface
- P0: next workbook editing layer
- P1: agent workflow layer
- P2: report-generation/object-model breadth
- Later: explicitly out of the current hillclimb

| Area | SpreadJS reference | xlcore target | Status | Test oracle |
| --- | --- | --- | --- | --- |
| Open/create/save | `Workbook`, JSON/file flows | Open bytes/path, create blank workbook, save bytes/path, preserve unrelated OOXML | Done | Rust API + save/reopen |
| Shared DTOs | SpreadJS `.d.ts` surface | Rust DTOs generated to TS from `xlcore-types` | Done | `scripts/regen-api-schema.sh` |
| Workbook metadata | `Workbook.options`, `docProps`, workbook name | Read/write core properties, active sheet, calc properties | P1 | OOXML inspection |
| Sheet collection | `Workbook.getSheet/addSheet/removeSheet/setSheet` | List/create/rename/delete/move/hide/show/active sheet | Done | Rust API + TS smoke |
| Cell scalar IO | `Worksheet.getValue/setValue`, `Range.value` | Get/set scalar values and errors by A1 ref | Done | Rust API + TS smoke |
| Cell formulas | `getFormula/setFormula`, calc APIs | Set formula text, preserve formula, explicit recalc/writeback | Partial | xlcore-engine + Excel fixtures |
| Range values | `Worksheet.getArray/setArray`, `Range.value` | Get/set rectangular matrices with shape validation | Done | Rust API + TS smoke |
| Range formulas | `Range.formula`, copy/fill APIs | Set formula matrices, copy formulas with relative refs | Partial | Rust API + TS smoke (matrix set/get); copy/fill still P1 |
| Clear modes | `clear`, `ClearPendingChangeType` | Clear values, formulas, styles, comments, or all | P0 | OOXML diff + reopen |
| Style subset | `Style`, `Range.backColor/font/border/formatter` | Number format, font, fill, alignment, wrap, simple borders | Done | Rust API + layout |
| Full styles | Themes, named styles, table styles | Preserve existing styles; author broader style model later | P2 | OOXML diff + preview |
| Row/column size | `setRowHeight/setColumnWidth`, auto fit | Resize rows/columns; optional auto-fit later | Done | Rust API + save/reopen |
| Row/column visibility | `setRowVisible/setColumnVisible` | Hide/show rows and columns | Done | Rust API + save/reopen |
| Insert/delete rows/cols | `addRows/deleteRows/addColumns/deleteColumns` | Structural edits with cell/formula/reference movement | P0/P1 | hsx + OOXML diff |
| Freeze panes | `frozenRowCount/frozenColumnCount` | Freeze/unfreeze panes | Done | Rust API + save/reopen |
| Merges | `addSpan/removeSpan/getSpans` | Merge/unmerge/list merged ranges | Done | Rust API + TS smoke |
| Selections/view state | `setSelection/getSelections/showCell` | Persist active cell/selection only if OOXML-backed and useful | Later | OOXML inspection |
| Search | `Worksheet.search` | Search values/formulas across sheets | P1 | hsx search oracle |
| Copy/paste/fill | `copyTo`, fill APIs | Copy ranges, fill down/right, translate relative formulas | P1 | hsx copy/fill oracle |
| Diff | Not a single SpreadJS API; hsx has diff | Compare workbook values/formulas/style subset/sheet structure | P1 | xlcore fixtures |
| Dependencies | Calc engine refs/deps behavior | Precedents/dependents from formula graph | P1 | hsx deps/refs oracle |
| Defined names | `Workbook.names`, `NameInfo` | List/create/update/delete workbook and sheet names | P1 | OOXML + formulas |
| Comments/notes | `Comments.CommentManager` | Add/edit/delete/list comments and threaded notes when present | P1 | Renderer + OOXML |
| Hyperlinks | Worksheet hyperlink APIs | Add/edit/delete/list cell hyperlinks | P1 | Renderer + OOXML |
| Tables | `Tables.TableManager`, `Table` | Create table from range, headers/totals, resize, style name | P1/P2 | Excel/hsx + renderer |
| AutoFilter | Table/filter APIs | Preserve filters first; author simple filters later | P1/P2 | OOXML diff |
| Data validation | `DataValidation` APIs | Add/edit/delete/list list and scalar validations | P1 | OOXML + Excel open |
| Conditional formatting | `ConditionalFormatting` APIs | Preserve existing; author basic rules once style writes exist | P2 | Renderer screenshot |
| Charts | `Charts`, `Shapes` | Preserve; create/edit chart types already rendered | P2 | Preview + OOXML |
| Images | `Shapes.Picture`, shape collection | Insert image, position, size, crop/rotation later | P2 | Preview + OOXML |
| Shapes | `Shapes.ShapeCollection` | Preserve first; author basic shape/text later | P2 | Preview + OOXML |
| Sparklines | `Sparklines` APIs | Preserve; author simple line/column/win-loss later | P2 | Renderer screenshot |
| Pivot tables | `PivotTableManager`, slicers/timelines | Preserve; refresh/create only after aggregation model exists | P2 | OOXML + Excel open |
| Slicers/timelines | `Slicers`, pivot slicers | Preserve; authoring deferred | Later | OOXML diff |
| Protection | Sheet/workbook protection APIs | Sheet/workbook protection metadata | P2 | OOXML + Excel open |
| Print/page setup | Print, page setup, headers/footers | Page setup, print areas, headers/footers | P2 | OOXML inspection |
| JSON import/export | `toJSON/fromJSON` | Optional xlcore JSON for app state, not SpreadJS-compatible by default | Later | Round-trip tests |
| Undo/redo/commands | Command manager, undo manager | Optional command journal/replay log | Later | Unit tests |
| Events/UI options | Events, context menu, scrollbars, hit testing | Out of scope unless needed by previewer harness | Later | Browser harness only |
| Diagnostics | SpreadJS often throws or mutates silently | Stable `ApiError` and future batch diagnostics | Partial | Snapshot tests |
| Browser harness | SpreadJS UI runtime | Minimal local page to mutate/recalc/render via TS wrapper | P0 | Playwright/screenshot |

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
| hsx/SpreadJS oracle | Ambiguous spreadsheet behavior: copy/fill, row/col movement, deps, search | `hsx`, `gc.spread.sheets.d.ts` |
| Renderer/browser | Visual layout integration and WASM lifetime issues | existing renderer, browser harness, screenshots |

Fixture builders:

- Use `openpyxl` for simple mechanical workbooks. It has no formula engine, but
  it is sufficient for sheet structure, values, styles, merges, comments, and
  tables.
- Use `hsx`/SpreadJS when behavior is the thing under test: copy/fill,
  insert/delete movement, dependency tracing, diffs, and screenshot comparisons.
- Do not require byte-for-byte parity with SpreadJS. Compare semantic workbook
  state, rendered output, object inventory, and selected OOXML invariants.

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
| `unsupported_object` | Requested chart/table/drawing/pivot operation is not implemented |
| `lossy_operation` | Operation completed but normalized/discarded unsupported details |
| `ooxml_write_error` | Writer could not serialize a valid workbook |

Future batch calls should return all warnings plus the first fatal error. If we
choose transactional semantics, the envelope must state whether mutations were
committed or rolled back.

## Next Slices

1. Insert/delete rows/columns with cell/formula/reference movement.
2. Browser mutation harness for open -> mutate -> recalc -> render -> save.
3. hsx oracle fixtures for copy/fill, row/column insert/delete, search, deps,
   and diff.
4. Range copy/fill with relative-formula translation (lifts Range formulas
   from Partial to Done).
5. Clear modes (`values` / `formulas` / `styles` / `all`).

## Definition Of Done

An API feature is done when:

- Rust facade exposes it.
- WASM/TypeScript expose it or explicitly document why it is Rust-only.
- Tests cover mutation, save, reopen, and diagnostics.
- Layout extraction reflects the mutation when visible.
- Recalc behavior is explicit when formulas are involved.
- Existing unrelated OOXML survives the operation.
- hsx/SpreadJS or Excel is used as the oracle for ambiguous spreadsheet
  semantics.
