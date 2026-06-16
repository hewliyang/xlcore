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

## Backlog

### Item 2 — feat(example): route editing through `WorkerWorkbook`
- `examples/xlsx-app.html`: make `recalcWorkbook` a `WorkerWorkbook`. Add
  `worker.js` to the `loadXlsxPreviewRuntime({ modules: [...] })` list and pull
  `WorkerWorkbook` off `runtime`.
- `applyEdit` becomes `async`: `const { layout } = await wb.applyEdit({ sheetName, address: addr, input, recalc: autoRecalc }); previewer.replaceLayout(layout);` then move/select.
- Previewer `engine` = `wb.engine`. Recalc (F9/button) → `await wb.recalculate()`
  then `replaceLayout`. Download → `await wb.save()`.
- Keep the Auto-recalc toggle (drives the `recalc` flag).
- Verify e2e: load a sample `.xlsx`, type `=1+2` in a cell, Enter → cell shows
  `3`; edit a precedent of an existing formula → dependent updates; highlights
  still appear instantly while typing a formula; main thread stays responsive.

## Shipped

- Item 1 — edit worker (`src/editWorker.ts`) + `WorkerWorkbook` proxy with shadow workbook under `./worker` entry.
