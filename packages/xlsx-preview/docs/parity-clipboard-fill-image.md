# Parity: clipboard, fill handle, and image insert (Excel/Sheets-style)

Goal: bring the interactive previewer's editing gestures to parity with Excel /
Google Sheets for the three things users reach for constantly but we don't yet
support in the UI:

1. **Clipboard** — copy/cut/paste a cell range (Ctrl/Cmd-C / X / V), including
   round-tripping with external apps (Excel, Sheets) via TSV/HTML.
2. **Fill handle** — drag the bottom-right handle of a selection to copy/series-
   fill into the dragged region.
3. **Images** — drop an image file onto the sheet, or paste an image from the
   clipboard, and have it anchored at the target cell.

## What exists today (the gap)

Interactive editing is **single-cell only**. `interact.ts` `onKeyDown`
(`packages/xlsx-preview/src/interact.ts:1022`) handles arrow/Tab/Enter nav, F2,
and type-to-edit; there is **no Ctrl/Cmd-C/X/V branch**. The previewer's
`editInput`/`formulaBox` commit one cell via the `celledit` event
(`previewer.ts`), which the example app routes to
`WorkerWorkbook.applyEdit` → single-cell `setValue`/`setFormula`
(`xlsx-app.html` `applyEdit`).

Drag/drop in the example app (`xlsx-app.html:1304`) only loads a **whole file**
as a new workbook; it rejects anything that isn't `.xlsx/.csv/.tsv/.parquet`.
The only `clipboard.writeText` calls copy text *out* (cell text, diagnostics);
there is **no `paste` listener anywhere**. No fill handle is drawn or hit-tested.

### The backend already has everything

These are exposed in WASM and the synchronous `api`/`api-range` layer but **not
plumbed through `editWorker`/`WorkerWorkbook`** (the async path the interactive
previewer/example app actually use):

- `setRangeValues(sheet, ref, values)` / `setRangeFormulas(sheet, ref, formulas)`
  — `api-range.ts:50,55`, WASM `xlcore_wasm.d.ts:117-118`.
- `copyRange(sheet, ref, destSheet, destRef)` — `api-range.ts:87`,
  WASM `:23`. Preserves formulas/formats (internal-copy fidelity).
- `clearRange(sheet, ref)` / `clearRangeWith(sheet, ref, mode)` —
  WASM `:18-19` (for cut + paste-over).
- `setImage(sheet, ImagePatch{ bytes, anchor, format, ... })` creates/anchors an
  image — `api-collections.ts:242`, WASM `:111`. `ImagePatch.anchor` is an
  `AnchorSpec` (a two-cell A1 range string like `"C3:E10"` or an explicit
  `ChartAnchor`); the Rust facade resolves the string form.

So the missing work is almost entirely in **`editWorker.ts` + `worker.ts`
(WorkerWorkbook) + `interact.ts` + `previewer.ts` + the example app** — wiring,
gesture handling, and clipboard (de)serialization. The Rust side should need no
changes.

## Architecture notes for implementers

- `interact.ts` is callback-driven: it gets `opts` (getters/setters +
  `redraw`/`onEditStart`/`onDrawingMoved`/...) from `previewer.ts`. New gestures
  add new `opts.on*` callbacks there, fired from `interact.ts`, handled in
  `previewer.ts`, re-emitted as a previewer **event** the example app subscribes
  to (mirror the existing `celledit` / `drawingmoved` pattern exactly).
- The previewer already tracks `activeCell` + `selection` (a `Selection`
  `{r1,c1,r2,c2}`) with `getSelection()` / `selectRange()`.
- The fast edit path is **single-sheet**: worker ops return
  `{ layout: w.layout({ sheetName }), structure }`, the example app calls
  `previewer.patchSheetLayout(layout)`. New range/image ops MUST follow this
  (return single-sheet layout, recalc when `recalc` flag set) so paint stays
  partial — see `docs/parity-spreadjs.md`.
- Clipboard MIME strategy (match Sheets): write `text/plain` (TSV) for external
  apps **and** a private `text/html` table (or an app-private JSON blob keyed in
  the HTML) so an internal paste can restore formulas/formats via `copyRange`
  instead of flattening to values. On paste, prefer the private payload when the
  source is us; otherwise parse TSV → values.
- Use the async Clipboard API (`navigator.clipboard.read`/`write` with
  `ClipboardItem`) where available; fall back to a hidden `paste`/`copy` event +
  `DataTransfer` for `text/html` and images. Image blobs arrive as
  `clipboardData.files` / `getType("image/png")`.

## Verification

Per item: `pnpm --filter @hewliyang/xlsx-preview test` (builds dist + vitest +
freshness gates), plus `typecheck` + `lint` + `knip`.

**LOC gotcha:** `check:loc` (1500-line cap) is **already red on `main`** — 7
files exceed it, including `previewer.ts` (1569) and `interact.ts` (1264). Do
NOT touch unrelated oversized files to "fix" the gate, and crucially **do not
grow `previewer.ts`/`interact.ts`**: put all new logic in new small modules
(clipboard model, fill projection, image-drop helpers) and keep edits to those
two files to thin wiring/dispatch.
- Pure helpers (TSV/HTML serialize+parse, fill-series projection, drop-cell→
  anchor math) get unit tests — they're the testable core.
- Worker ops get an in-process `Workbook`-level round-trip test (open → mutate →
  read back → `save`/reopen) like `api.richText.test.ts`.
- Interactive gestures: dogfood in the example app (`pnpm run preview`), or
  drive a tiny render assertion; note that headless e2e of clipboard/drag is
  limited — supervisor verifies by hand in the browser.

## Backlog (one item per agent; do in order — later items depend on earlier)

### 1. Worker range ops (foundation)
Add `setRangeValues`, `setRangeFormulas`, `copyRange`, `clearRange` ops to
`editWorker.ts` and matching async methods on `WorkerWorkbook` (`worker.ts`),
each returning a single-sheet `{ layout: layout({ sheetName }), structure }` and
honoring a `recalc` flag (mirror `applyEdit`). No UI yet. Round-trip test at the
`Workbook` level. Gotcha: `copyRange` signature is
`(sheet, ref, destSheet, destRef)`; keep same-sheet dest the common case.

### 8. Image paste from clipboard
Handle a clipboard image on Ctrl/Cmd-V (when the payload is an image, not
cells): read the blob, anchor at the active cell, call #2 `setImage`. Shares the
paste entrypoint from #5 — branch on payload type (image blob vs TSV/HTML).
Verify: screenshot to clipboard → paste → image anchored at active cell, round-
trips through save.

## Shipped

1. Worker range ops — `setRangeValues`/`setRangeFormulas`/`copyRange`/`clearRange` on `WorkerWorkbook` + `editWorker.ts`, single-sheet layout, `recalc` flag; Workbook-level round-trip test.
2. Worker image-insert op — `setImage` op on `editWorker.ts` + `WorkerWorkbook.setImage(sheetName, patch)`, single-sheet layout, bytes sent as transferable `ArrayBuffer`; round-trip test.
3. Clipboard serialize/parse helpers — pure `clipboardModel.ts` (`serializeRange`/`parseClipboard`) with TSV quoting, HTML `<table>` + `data-xlcore` internal payload (values+formulas+range); unit-tested round-trips.
4. Range copy / cut — `interact.ts` Cmd/Ctrl+C/X → `opts.onCopy`; `previewer.ts` `handleCopy` serializes via #3, writes `text/plain`+`text/html` (`clipboardIo.ts`, fallback to `writeText`), records `cutRange` (for #5), emits `rangecopy`/`rangecut`. Marching-ants cut overlay deferred.
6. Fill handle — `interact.ts` hit-tests the bottom-right handle in `onPointerDown` (`fillHandleAt`, ~5px), drags a dominant-axis preview via `opts.selection`, fires `opts.onFill` on grow; `previewer.ts` `handleFill` reads source via `readRangeValues` (added to `clipboardModel.ts`), tiles with pure `projectFill` (`fillModel.ts`, unit-tested), emits `rangefill`; example app `applyFill` → `setRangeValues`. Copy-fill (tile) only; linear/date series detection deferred.
7. Image drop into sheet — `xlsx-app.html` drop IIFE branches image MIME (`image/png|jpeg|gif|webp`) to `insertDroppedImage` → `recalcWorkbook.setImage(sheetName, patch)` anchored as a default box at the active cell → `patchSheetLayout`. Drop-point anchoring + image-size autofit deferred (baseline 5×10 box at active cell).
5. Range paste — `interact.ts` Cmd/Ctrl+V → `opts.onPaste`; `previewer.ts` `handlePaste` reads clipboard (`readClipboard`), parses (#3), emits `rangepaste` with target/values/formulas/source/sourceSheet/sourceRange/cutRange; example app `applyPaste` → internal `copyRange` (keeps formulas) / external `setRangeValues`, clears cut source, reselects pasted region. Single-cell-source → multi-target tiling deferred (paste at top-left only).
