# API Parity Hillclimb

This is the working checklist for the workbook manipulation API: a Rust-first
facade that can be exposed through WASM and wrapped in TypeScript. The target is
not just "edit cells"; it is an agent-usable spreadsheet API that can open
existing workbooks, preserve unrelated OOXML, mutate safely, recalculate, render,
and save.

The short version: model the ergonomic surface after SpreadJS/EPPlus, keep the
actual mutation semantics in Rust, and prove every public operation through the
same fixture loop we use for rendering and formula parity.

## Current State

Implemented:

- xlcore-api crate with Workbook::new, open_bytes, open_path, save_bytes,
  save_path, load_report, sheets, create_sheet, rename_sheet, delete_sheet,
  get_cell, set_value, set_formula, clear, batch, recalculate, layout, and
  recalculate_layout.
- A1 cell refs with quoted sheet names and absolute markers.
- Blank workbook creation through a minimal OOXML template.
- Scalar value writes for blank, string, number, boolean, and error cells.
- Formula writes preserve source formula text and mark workbook formula caches
  stale until recalculate() is called.
- Recalc/writeback is wired through xlcore-bridge.
- Layout extraction uses the same xlcore-export path as preview.
- xlcore-wasm exposes a stateful WorkbookHandle with open/create, sheet/cell
  mutation, recalc, layout, save, and dispose.
- packages/xlsx-preview/src/api.ts wraps the wasm handle as a TypeScript
  Workbook class.
- packages/xlsx-preview has smoke:api for create -> mutate -> recalc -> layout
  -> save -> reopen.

Still open from P0:

- Range get/set values and formulas.
- Style patch writes.
- Row/column resize, hide, insert/delete, and freeze panes.
- Merge/unmerge.
- Transactional or diagnostic-rich batch semantics beyond a simple closure.
- Browser harness UI for manual API mutation testing.

## Blueprint Sources

### SpreadJS / hsx

Local references:

- which hsx resolves to the local @hewliyang/headless-spreadjs install.
- @hewliyang/headless-spreadjs/dist/index.d.ts exposes init(), ExcelFile,
  SpreadWorkbook, SpreadWorksheet, and SpreadStyle.
- @mescius/spread-sheets/dist/gc.spread.sheets.d.ts is the full SpreadJS API
  declaration set.

Useful shapes to mirror:

- Stateful workbook handle: ExcelFile.open(), openFromBuffer(), saveToBuffer(),
  toJSON(), fromJSON(), and batch().
- Workbook operations: sheet list/add/delete/rename, JSON import/export,
  calc suspension/resume, command/undo managers.
- Worksheet operations: get/set value, get/set formula, get/set style, range
  copy, row/column mutation, sizing, visibility, freeze panes.
- Range-first ergonomics: A1 refs are the public language for users and agents.
- hsx command coverage is a good minimum API smoke list: create, info, get, csv,
  set, clear, search, copy, diff, deps, refs, sheet, rc, resize, objects,
  screenshot, and eval.

SpreadJS is the best behavioral oracle for interactive workbook manipulation,
not the implementation base. Our implementation must still preserve OOXML parts
that SpreadJS may normalize or discard.

### EPPlus

EPPlus is the object-model reference to mine when the Rust shape gets more
complete:

- ExcelPackage as the open/save root.
- Workbook.Worksheets as the sheet collection.
- ExcelWorksheet.Cells[...] / ExcelRange for values, formulas, styles, merge,
  copy, clear, and load helpers.
- First-class collections for tables, drawings, comments, data validation,
  conditional formatting, names, charts, and pivot tables.

Use EPPlus to sanity-check breadth and naming once the P0 API exists. Clone to
/tmp/EPPlus only when we are ready to mine exact object coverage.

## Design Rules

- Rust owns mutation semantics. WASM and TypeScript are wrappers over the same
  Rust path, not independent workbook models.
- OOXML fidelity wins over API convenience. Unknown parts, relationship IDs,
  extensions, charts, drawings, styles, comments, tables, and workbook metadata
  must survive unrelated edits.
- Public refs are A1-style strings. Internal row/column indexes can stay
  zero-based or schema-native, but the user-facing API should avoid coordinate
  footguns.
- Mutations should be batchable. Batch mode gives agents a single diagnostic
  envelope and avoids repeated parse/recalc/layout work.
- Recalc is explicit by default. Mutation does not silently pretend formulas are
  fresh; callers choose recalculate() or save({ recalculate: true }).
- Errors are structured diagnostics. Partial success is allowed only when every
  skipped or degraded operation is reported with sheet/range context.
- Layout extraction is a first-class output. The same workbook handle should
  feed layout(), screenshot tests, and final save().
- TypeScript should feel natural, but not hide unsupported behavior. Prefer
  narrow typed options over a huge SpreadJS-compatible surface that silently
  no-ops.

## Proposed Layers

| Layer | Role |
| --- | --- |
| xlcore-api | Rust facade for open/save, workbook sessions, sheet/cell/range mutation, recalc, layout, and diagnostics. |
| xlcore-wasm | WASM bindings over xlcore-api, including a stateful workbook handle with dispose(). |
| packages/xlsx-preview or a sibling package | TypeScript wrapper that exposes the ergonomic workbook API and reuses the existing renderer. |
| xlcore-cli | Thin smoke/debug surface over the same API for fixtures and agent scripts. |

xlcore-api can start as an internal crate if the public Rust API is still fluid,
but it should quickly become the only mutation path used by CLI, WASM, and tests.

## Target TypeScript Shape

This is a sketch, not a committed declaration file:

~~~ts
import { Workbook } from "@hewliyang/xlsx-preview/api";

const workbook = await Workbook.open(bytes);

workbook.sheets.list();
workbook.sheets.create("Scenario");
workbook.sheets.rename("Sheet1", "Inputs");

workbook.cell("Inputs!A1").setValue("Units");
workbook.cell("Inputs!B1").setValue(100);
workbook.cell("Inputs!C1").setFormula("=B1*1.08");

workbook.range("Inputs!A1:C10").setStyle({
  font: { bold: true },
  formatter: "$#,##0.00",
});

workbook.batch((tx) => {
  tx.range("Inputs!A2:A4").setValues([["North"], ["South"], ["West"]]);
  tx.range("Inputs!B2:B4").setValues([[10], [20], [30]]);
  tx.cell("Inputs!B5").setFormula("=SUM(B2:B4)");
});

const recalc = await workbook.recalculate();
const layout = await workbook.layout({ sheetName: "Inputs" });
const output = await workbook.save({ recalculate: false });

workbook.dispose();
~~~

### Core Types

~~~ts
type CellScalar = string | number | boolean | null;

type CellPatch = {
  value?: CellScalar;
  formula?: string;
  style?: CellStylePatch;
  note?: string | null;
  hyperlink?: string | null;
};

type CellStylePatch = {
  formatter?: string;
  font?: {
    family?: string;
    size?: number;
    bold?: boolean;
    italic?: boolean;
    underline?: boolean;
    color?: string;
  };
  fill?: { color?: string };
  align?: {
    horizontal?: "left" | "center" | "right";
    vertical?: "top" | "middle" | "bottom";
    wrap?: boolean;
  };
  border?: Partial<Record<"top" | "right" | "bottom" | "left", BorderPatch>>;
};

type BorderPatch = {
  style?: "thin" | "medium" | "thick" | "dashed" | "dotted" | "double";
  color?: string;
};

type ApiDiagnostic = {
  severity: "info" | "warning" | "error";
  code: string;
  message: string;
  sheet?: string;
  ref?: string;
  part?: string;
};
~~~

Keep style patches intentionally smaller than the extraction schema at first.
The P0 goal is useful workbook creation and editing, not complete style
round-tripping through a handwritten object model.

## Target Rust Shape

Again, sketch:

~~~rust
let mut workbook = xlcore_api::Workbook::open_bytes(bytes)?;

workbook.sheet("Inputs")?.cell("B1")?.set_value(100.0)?;
workbook.sheet("Inputs")?.cell("C1")?.set_formula("=B1*1.08")?;

workbook.batch(|tx| {
    tx.range("Inputs!A2:A4")?.set_values([["North"], ["South"], ["West"]])?;
    tx.cell("Inputs!B5")?.set_formula("=SUM(B2:B4)")?;
    Ok(())
})?;

let recalc = workbook.recalculate()?;
let layout = workbook.layout(Default::default())?;
let out = workbook.save_bytes(SaveOptions { recalculate: false })?;
~~~

The Rust API should use strongly typed structs internally, but accept A1 refs at
the boundary because that is what agents, users, and existing spreadsheet tools
already speak.

## Parity Ladder

### P0: Basic Workbook Editing

Goal: create/open/edit/save a workbook and preview the result in the browser.

- Open .xlsx bytes into a mutable workbook session.
- Create a blank workbook with one sheet.
- Save back to .xlsx bytes.
- List sheets with id, name, visibility, active state, dimensions, and tab color.
- Create, rename, delete, move, hide, and show worksheets.
- Read cells and ranges as values, formulas, and simple style summaries.
- Set scalar cell values: string, number, boolean, blank, and error literals.
- Set formulas while preserving the source formula text.
- Clear values, formulas, styles, or all cell content.
- Set a narrow style subset: number format, font family/size/bold/italic/color,
  fill color, horizontal/vertical alignment, wrap, and simple borders.
- Resize rows and columns.
- Freeze panes.
- Merge and unmerge ranges.
- Batch multiple mutations with one diagnostic result.
- Recalculate scalar formulas through xlcore-bridge.
- Extract layout from the mutated workbook.
- Return structured diagnostics for unsupported formulas, invalid refs,
  unsupported style fields, and lossy operations.

Acceptance: an API test can create a workbook from scratch, write values and
formulas, recalculate, render through the existing preview pipeline, save, reopen,
and observe the same values/formulas/styles.

### P1: Range Semantics And Agent Workflows

Goal: match the operations agents need to manipulate existing workbooks.

- Copy/paste ranges with relative formula translation.
- Fill/down/right simple patterns.
- Insert/delete rows and columns with reference movement.
- Hide/unhide rows and columns.
- Search values and formulas across sheets.
- Diff two workbooks by value, formula, style subset, and sheet structure.
- Trace precedents and dependents using the formula dependency graph.
- Expose workbook and sheet defined names.
- Add/edit comments and hyperlinks.
- Add simple data validation lists.
- Add tables from a range with header/totals options.
- Evaluate table totals and formulas well enough for rendered previews.
- Return object inventory: tables, charts, pivots, drawings, images, comments.

Acceptance: we can reproduce the useful non-screenshot parts of the hsx command
surface using xlcore only.

### P2: Spreadsheet Object Model Breadth

Goal: cover the surface expected by consumers building spreadsheet-generating
apps, closer to SpreadJS/EPPlus.

- Conditional formatting authoring, including formula rules once engine support
  is real.
- Chart creation/editing for the chart families already rendered by
  xlsx-preview.
- Image insertion, sizing, crop, and rotation.
- Rich text writes.
- Table style writes and autoFilter changes.
- Pivot table preservation plus limited refresh/creation once aggregation
  support exists.
- Sheet protection and workbook protection metadata.
- Page setup, print areas, headers/footers.
- Undo/redo or a replayable command journal.
- Optional JSON import/export for app-state persistence.

Acceptance: consumers can generate non-trivial reports from scratch without
dropping to raw OOXML for common spreadsheet features.

## Testing Strategy

Use a layered loop. Every API feature should be proven as low as possible and
then at least once through the public WASM/TypeScript surface.

### 1. Rust API Unit Tests

Use these for parsing, validation, mutation structs, A1 refs, and small in-memory
workbooks.

Good cases:

- A1 parsing with quoted sheet names, absolute refs, ranges, whole rows/columns.
- Set/get scalar values.
- Set/get formulas with and without a leading equals sign.
- Merge/unmerge bookkeeping.
- Batch rollback or partial-success semantics, whichever we choose.
- Diagnostics for invalid refs and unsupported operations.

### 2. OOXML Round-Trip Fixtures

Use committed .xlsx fixtures to prove that the writer changes only the intended
parts and preserves unrelated workbook content.

Fixture builders can use openpyxl for mechanical workbook creation when no
calculation is needed. Use hsx/SpreadJS when we need SpreadJS behavior, reference
screenshots, dependency tracing, or a comparison against its mutation semantics.

For each fixture:

- Apply one xlcore API operation script.
- Save the workbook.
- Reopen with xlcore and hsx if applicable.
- Assert values, formulas, sheet metadata, and layout JSON.
- Inspect key OOXML parts when preservation is the feature under test.

### 3. WASM And TypeScript Tests

Add Vitest coverage around the generated WASM package and the TypeScript wrapper.

Good cases:

- Workbook.open(bytes) creates a disposable handle.
- Mutations survive save() and reopen.
- layout() returns the same schema currently consumed by the previewer.
- Invalid calls throw enriched errors with code, message, and range context.
- Handles reject use-after-dispose.
- Batches return diagnostics in stable order.

### 4. hsx / SpreadJS Oracle Tests

Use hsx as a behavioral oracle where it is strongest:

- Range copy/fill behavior.
- Row/column insert/delete movement.
- Dependency tracing through deps and refs.
- Workbook diffs.
- Screenshot smoke tests after mutations.
- Object inventory for charts/tables/pivots/drawings.

Do not require byte-for-byte parity with SpreadJS output. The comparison should
be semantic: values, formulas, visible layout, object inventory, and selected
OOXML invariants.

### 5. Browser Workflow Tests

Once the TypeScript wrapper exists, add a tiny browser harness that opens a
workbook, mutates it, recalculates, and renders the result through the existing
canvas preview. This catches WASM lifetime issues and renderer integration bugs
that Node-only tests miss.

## Fixture Matrix

| Fixture | Purpose | Oracle |
| --- | --- | --- |
| api/create-basic.xlsx | create workbook, values, formulas, styles | xlcore reopen + layout |
| api/sheet-ops.xlsx | create/rename/delete/move/hide sheets | xlcore + hsx info |
| api/range-copy.xlsx | copy/paste formulas and styles | hsx copy behavior |
| api/row-col-ops.xlsx | insert/delete/hide/resize/freeze | hsx + OOXML inspection |
| api/merge-clear.xlsx | merge/unmerge and clear modes | xlcore reopen |
| api/comments-links.xlsx | comments and hyperlinks | existing renderer + hsx |
| api/table-basic.xlsx | table creation and totals | Excel/hsx |
| api/style-subset.xlsx | supported write style subset | renderer screenshot |
| api/preserve-rich.xlsx | edits around charts/drawings/tables/comments | OOXML diff + preview |
| api/errors.xlsx | invalid refs, bad formulas, unsupported writes | diagnostics snapshots |

## Diagnostics Contract

API calls should return or throw structured diagnostics, not string-only errors.
Suggested codes:

| Code | Meaning |
| --- | --- |
| invalid_ref | A1 ref cannot be parsed or points outside supported bounds. |
| missing_sheet | Referenced worksheet does not exist. |
| duplicate_sheet | Create/rename would duplicate a sheet name. |
| unsupported_formula | Formula could not be evaluated; source/cache preserved where possible. |
| unsupported_style | Style patch contains fields we do not write yet. |
| unsupported_object | Requested chart/table/drawing/pivot operation is not implemented. |
| lossy_operation | Operation completed but normalized or discarded unsupported details. |
| ooxml_write_error | Writer could not serialize a valid workbook. |

Batch calls should return all non-fatal warnings plus the first fatal error. If
we choose transactional batch semantics later, the diagnostic envelope must also
say whether mutations were committed or rolled back.

## Open Design Questions

- Package shape: keep mutation API under @hewliyang/xlsx-preview, or split a
  sibling package such as @hewliyang/xlcore.
- WASM lifetime: integer handles behind functions, or exported WorkbookHandle
  class with explicit dispose().
- Async boundary: make all public TypeScript operations async, or only open/save
  and expensive recalc/layout calls.
- Batch semantics: transactional rollback vs partial success with diagnostics.
- Date/time values: expose JS Date, serial numbers, or explicit tagged values.
- Error literals: typed CellError enum vs strings like "#DIV/0!".
- Style compatibility: use our renderer schema, a SpreadJS-like patch object, or
  a deliberately small xlcore-specific style patch.
- Formula freshness: should save() default to recalc once recalc coverage is
  stronger, or remain explicit forever.
- Undo/redo: command journal in Rust, TypeScript wrapper only, or defer.

## Definition Of Done

An API feature is done only when all of these are true:

- The operation is available through the Rust facade.
- WASM and TypeScript expose the same behavior or document why it is Rust-only.
- Tests cover successful mutation, save, reopen, and diagnostics.
- Existing unrelated OOXML survives the operation.
- Layout extraction reflects the mutation.
- Recalc behavior is explicit and tested when formulas are involved.
- hsx/SpreadJS or Excel behavior is used as an oracle when the feature has
  ambiguous spreadsheet semantics.

## Next Slice

Continue P0 in this order:

1. Add range get/set values and formulas, including matrix shape validation.
2. Add the narrow style patch writer for number format, font, fill, alignment,
   wrap, and simple borders.
3. Add row/column resize, hide, insert/delete, and freeze panes.
4. Add merge/unmerge.
5. Add a browser harness for manual workbook mutation testing.
6. Add xlcore api or equivalent CLI smoke commands once the Rust/WASM shape
   stops moving.
