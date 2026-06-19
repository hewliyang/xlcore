# Parity: reactive editing (SpreadJS-style)

Goal: sub-50ms edit→paint roundtrip on large files by doing only the work an
edit actually invalidates, instead of refreshing the whole workbook.

## Triage: what one keystroke costs today

`editWorker.applyEdit` → `Workbook.recalculate()` → `Workbook.layout({})`. Per
edit, for the **entire workbook**:

1. Re-harvest the OOXML DOM — shared strings + every sheet/cell
   (`xlcore-bridge/src/lib.rs:34` `recalculate_doc`).
2. Rebuild a fresh engine from scratch — `WorkbookEngine::new` + `load_engine`
   re-inserts every cell/formula/defined-name (`lib.rs:86`). Engine is **not**
   resident; the handle only holds `self.doc` (`xlcore-api/src/lib.rs:181`).
3. Full recalc — `engine.evaluate()` clears all results and recomputes every
   cell (ironcalc `model.rs:1886`).
4. Writeback cached values into the DOM, mark current.
5. `extract_doc` re-extracts the **whole** layout (`recalculate_layout_doc`);
   `editWorker` passes `layout({})` even though `sheet_index`/`sheet_name`
   exist (`api.ts:335`).
6. Serialize full layout across the worker boundary → main thread full redraw.

So ~4 full-workbook passes + full serialize + full repaint per cell. Formula
math is a minority of the ~500ms; harvest/rebuild/extract dominate.

Neither engine keeps a reverse-dependency index or dirty set; `ooxmlsdk-formula`
builds a `DependencyGraph` but never uses it for ordering. ironcalc's
`evaluate_cell` is lazy + memoized with `#CIRC!` detection (`model.rs:801`) — a
good base for incrementality once the engine is resident.

## Shipped

### 1. Resident engine
`Workbook` holds `engine: Option<ResidentEngine>`, built lazily on first
recalc and reused after. `set_value`/`set_formula` route the single mutation
into it (`set_input`/`set_formula`); every other mutation invalidates it
(`engine = None`) for a clean rebuild. DOM stays source-of-truth for `save`.

### 2. Scope layout to active sheet
`editWorker.applyEdit` now returns `layout({ sheetName })` (single sheet, full
shared pools); the `layout` op accepts `{ sheetName }` for on-demand refetch.

### 3. Single-sheet patch + partial redraw
`previewer.patchSheetLayout` merges a single-sheet layout into the resident
multi-sheet layout (swaps the named sheet + shared pools, re-decodes only that
sheet, `scheduleDraw` without re-rendering tabs/interactivity); scroll +
selection preserved. Falls back to `replaceLayout` if the sheet is absent.
Cross-sheet staleness fixed by refetching the target sheet on switch.

## Actionable items (ordered by leverage)

### 4. Incremental recalc (do last, after #1)
Only recompute the edited cell's transitive dependents.
- [ ] Build a reverse-dep index (precedent→dependents) in the engine.
- [ ] On edit: mark cell dirty, propagate, re-evaluate only that closure
      (don't `cells.clear()`); reuse memoized values for clean cells.
- [ ] Handle volatiles (`NOW`/`RAND`/`OFFSET`/`INDIRECT`) as always-dirty.

### 5. Coalesce + debounce
- [ ] Batch rapid edits (typing) into one recalc; expose `pauseEvaluation`
      (ironcalc `user_model` already has it).
- [ ] Debounce layout extraction to animation frames.

## Sequencing

Ship #1 + #2 + #3 and measure first — likely gets interactive latency without
touching the recalc algorithm. Pay for #4's complexity (correctness of dirty
propagation, cycles, volatiles) only if eval is still the bottleneck after that.
