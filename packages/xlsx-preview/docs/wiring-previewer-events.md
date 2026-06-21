# Wiring previewer events to the recalc workbook

The editable previewer and the recalc workbook (`WorkerWorkbook`) are two
independent halves on purpose:

- **The previewer** renders the grid and handles UI gestures (edit a cell, drag
  a chart, paste, fill, delete). It is pure view — it **never** mutates the real
  document. Every gesture is emitted as an *intent* event.
- **The recalc workbook** owns the actual `.xlsx` and recalcs formulas. It is the
  source of truth and the thing `save()` serializes.

Nothing connects them automatically. The host app is responsible for forwarding
each previewer event into the corresponding recalc-workbook mutation and pushing
the returned layout back into the previewer. This is intentional — the host owns
the policy (editable or not, auto-recalc, debounce, undo, error surfacing, and
whether a recalc workbook even exists for read-only previews).

## The footgun

Wiring `celledit` is obvious, so it's easy to ship an "editable" previewer that
handles typing but silently drops everything else — including **chart/image
moves and deletes**. The gesture updates the previewer's on-screen layout, but
because the recalc workbook never hears about it, `recalc.save()` serializes the
*original* anchor and the move is lost on download.

If you support editing at all, wire the **full** event set below, not just
`celledit`. The simplest way to get this right is the
[`bindRecalcWorkbook`](#recommended-bindrecalcworkbook-helper) helper, which
wires the whole set for you.

## Recommended: `bindRecalcWorkbook` helper

Most hosts don't need custom per-event policy — they just want correct editing.
`bindRecalcWorkbook` (exported from `@hewliyang/xlsx-preview/worker`) wires the
full event set in one call and is the correct-by-default path:

```js
import { WorkerWorkbook, bindRecalcWorkbook } from "@hewliyang/xlsx-preview/worker";

const recalc = await WorkerWorkbook.open(bytes, { wasmBinaryUrl, workerUrl });
const binding = bindRecalcWorkbook(previewer, recalc, {
  autoRecalc: () => autoRecalcEnabled,
  onStatus: (msg) => setStatus(msg),
  onChange: ({ event }) => renderState(),
});

// later, e.g. before loading another workbook:
binding.unbind();
```

It handles `celledit`, `rangefill`, `cellclear`, `rangepaste`, `imagepaste`,
`sheetadd`, `drawingmoved`, `drawingdeleted`, and (unless
`resyncOnSheetChange: false`) re-syncs the active sheet on `sheetchange`. Pure
view events (`selectionchange`, `zoomchange`) are left to the host.

Options: `autoRecalc` (`boolean | () => boolean`, default `true`),
`resyncOnSheetChange`, `imageAnchor` (`{ rows, cols }` span for pasted images),
`imageName`, `onChange`, `onStatus`, `onError`.

Hosts that need custom policy can still wire each event by hand — see below.

## Full event → recalc-workbook mapping

| Previewer event   | Recalc method                                   | Push result back via   |
| ----------------- | ----------------------------------------------- | ---------------------- |
| `celledit`        | `applyEdit({ sheetName, address, input, recalc })` | `replaceLayout` / `patchSheetLayout` |
| `rangefill`       | `setRangeValues({ sheetName, ref, values, recalc })` | `patchSheetLayout` |
| `cellclear`       | `clearRange({ sheetName, ref, recalc })`        | `patchSheetLayout`     |
| `rangepaste`      | `pasteCells` / `copyRange` / `moveRange`*       | `patchSheetLayout`     |
| `imagepaste`      | `setImage(sheetName, { anchor, bytes, format, name })` | `patchSheetLayout` |
| `drawingmoved`    | `moveDrawing(detail)`                           | `patchSheetLayout`     |
| `drawingdeleted`  | `removeDrawing(detail)`                          | `patchSheetLayout`     |
| `sheetadd`        | `addSheet(name)`                                | `replaceLayout` + repopulate tabs |

\* `rangepaste` picks the method from `detail`: internal cut → `moveRange`,
internal copy → `copyRange`, otherwise external → `pasteCells`.

`drawingmoved` / `drawingdeleted` `detail` is already shaped for the recalc
methods (`{ sheetName, kind, drawingIndex, anchor, prevAnchor }`) — pass it
through untouched.

> `replaceLayout` swaps the whole workbook layout (use after structural changes
> like adding a sheet). `patchSheetLayout` updates just the active sheet and is
> cheaper — prefer it for cell/drawing edits.

## Manual wiring (custom policy)

If you need per-event control, wire each one yourself:

```js
previewer.on("celledit", (e) => applyEdit(e.detail));
previewer.on("rangefill", (e) => applyFill(e.detail));
previewer.on("cellclear", (e) => applyClear(e.detail));
previewer.on("rangepaste", (e) => applyPaste(e.detail));
previewer.on("imagepaste", (e) => applyImagePaste(e.detail));
previewer.on("sheetadd", (e) => addSheet(e.detail));

// Easy to forget — without these, chart/image reposition + delete are lost on save:
previewer.on("drawingmoved", async (e) => {
  const layout = await recalc.moveDrawing(e.detail);
  previewer.patchSheetLayout(layout);
});
previewer.on("drawingdeleted", async (e) => {
  const layout = await recalc.removeDrawing(e.detail);
  previewer.patchSheetLayout(layout);
});
```

See `examples/xlsx-app.html` for a full reference host. It uses
`bindRecalcWorkbook` for the editing events and wires the pure view events
(`selectionchange`, `zoomchange`, `sheetchange`) itself.
