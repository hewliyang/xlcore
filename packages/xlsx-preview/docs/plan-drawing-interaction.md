# Plan: selectable / movable / resizable drawings (charts, images, shapes)

## Problem

Charts and other drawings render onto the grid canvas (`drawDrawings` in
`render.ts`) but are inert. Clicking a chart falls straight through to the cell
underneath: in `interact.ts::onPointerDown`, any click in the cell area runs
`cellAt(...)` and sets a cell selection. Drawings are never hit-tested for
selection, only for hyperlink click-through (`drawingHyperlinkAt`). There is
already an unused `drawingAtPoint()` in `drawingHits.ts` that does the hit-test
we need — it's just not wired in.

Goal: click a drawing to select it (box + 8 handles), drag to move, drag handles
to resize, with live redraw, and round-trip the new anchor back into the
workbook so `save()` persists it.

## Architecture map (read before starting)

- **Render schema drawing** `schema/Drawing.ts`: `{ kind, anchor: DrawingAnchor,
  chart?, image?, shape? }`. **No `id`.** Lives in `sheet.drawings[]`.
- **Anchor (wire)** `schema/DrawingAnchor.ts`: `fromCol/fromColOffEmu`,
  `fromRow/fromRowOffEmu`, `toCol/toColOffEmu`, `toRow/toRowOffEmu`,
  `extEmuCx?/extEmuCy?` (absolute size override). 0-based indices.
- **Pixel rect** `grid.ts::anchorToRect(d, grid)` → `{x,y,w,h}` (logical px).
  `PX_PER_EMU = 1/9525`. When `extEmuCx/Cy > 0`, size is absolute and `to*` is
  ignored; otherwise it's a two-cell anchor.
- **Engine API anchor** `api-schema/ChartAnchor.ts`: same data, different names
  (`fromColumn`, `fromColumnOffsetEmu`, … `toRowOffsetEmu`, offsets are `bigint`).
- **Engine mutation (round-trip, charts):** `worksheet.charts.update(id,
  { anchor })` → WASM `updateChart`. `ChartUpdate.anchor?: AnchorSpec`
  (`string | ChartAnchor`). This is the only kind with anchor-only update.
  `ChartInfo` carries `{ id, name, anchor }`.
- **Worker path (example app uses this):** `WorkerWorkbook` (`src/worker.ts`)
  proxies ops to `editWorker.ts` which calls the WASM handle. No drawing op
  exists yet. `previewer.patchSheetLayout(layout)` merges one re-extracted sheet
  back, preserving scroll/selection. Example wiring lives in
  `examples/xlsx-app.html` (`applyEdit`, `recalcWorkbook`, `engine`).
- **Previewer events:** `emit(name)` dispatches `CustomEvent(name, {detail:
  getState()})`; custom-detail events (`celledit`, `pivotfilter`) dispatch their
  own `CustomEvent` directly. Add new names to `PreviewerEventName` +
  `WorkbookPreviewer` interface.
- **Interact wiring:** `attachInteractivity` gets `InteractOptions` from
  `previewer.ts` (~line 601: `getSheet`, `getLayout`, `activeCell`, `selection`,
  `redraw`, …). Drag state vars: `drag` (resize col/row), `selDrag`, `pointDrag`.
  Move/up handled in `onPointerMove` / `onPointerUp`.

## Identity mapping (render Drawing ↔ engine chart id)

Render `Drawing` has no id. To resolve the engine id for round-trip, match the
**pre-move** drawing anchor against `charts(sheet)` entries by anchor equality
(convert wire→ChartAnchor and compare from/to cols+rows+offsets), with the Nth
chart-kind drawing → Nth `ChartInfo` as index fallback. Encode this in the
`drawingmoved` event payload as `{ kind, drawingIndex, prevAnchor }` so the host
resolves the id; keep the resolver pure + tested.

## Known limitations (visual-only kinds; note as follow-ups, don't block)

- **Images:** `setImage` requires `bytes`; there is no `updateImage`/bytes
  accessor in the WASM bindings → cannot persist a move from JS. Visual move/
  resize only. Follow-up: Rust-side anchor-only `updateImage`/`moveDrawing`.
- **Shapes:** `setShape` needs full `preset` + style; `ShapeInfo` exposes enough
  to reconstruct but it's lossy. Visual-only for now; attempt persistence only
  if a clean `ShapePatch` round-trips in a test, else defer.
- **ChartEx:** no anchor update in bindings → visual-only.

Persistence (round-trip) in this plan targets **charts**. All kinds get visual
move/resize.

## Conventions

- No comments / docstrings (repo rule). Terse changelog one-liners.
- `pnpm test` (builds + vitest), `pnpm check` (typecheck/lint/loc/knip/schema/
  api) must pass. Watch `check:loc` budget — keep new code tight, prefer new
  small modules over bloating `interact.ts`.
- e2e: build an `.xlsx` with a chart and render via the `xlsx-preview` CLI, or
  use `hsx`, to verify selection chrome + moved position.
- Conventional commits, one item per commit.

## Backlog

### A — Selection (visual)

- **A3. Cursor + deselect polish.** Hover over a selected drawing's body →
  `move` cursor; over a handle → resize cursor (set in A-C as handles land).
  Deselect on scroll-to-cell / sheet change. Files: `interact.ts`,
  `previewer.ts`. Verify: `pnpm test`.

### B — Move (visual)

- **B1. Pixel→anchor inverse.** In `grid.ts` add `rectToAnchor(rect, grid,
  template)` that produces a `DrawingAnchor` from a target `{x,y,w,h}`,
  preserving the template's anchor *style*: if `extEmuCx/Cy` present, keep
  absolute size and recompute `from*` cell+offset from `rect.x/y`; else
  recompute both `from*` and `to*` cell+offsets. Must satisfy
  `anchorToRect(rectToAnchor(r)) ≈ r` (±1px). Use binary search over
  `grid.colX/rowY` for the containing cell, remainder→EMU. Files: `grid.ts`,
  `grid.anchor.test.ts` (round-trip property test, incl. both anchor styles +
  off-grid clamping).

- **B2. Drag to move.** New drag mode in `interact.ts` (`drawDrag = { index,
  startPx, startRect }`). On pointer-down inside a selected drawing's body (not
  a handle), capture pointer + start. On move: compute new rect = start + delta
  (clamp to ≥ origin), `sheet.drawings[i].anchor = rectToAnchor(newRect, grid,
  prevAnchor)`, `invalidateGrid()`, `redraw()`. On up: emit move (D1). Files:
  `interact.ts`. Verify: CLI render before/after a programmatic drag shows the
  chart translated; `pnpm test`.

- **B3. Keyboard nudge.** With a drawing selected and no cell active, arrow keys
  move it by 1px (Shift = 10px); `Delete` is **out of scope** (no remove here).
  Files: `interact.ts` (`onKeyDown`). Verify: `pnpm test`.

### C — Resize (visual)

- **C1. Handle hit-test + cursors.** Add handle geometry (8 rects from the
  selection box, ~7px) in `drawingSelection.ts` (shared with A2). In
  `interact.ts`, before body-move, test handles; set the matching resize cursor
  (`nwse`/`nesw`/`ns`/`ew`). Files: `drawingSelection.ts`, `interact.ts`.

- **C2. Drag handle to resize.** Resize drag mode: each handle adjusts the
  corresponding rect edge(s); enforce `MIN` size (reuse the >1px floor in
  `anchorToRect`). Convert via `rectToAnchor` keeping anchor style; live redraw;
  emit resize (D1). Corner handles scale two edges, edge handles one. Files:
  `interact.ts`. Verify: CLI before/after render shows resized chart; round-trip
  test in `grid.anchor.test.ts` covers each handle.

### D — Events + round-trip (charts)

- **D1. Previewer drawing events.** Add `"drawingmoved"` to `PreviewerEventName`
  + `WorkbookPreviewer`. On move/resize commit, dispatch
  `CustomEvent("drawingmoved", { detail: { sheetName, kind, drawingIndex,
  anchor: ChartAnchor, prevAnchor: ChartAnchor } })`. Files: `previewer.ts`,
  `interact.ts` (callback `onDrawingMoved` in `InteractOptions`). Verify:
  `pnpm test` + a vitest asserting the event fires with the new anchor.

- **D2. Anchor conversion helpers.** Pure `wireAnchorToChartAnchor` /
  `chartAnchorToWireAnchor` (handle offset number↔bigint, `extEmuCx/Cy`).
  New `src/anchorConvert.ts` + test. Files: `anchorConvert.ts`,
  `anchorConvert.test.ts`.

- **D3. WorkerWorkbook + editWorker `moveDrawing` op.** Add
  `WorkerWorkbook.moveDrawing({ sheetName, kind, drawingIndex, anchor,
  prevAnchor }): Promise<WorkbookLayout>` and an `editWorker` `"moveDrawing"`
  case: resolve the chart id (pure resolver: anchor-match `charts(sheet)` vs
  `prevAnchor`, index fallback), call `updateChart(sheet, id, { anchor })`,
  return `{ layout: layout({ sheetName }), structure }`. Non-chart kinds → no-op
  layout refetch (visual already applied). Files: `worker.ts`, `editWorker.ts`,
  new pure resolver module + test. Verify: `pnpm test`.

- **D4. Example app wiring.** In `examples/xlsx-app.html`, on
  `previewer.on("drawingmoved", …)` call `recalcWorkbook.moveDrawing(detail)`
  then `previewer.patchSheetLayout(layout)`. Verify: build, open an xlsx with a
  chart in the demo, drag it, hit Download, reopen — chart stays moved.

- **D5. Round-trip test.** Node/vitest: open a fixture xlsx with a chart via the
  `Workbook` API, `charts.update(id, { anchor })`, `save()`, reopen, assert the
  chart anchor changed. Confirms the persistence path independent of UI. Files:
  test under `src/` + a small fixture (or generate via the API). Verify:
  `pnpm test`.

## Shipped

- **A2.** `drawDrawingSelection`/`drawingHandles` in `src/drawingSelection.ts`; `selectedDrawingRect` threaded through `RenderOptions` (computed in `previewer.ts` via `anchorToRect`) and drawn per-pane after `drawDrawings`.
- **A1.** Top-most drawing hit-test (`drawingIndexAtPoint`) + per-sheet `selectedDrawing` state wired through `InteractOptions`; click selects a drawing, Escape/cell click clears it.
