# Plan: off-main-thread formula recalc

## Problem

`@hewliyang/xlsx-preview` exposes a WASM `Workbook` (see `src/api.ts`). Hosts
that wire cell editing (see `examples/xlsx-app.html` → `applyEdit`) call
`Workbook.recalculate()` + `Workbook.layout()` **synchronously on the main
thread** on every commit. Both are heavy wasm-bindgen calls, so the previewer
janks on Enter.

`previewer.ts` itself does NOT evaluate formulas. Its only engine calls are
lightweight and must stay **synchronous + zero-latency** because they run inside
the render path on every keystroke:

```ts
interface PreviewerEngine {
  parseReferences(sheetName, anchorRef, formula): DependencyReference[]; // highlights
  functionNames(): string[];                                             // autocomplete
}
```

## Solution

Run the authoritative `Workbook` inside a persistent Web Worker; keep the
previewer a pure UI component. Provide a `WorkerWorkbook` proxy under a new
`./worker` package entry.

Zero-latency highlights: `parse_formula_references` (Rust) only needs the
workbook's **sheet names + defined names** — never cell values (verified in
`crates/xlcore-api/src/dependencies.rs`: `dependency_context()` builds only
`sheet_names` + `defined_names`). So the proxy keeps a tiny main-thread "shadow"
`Workbook` (sheets-by-name + defined names, no cell data) purely to satisfy
`PreviewerEngine` synchronously. The shadow is rebuilt only when structure
changes (rare), not on cell edits.

Existing worker infra to mirror: `src/xlsxWorker.ts` (one-shot extraction
worker) + `createExtractionWorker()`/cross-origin shim in `src/browserLoader.ts`.

## RPC protocol (editWorker)

Request:  `{ id: number, op: Op, args }`
Response: `{ id, ok: true, result } | { id, ok: false, error: XlsxLoadErrorPayload }`

| op            | args                                              | result                          |
| ------------- | ------------------------------------------------- | ------------------------------- |
| `open`        | `{ bytes: ArrayBuffer, wasmBinaryUrl: string }`   | `{ layout, structure }`         |
| `applyEdit`   | `{ sheetName, address, input, recalc: boolean }`  | `{ layout, structure }`         |
| `recalculate` | `{}`                                              | `{ layout, structure }`         |
| `layout`      | `{ options? }`                                    | `{ layout }`                    |
| `save`        | `{}`                                              | `{ bytes: Uint8Array }`         |

- `structure` = `{ sheets: string[]; definedNames: DefinedNameInfo[] }`.
- `applyEdit`: if `input.startsWith("=")` → `setFormula`, else `setValue(coerce(input))`;
  if `recalc` → `recalculate()`; then return fresh `layout()` + `structure`.
- `coerce(input)`: `""`→`null`; `/^-?\d+(\.\d+)?$/`→`Number`; `"true"`/`"false"`→bool; else string
  (mirror `coerceInput` in `examples/xlsx-app.html`).
- Errors: wrap via `xlsxLoadErrorPayloadFromUnknown` from `src/errors.ts`.

### Item 3 — feat(xlsx-preview): async-capable pivot/table filter controllers + WorkerWorkbook ops

Routing pivot/table filtering through the worker needs the controllers to allow
`Promise` returns and `WorkerWorkbook` to expose the data/mutation ops.

- `src/pivotFilterPopover.ts` + `src/tableFilterPopover.ts`: relax the controller
  interfaces to `T | Promise<T>` for every method (`items`, `hiddenValues`/
  `activeValues`, `setHidden`/`setFilter`/`setSort`). Make each popover `render`
  `async` and `await Promise.resolve(controller.x(...))`; re-guard `if (!menu) return`
  after each await (menu may close during the await). Checkbox/sort/clear handlers
  `await` the mutation then `onChange(layout)` + re-render. Sync controllers must
  still work unchanged (awaiting a non-Promise is a no-op).
- `src/editWorker.ts`: refactor to hold a high-level `Workbook` (from `./api.js`)
  instead of the raw `WasmWorkbookHandle` (so it can reuse `distinctValuesFor`,
  `ws.pivots`, `ws.autoFilter`). `open` -> `Workbook.open(bytes, { wasmBinaryUrl })`
  (drops the manual `init`); `applyEdit` -> `wb.sheet(name).cell(addr).setFormula/
  setValue` + `wb.recalculate()`; `layout`/`save`/`structure` via the `Workbook` API.
  Add ops:
  - `pivotMetas` `{}` -> `{ pivots: Array<{ name; sheet; id; sourceRef }> }`
    (flatten `ws.pivots.list()` across `wb.worksheets()`).
  - `distinctValues` `{ sourceRef, field }` -> `{ values: string[] }`
    (`distinctValuesFor(wb, sourceRef, field)`).
  - `updatePivot` `{ sheet, id, patch }` -> `{ layout, structure }`
    (`wb.sheet(sheet).pivots.update(id, patch)`).
  - `tableSetFilter` `{ rangeRef, columnOffset, field, values }` -> `{ layout, structure }`:
    resolve ws from `rangeRef` (lastIndexOf `!`, unquote; else `activeSheet()`),
    ensure `autoFilter.get()` else `set(rangeRef)`; `all = distinctValuesFor(...)`;
    if `values.length === 0 || values.length >= all.length` -> `removeColumn(columnOffset)`
    else `setColumnValues(columnOffset, values)`.
  - `tableSetSort` `{ rangeRef, columnOffset, descending }` -> `{ layout, structure }`:
    same ws resolution; `descending === null` -> `clearSort()` else `setSort(columnOffset, { descending })`.
- `src/worker.ts` (`WorkerWorkbook`): add async methods mirroring the ops:
  `pivotMetas()`, `distinctValues(sourceRef, field)`, `updatePivot(sheet, id, patch)`,
  `tableSetFilter({ rangeRef, columnOffset, field, values })`,
  `tableSetSort({ rangeRef, columnOffset, descending })`. Mutating ones refresh the
  shadow from the returned `structure` and resolve to the `WorkbookLayout`.
- Verify: `pnpm -C packages/xlsx-preview build:ts` + `node scripts/check-dist-imports.mjs`.

### Item 4 — feat(example): re-enable pivot/table filtering through WorkerWorkbook
- `examples/xlsx-app.html`: rewrite `buildPivotController`/`buildTableController` as
  **async** builders (`await buildPivotController(wb)` at the call site) that keep
  their local state maps (`hidden` / `kept`) on the main thread but route all data
  reads + mutations through the new `WorkerWorkbook` async methods. Drop the
  `typeof wb.worksheets !== "function"` guards. `items` -> `await wb.distinctValues`;
  pivot `setHidden` -> `await wb.updatePivot`; table `setFilter` -> `await wb.tableSetFilter`;
  `setSort` -> `await wb.tableSetSort`.
- Verify e2e in browser (server: `node scripts/preview.mjs`): open
  `tests/fixtures/pivot/pivot-simple.xlsx`, click a pivot field filter, uncheck a
  value -> grid updates; open a workbook with a table/autofilter, filter + sort a
  column -> grid updates; no console errors.

## Shipped

- Item 1 — edit worker (`src/editWorker.ts`) + `WorkerWorkbook` proxy with shadow workbook under `./worker` entry.
- Item 2 — `examples/xlsx-app.html` routes editing/recalc/save through `WorkerWorkbook`; previewer `engine` = `wb.engine`.
- Item 3 — async-capable pivot/table filter controllers + `WorkerWorkbook` pivot/table ops (`pivotMetas`/`distinctValues`/`updatePivot`/`tableSetFilter`/`tableSetSort`).
