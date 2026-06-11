# Triage: API expressiveness vs the OOXML schema space

## Current goal

Refactor the API/binding layer first: move semantics out of the wasm→TS frontend into `xlcore-api`, keep bindings as marshaling-only, and leave per-language wrappers as thin idiom layers. Do this before expanding chart/style/schema coverage.

Question: our curated DTOs (`xlcore-types` → ts-rs → `api-schema/*.ts`) cover a thin
slice of what OOXML can express (e.g. `set_chart` exists but you can't set axis ticks).
Should we instead copy/derive types from what ooxmlsdk gives us? Reference point:
openpyxl's object model.

## Scale of the gap

| Domain | ooxmlsdk structs | openpyxl classes | xlcore-types DTOs |
| --- | --- | --- | --- |
| chart (`c:`) | 386 | 88 | 10 |
| spreadsheetml main | 564 | ~132 (worksheet+styles) | ~100 |
| drawingml main (`a:`) | 552 | ~40 (shapes/text subset) | 3 (shape) |

The read/render path (`xlsx-preview` renderer) consumes far more of the schema than
the write path exposes — e.g. the renderer handles combo charts, dual axes, radar,
stock, `dispUnits`, tick marks, gridline toggles, markers, gapWidth/overlap, and
chartEx, but `ChartPatch` can author none of it.

## Verdict on "copy ooxmlsdk types"

Do **not** expose ooxmlsdk types directly as the public API:

1. **No serde.** ooxmlsdk structs derive `Clone/Debug/Default/PartialEq + SdkType`
   only. Everything crossing the wasm JSON boundary needs a serde mirror anyway, so
   "free types" aren't free.
2. **1:1 XML shape is agent-hostile.** `Scaling.max` is
   `Option<MaxAxisValue { val: DoubleValue }>` behind a `Box`; choice enums,
   `extLst`, and required-but-defaultable children leak everywhere. SpreadJS/EPPlus
   wrap the same schema for the same reason.
3. **openpyxl doesn't do this either.** Its win is *naming discipline*, not raw
   schema exposure: every class mirrors the OOXML `tagname` and `__elements__`
   order, but `CT_Double/@val` wrappers are flattened to plain floats
   (`majorUnit = NestedFloat()`), defaults are filled in, and pythonic aliases
   (`number_format` → `numFmt`) sit on top. The schema *is* the documentation; the
   API stays flat.

What we **should** copy from ooxmlsdk is its field inventory: when designing a
patch type for domain X, walk the corresponding `CT_*` struct and make a deliberate
include/exclude call per field. That converts "are we expressive enough?" from a
feeling into a checklist. The 30 `SdkEnum`s (tick marks, crosses, marker styles,
built-in units, …) should be transliterated 1:1 into our serde enums — that part is
mechanical and lossless.

## Pattern to adopt (openpyxl-style)

- One DTO per OOXML element, same name where sane (`Scaling`, not `AxisBounds`).
- Flatten `val`-wrapper children to scalars.
- Group exactly as the schema groups (axis has `scaling`, chart has `axes`), so a
  user reading ECMA-376 or an Excel forum post can map concepts directly.
- Keep `Info`/`Patch` symmetry; patches partial, infos total.
- Per-domain coverage note in the DTO doc comment listing intentionally-excluded
  schema fields (`extLst`, `pictureOptions`, …) so gaps are decisions, not accidents.

### Concrete example: chart axes (the reported gap)

ooxmlsdk `CT_ValAx`/`CT_CatAx` + openpyxl `_BaseAxis` distilled:

```rust
pub struct ChartAxisPatch {
    pub title: Option<String>,
    pub hidden: Option<bool>,              // c:delete
    pub min: Option<f64>,                  // c:scaling/c:min
    pub max: Option<f64>,                  // c:scaling/c:max
    pub log_base: Option<f64>,             // c:scaling/c:logBase (2..=1000)
    pub reversed: Option<bool>,            // c:scaling/c:orientation = maxMin
    pub major_unit: Option<f64>,           // valAx only
    pub minor_unit: Option<f64>,
    pub major_gridlines: Option<bool>,
    pub minor_gridlines: Option<bool>,
    pub major_tick_mark: Option<TickMark>, // cross|inside|outside|none
    pub minor_tick_mark: Option<TickMark>,
    pub tick_label_position: Option<TickLabelPosition>, // high|low|nextTo|none
    pub number_format: Option<String>,     // c:numFmt
    pub cross_between: Option<CrossBetween>, // between|midCat (valAx)
    pub crosses_at: Option<f64>,
    pub display_units: Option<DisplayUnits>, // builtin enum or custom f64
    pub label_rotation: Option<i32>,       // txPr bodyPr rot
}
```

`ChartPatch` grows `category_axis: Option<ChartAxisPatch>`,
`value_axis: Option<ChartAxisPatch>`, `secondary_value_axis: Option<ChartAxisPatch>`
(the existing `category_axis_title`/`value_axis_title` become sugar/deprecated).

## Gap inventory (write surface)

### Charts — P0 (renderer already understands most of these)

- Axis object: min/max/log/reversed, units, tick marks, tick label pos, gridlines,
  numFmt, crossBetween/crossesAt, dispUnits, hidden axis. **Nothing exposed today.**
- Combo charts / secondary axis: renderer renders them; `set_chart` can't author
  them. Needs `ChartSeriesPatch.axis: primary|secondary` + per-series `kind`.
- Series styling: marker (style/size), line width/dash, `smooth`, `gapWidth`,
  `overlap`, `varyColors`, `invertIfNegative`.
- Pie/doughnut: `firstSliceAngle`, `holeSize`, per-point `explosion`.
- Per-point fills (`c:dPt`) — required for the waterfall-via-noFill idiom we
  already render.
- `dispBlanksAs` (span|gap|zero).
- Kinds: radar + stock render today but aren't in `ChartKind`.
- Data label `numFmt`.

### Charts — P2

Trendlines, error bars, data table, 3D variants, surface, ofPie, manual layout,
plot-area/legend spPr+fonts, chartStyle/colorStyle companion parts, chartEx
authoring, per-point data labels.

### Styles — P1

- Cell protection `locked`/`hidden` — sheet protection is half-useless without it.
- `FillPatch`: pattern type + fg/bg, gradient fills (schema: `CT_PatternFill`,
  `CT_GradientFill`).
- `FontPatch`: `vertAlign` (sub/superscript), `family`, `scheme`.
- `BorderPatch`: `diagonal` + `diagonalUp`/`diagonalDown`.
- `AlignmentPatch`: `shrinkToFit`, `justifyLastLine`, `readingOrder`.
- Named styles / `cellStyles` authoring — P2.

### Worksheet — P1

- Row/column outline grouping (`outlineLevel`, collapsed) — gutter renderer exists.
- Tab color, zoom, `showZeros`, `rightToLeft`, default row height / col width.
- Print area + print titles (defined-name backed) and manual page breaks —
  page_setup.rs covers everything except these.
- Rich text runs in cells (`CT_RElt`) — P2, but agents ask for it.

### Already adequate

Tables, autofilter, data validation, CF (incl. colorScale/dataBar/iconSet),
sparklines, comments/threaded notes, hyperlinks, merges, defined names, page
setup/margins/header-footer, protection (minus cell-level), images, pivots (v1
scope), search, structural ops.

## Process recommendation

> Status: step 1 (schema-diff script) landed as `scripts/schema_diff.py`.


1. **Schema-diff script** — *done*: `scripts/schema_diff.py <SdkStruct> [DtoStruct]`
   parses an `ooxmlsdk` `CT_*` struct's fields (resolving the ooxmlsdk version from
   `Cargo.lock`, preferring the canonical `schemas_openxmlformats_org` definition
   when a name is duplicated) and diffs them against an `xlcore-types` DTO, emitting
   a markdown coverage table (field, xml tag, optional, choice, covered) plus an
   `unmodeled`/`dto-only` summary. Run it when touching a domain; paste the table
   into the DTO doc comment. E.g. `ValueAxis` vs `ChartPatch` reports covered 1/20.
2. **Transliterate sdk enums verbatim** when a domain is opened up.
3. **Escape hatch** (openpyxl ≈ lxml access): raw part XML get/set on `Workbook`
   for anything we haven't modeled yet, so users are never hard-blocked.
4. Land P0 chart axis + combo/series styling first — it's the densest cluster of
   "renderer reads it, API can't write it" asymmetry.

## Code organization + API design vs openpyxl

Verdict: the **architecture** is more principled than openpyxl's; the **surface
conventions** are less principled. Different layers, different grades.

### Where we're ahead

- **Layering.** Rust owns one mutation path; DTOs are generated once (ts-rs); TS
  is a thin façade. openpyxl smears logic across descriptors, `Serialisable`
  metaclass magic, and reader/writer modules — and famously drops
  charts/images/pivots on round-trip. Preserve-by-default is a categorically
  better foundation.
- **Collection pattern.** `SheetScopedCollection` + `qref()` qualification +
  `Info`/`Patch` symmetry is consistent and agent-friendly. openpyxl's
  equivalents are ad-hoc (`ws.add_chart(chart, anchor)`, `ws.merge_cells(str)`,
  dict-like `ws.tables` — three idioms).
- **Patch objects over property assignment.** `cell.font = Font(...)` doesn't
  survive an RPC boundary; one-call patches diff better and suit agents.

### Where we're unprincipled

1. **Verb soup.** `merges.add` / `hyperlinks.set` / `threadedNotes.add` +
   `removeThread` / `conditionalFormats.set` + `clear` / `autoFilter.set` +
   `setColumn` + `setColumnValues`. Naming drift: `AutoFilterApi`,
   `SheetPageSetupApi`, `SheetFreeze`, `WorkbookPropertiesApi` — the `Api`
   suffix appears on ~half. Fix: pick `list/get/set/add/remove/clear`
   semantics, write them down, audit every collection.
2. **`sheet` inside some patches, argument in others.** `ChartPatch.sheet`,
   `ShapePatch.sheet`, `PivotPatch.sheet` vs `setComment(qref, patch)`. The TS
   layer betrays it — `Omit<ShapePatch, "sheet">` then re-inject. Fix in
   `xlcore-types`: sheet-scoped patches lose their `sheet` field; wasm fns take
   sheet as first arg uniformly.
3. **`ChartCollection.update` is remove+set in TS** (`chartInfoToPatch` →
   `removeChart` → `setChart` → manual rollback). Three problems: not atomic
   (rollback can fail; success regenerates the rId so stored chart ids go
   stale); **violates the preservation principle** — any chart XML not modeled
   by `ChartPatch` (~95% of the schema, per above) is destroyed by an update
   touching one field; and it's mutation logic living in TS when Rust is
   supposed to own mutation. Fix: `update_chart` in Rust, mutating the existing
   `chart<n>.xml` in place, leaving unmodeled elements untouched. **Do this
   before growing `ChartPatch`** — the hand-copied field list in
   `chartInfoToPatch` drifts on every DTO addition.
4. **Triple hand-written glue.** Every feature = Rust facade fn + wasm binding
   (107 fns of serde_wasm_bindgen boilerplate) + TS collection method (pure
   forwarding + `as T` cast). Mechanical and hand-maintained. A declarative
   method table → codegen for the wasm+TS layers would eliminate the drift
   class entirely. openpyxl's metaclass is the same instinct at runtime;
   codegen is the better version of it.
5. **Worksheet identity is fake.** `Worksheet` wraps a throwaway
   `{current: name}` ref; `wb.sheet("X")` twice gives two objects, and
   `rename()` on one strands the other (and any stored `Range`s). openpyxl's
   workbook owns its worksheet objects — same name, same object. Fix:
   `Workbook` caches `Worksheet` per stable `SheetInfo.id`; rename updates the
   shared ref.
6. **No bulk-data idiom.** openpyxl's most-used method is `ws.append(row)`.
   `setValues(matrix)` exists but there's no append/iter-rows ergonomic; agents
   reconstruct ranges manually.

### Priority

#3 is correctness — land before expanding `ChartPatch`. #1/#2 are breaking
renames, cheapest while the user count is ~1. #4 pays off across every future
domain. #5/#6 are nice-to-haves.

## Where to enforce the structure (Rust vs binding) — pyo3/napi readiness

Test: could someone write a pyo3 binding without reading `api-collections.ts`?
Today no — semantics have leaked into the TS frontend:

- `charts.update` / `pivots.update` merge semantics + rollback
- dataBar `min`/`max` defaulting
- autoFilter `setColumnValues/Top10/Custom` sugar + friendly `criteria.kind` error
- `anchorA1` string→`ChartAnchor` parsing (no Rust counterpart at all)
- `qref` sheet-qualification of unqualified refs
- `recalculate({errorsOnly})` filtering, `search` defaults
- matrix-shape validation duplicated in both layers (two error behaviors)

Each is a re-implement-and-drift liability per future binding.

### Three layers

1. **`xlcore-api` (Rust): all semantics.** Anything that changes *what happens*:
   defaults, validation, ref qualification, merge-update, error messages.
   Migrate the list above down; facade methods take `sheet: &str` + possibly
   unqualified `ref` and qualify internally; `anchor` accepts A1 strings.
2. **Bindings (wasm / pyo3 / napi): marshaling only.** serde↔JsValue/PyObject,
   `ApiError`→JsValue/PyErr. Zero branching logic. This is the layer the
   method-table codegen (#4 above) should emit — one table, N bindings.
   `xlcore-types` as plain serde DTOs already makes types portable (ts-rs
   today; a pydantic/stub emitter slots in the same way).
3. **Per-language fluent wrapper: idiom only.** `Workbook/Worksheet/Range`
   classes, chaining, properties. Stays per-language and thin — Python wants
   `ws["A1"]`/snake_case/iterators, JS wants chaining/camelCase. Do **not**
   hoist the object model into Rust: exported-class graphs (sheet wrappers
   borrowing the workbook handle) fight the borrow checker in every binding
   for zero semantic gain. A wrapper that contains no decisions is a weekend
   rewrite per language, which is the goal.

The current TS collections are already Layer-3 shaped; fixing the ~8 semantic
intrusions makes the structure port cleanly.
