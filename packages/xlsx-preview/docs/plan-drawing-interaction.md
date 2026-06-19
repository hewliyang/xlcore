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

_(complete)_

## Shipped

- **D5.** `src/chartRoundtrip.test.ts` opens `tests/fixtures/charts/chart-bar3d.xlsx` via `Workbook.open` (built `dist/api.js` + `dist/xlcore_wasm_bg.wasm`), moves the first chart's anchor (+3 cols/+5 rows) with `charts.update(id, { anchor })`, `save()`s, reopens, and asserts the reopened anchor's from/to col/row reflect the move — confirming the chart persistence path independent of UI.

- **D4.** `examples/xlsx-app.html` wires `previewer.on("drawingmoved", …)` → `recalcWorkbook.moveDrawing(e.detail)` → `previewer.patchSheetLayout(layout)`, guarded with try/catch + `setStatus`, mirroring the `celledit`/`applyEdit` pattern.

- **D3.** Pure `resolveChartId(charts, prevAnchor, chartOrdinal?)` in `src/drawingResolve.ts` (cell-coord match tolerant to offsets; single match→id, else ordinal fallback into `charts[]`, else null; tests in `drawingResolve.test.ts`); `editWorker` `"moveDrawing"` op resolves chart id + `charts.update(id, { anchor })` for chart kind (non-chart no-op) and returns `{ layout: layout({ sheetName }), structure }`; `WorkerWorkbook.moveDrawing(input)` mirrors `updatePivot` (request→syncShadow→layout).

- **D1.** `"drawingmoved"` added to `PreviewerEventName`; previewer's `onDrawingMoved` builds the detail via pure `buildDrawingMovedDetail(sheetName, kind, index, prevAnchor, anchor)` (in `anchorConvert.ts`, converts both wire anchors→ChartAnchor) and `dispatchEvent`s its own `CustomEvent("drawingmoved", { detail })` (not `emit()`); payload-builder test in `anchorConvert.test.ts`.

- **D2.** Pure `wireAnchorToChartAnchor`/`chartAnchorToWireAnchor` in `src/anchorConvert.ts` (offset number↔bigint, omit zero/absent offsets on the chart side; ChartAnchor has no `extEmuCx/Cy`); round-trip + omission tests in `anchorConvert.test.ts`.

- **C2.** `resizeRect(startRect, handle, dx, dy, min=8)` in `drawingSelection.ts` (left edge {0,6,7}, right {2,3,4}, top {0,1,2}, bottom {4,5,6}; min-size clamp, no inversion); `interact.ts` `resizeDrag` mode tested before body-move drawDrag — pointer-down on a selected drawing's handle captures pointer+startRect+prevAnchor, move applies `rectToAnchor(resizeRect(...))`+invalidate/redraw, up clears and fires `onDrawingMoved`; unit tests in `render.test.ts`.

- **C1.** `drawingHandleAtPoint(rect, x, y, tol)` + `drawingHandleCursor(i)` in `drawingSelection.ts`; `interact.ts` hover section tests handles before body-move and sets `nwse`/`nesw`/`ns`/`ew` cursors (handle order TL,top,TR,right,BR,bottom,BL,left); unit test in `render.test.ts`.

- **B3.** Arrow keys in `onKeyDown` nudge the selected drawing (1px, 10px with Shift) before cell-movement logic: `anchorToRect`→translate (clamped to origin)→`rectToAnchor` keeping anchor template, set `sheet.drawings[i].anchor`, `invalidateGrid()`+`redraw()`, fire `onDrawingMoved` with snapshotted `prevAnchor`; Delete/Backspace out of scope.

- **B2.** `drawDrag` mode in `interact.ts`: pointer-down on an already-selected drawing's body captures pointer + start rect; move sets `sheet.drawings[i].anchor = rectToAnchor(start+delta clamped to origin)` + redraw; up clears and fires optional `onDrawingMoved` (unwired in previewer).

- **B1.** `rectToAnchor(rect, grid, template)` in `grid.ts` inverts `anchorToRect` via binary search over `colX`/`rowY` (remainder→EMU), preserving two-cell vs absolute (`extEmuCx/Cy`) anchor style; round-trip property tests in `grid.anchor.test.ts`.
- **A3.** `move` cursor when hovering a selected drawing's body (`interact.ts`); deselect drawing on sheet change / `selectRange` / `scrollToCell` (`previewer.ts`).
- **A2.** `drawDrawingSelection`/`drawingHandles` in `src/drawingSelection.ts`; `selectedDrawingRect` threaded through `RenderOptions` (computed in `previewer.ts` via `anchorToRect`) and drawn per-pane after `drawDrawings`.
- **A1.** Top-most drawing hit-test (`drawingIndexAtPoint`) + per-sheet `selectedDrawing` state wired through `InteractOptions`; click selects a drawing, Escape/cell click clears it.
