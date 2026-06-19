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

### 3. Clipboard serialize/parse helpers (pure)
New module (e.g. `clipboardModel.ts`): `serializeRange(layout, sheet, selection)
→ { tsv, html }` and `parseClipboard({ html?, tsv? }) → { values, formulas?,
source: "internal" | "external" }`. TSV uses tab/newline with the usual
quoting/escape rules; HTML emits a `<table>` plus an app-private payload (JSON in
a data attribute or comment) carrying formulas + the source range for fidelity.
Unit-test round-trip incl. embedded tabs/newlines/quotes and the Sheets/Excel
TSV dialect. No wiring yet.

### 4. Range copy / cut (Ctrl/Cmd-C, Ctrl/Cmd-X)
In `interact.ts onKeyDown`, intercept Cmd/Ctrl+C and +X (guard on
`opts.selection`); call a new `opts.onCopy/onCut(selection)`. `previewer.ts`
serializes the selection via #3 and writes the clipboard (`ClipboardItem` with
`text/plain`+`text/html`, fallback to a synthetic copy event). Cut marks the
range (dashed "marching ants" overlay is optional/nice-to-have) and clears it on
the next successful paste. Emit a `rangecopy`/`rangecut` event for the app if
needed. Verify: copy from previewer → paste into Excel/Sheets/TextEdit.

### 5. Range paste (Ctrl/Cmd-V)
Intercept Cmd/Ctrl+V in `onKeyDown` → `opts.onPaste`. `previewer.ts` reads the
clipboard, parses via #3, and emits a `rangepaste` event with the target
top-left (active cell) + 2D values (+ formulas/source). Example app applies it:
internal-source paste → `WorkerWorkbook.copyRange` (preserves formulas/formats);
external → `setRangeValues`. Then `patchSheetLayout` + reselect the pasted
region. Handle single-cell-source → multi-cell-target tiling like Excel
(optional first cut: paste at top-left only). For cut-source, clear the original
after paste. Verify: paste TSV from Sheets; internal copy keeps formulas.

### 6. Fill handle (drag-to-fill)
Draw a small square handle at the selection's bottom-right corner (only in
`editable` mode, only when not editing); hit-test it in `onPointerDown`, and on
drag extend a preview rectangle along the dominant axis. On release, fill the
new cells: copy-fill (repeat source) as the baseline; linear/series detection
(numbers, dates, `Jan/Feb…`, `1,2,3`) as a follow-up — note in the doc if you
defer series. Apply via the #1 ops (`copyRange` for copy-fill, `setRangeValues`
for series). Add a pure `projectFill(sourceValues, sourceRange, targetRange)`
helper with unit tests. Gotcha: keep the drag handling consistent with the
existing drawing-resize pointer flow in `interact.ts`.

### 7. Image drop into sheet
In the example app (and/or previewer), accept an **image** file dropped onto the
canvas (not just whole-workbook files): map the drop point to a cell, build an
`ImagePatch` anchored there (one-cell or sized two-cell anchor), and call the #2
`setImage` op → `patchSheetLayout`. Extend `isSupported`/the drop handler in
`xlsx-app.html` to branch on image MIME (`image/png|jpeg|gif|webp`) vs workbook
files. Verify: drop a PNG, it appears anchored at the drop cell and persists
through `save`/reopen.

### 8. Image paste from clipboard
Handle a clipboard image on Ctrl/Cmd-V (when the payload is an image, not
cells): read the blob, anchor at the active cell, call #2 `setImage`. Shares the
paste entrypoint from #5 — branch on payload type (image blob vs TSV/HTML).
Verify: screenshot to clipboard → paste → image anchored at active cell, round-
trips through save.

## Shipped

1. Worker range ops — `setRangeValues`/`setRangeFormulas`/`copyRange`/`clearRange` on `WorkerWorkbook` + `editWorker.ts`, single-sheet layout, `recalc` flag; Workbook-level round-trip test.
2. Worker image-insert op — `setImage` op on `editWorker.ts` + `WorkerWorkbook.setImage(sheetName, patch)`, single-sheet layout, bytes sent as transferable `ArrayBuffer`; round-trip test.
