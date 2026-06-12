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
  numFmt, crossBetween/crossesAt, dispUnits, hidden axis. **— done** (except
  label rotation, deferred to P2): `ChartAxisPatch` is on
  `ChartPatch`/`ChartUpdate`/`ChartInfo` as `category_axis`/`value_axis`, with
  build + read + in-place update. `min`/`max`/`major_unit`/`major_gridlines`/
  `number_format` are renderer-visible; the rest round-trips for Excel.
  `display_units` (`ChartAxisPatch.display_units`, value axis only,
  `DisplayUnits` = builtin `ST_BuiltInUnit` name | custom divisor, sdk
  `DisplayUnits` distilled) builds `c:dispUnits` (+ empty `c:dispUnitsLbl` so the
  builtin unit name renders as a label band), round-trips, updates in place, and
  is renderer-visible (column chart scaled to "Millions" — axis labels 0/5/10/15
  + label band verified e2e).
- Combo charts / secondary axis: **— done**. `ChartSeriesPatch`/`ChartSeriesInfo`
  carry `kind: Option<ChartKind>` (per-series type override, cartesian-only) and
  `axis: Option<ChartAxisGroup>` (`primary`/`secondary`, sdk transliteration).
  Cartesian series are grouped by (effective kind, axis) into multiple
  `c:barChart`/`c:lineChart`/`c:areaChart` plot groups; any secondary series
  synthesizes a right-hand `c:valAx` + deleted secondary `c:catAx`. Builds via
  `build_cartesian_plot_charts`, round-trips (read tags each series with its
  group kind/axis; simple single-group charts stay clean), updates in place, and
  is renderer-visible (column + line on dual axes verified e2e).
- Series styling: marker (style/size), line width/dash, `smooth`,
  `varyColors`, `invertIfNegative`. **— done.** `gapWidth`/`overlap` + marker + `smooth`
  + line width/dash + `varyColors` + `invertIfNegative`; `ChartSeriesPatch.marker` (`ChartMarker { style, size }`
  + `MarkerStyle` enum, sdk `ST_MarkerStyle` transliterated) builds `c:marker` on
  line/scatter series, round-trips, and is renderer-visible. `ChartSeriesPatch.smooth`
  (`c:smooth`) builds on line/scatter series, round-trips, updates in place, and
  on scatter charts any smoothed series flips the chart's `c:scatterStyle` to
  `smoothMarker` — renderer-visible (smooth scatter draws curved splines verified
  e2e). `ChartSeriesPatch.line` (`ChartLine { width_emu, dash, none }` +
  `LineDash` enum, sdk `Outline`/`ST_PresetLineDashVal` distilled) builds
  `spPr/a:ln` (`@w` + `a:prstDash`, or `a:noFill` for markers-only) on every
  series kind, round-trips, updates in place, and `width_emu`/`dash` are
  renderer-visible (thick red line + dashed blue line on a line chart verified
  e2e); `none` round-trips for Excel only. `ChartPatch.vary_colors`
  (`c:varyColors`, chart-level; defaults pie/doughnut/bubble true, others false)
  builds across every plot group, round-trips, and updates in place;
  `ChartSeriesPatch.invert_if_negative` (`c:invertIfNegative`, sdk `CT_BarSer`/
  `CT_BubbleSer` distilled; bar/column + bubble series) builds, round-trips, and
  survives in-place update. Both round-trip for Excel only (renderer reads
  neither yet).
- Pie/doughnut: **— `holeSize` + `firstSliceAngle` done**. `ChartPatch.hole_size`
  (10..=90, `c:holeSize`, doughnut only) and `ChartPatch.first_slice_angle`
  (0..=360, `c:firstSliceAng`, pie/doughnut) on
  `ChartPatch`/`ChartUpdate`/`ChartInfo`; build (default hole 50), read,
  in-place update, validated in Rust, and both renderer-visible (doughnut hole
  diameter + slice rotation verified e2e). Per-point `explosion`
  (`ChartDataPoint.explosion`, `c:explosion/@val`, 0..=400 percent of radius)
  builds on every series kind, round-trips, survives in-place update, and is
  renderer-visible (pie slices offset outward — North +30% / East +15% verified
  e2e). **— done.**
- Per-point fills (`c:dPt`): **— done**. `ChartSeriesPatch.data_points`
  (`ChartDataPoint { index, fill, explosion }`, sdk `CT_DPt` distilled) builds
  `c:dPt` with a solid `RRGGBB`/`AARRGGBB` fill or `fill: "none"` → `a:noFill`
  for the waterfall idiom, on every series kind (bar/line/area/pie/doughnut/
  scatter/bubble/radar). Round-trips (reads solidFill + noFill back), survives
  in-place update, and is renderer-visible (column chart with 4 distinct
  per-point fills verified e2e). Remaining dPt fields (`invertIfNegative`,
  per-point marker, gradient/pattern fills) preserved-on-update, deferred.
- `dispBlanksAs` (span|gap|zero): **— done**. `ChartPatch.disp_blanks_as`
  (`DispBlanksAs` enum, sdk `ST_DispBlanksAs` transliterated) on
  `ChartPatch`/`ChartUpdate`/`ChartInfo`; builds `c:dispBlanksAs` (default `Gap`),
  round-trips, and updates the chart-root element in place. Round-trip-only for
  Excel (renderer's read-side chart schema doesn't consume it yet).
- Kinds: **— radar + stock done**. Radar: `ChartKind::Radar` + `RadarStyle` enum
  (sdk `ST_RadarStyle`); builds `c:radarChart`, round-trips, updates in place,
  renderer-visible e2e. Stock: `ChartKind::Stock` builds `c:stockChart` from
  3..=6 line series (high-low-close, open-high-low-close, or volume + OHLC),
  with the connecting line suppressed (`a:ln/a:noFill`) so points show as
  markers. `ChartPatch.hi_low_lines`/`up_down_bars`/`drop_lines`
  (`c:hiLowLines`/`c:upDownBars`/`c:dropLines`, sdk `CT_StockChart` distilled)
  gate the overlays — hi/low lines default on, up/down bars default on for
  open-high-low-close (4+ series), drop lines default off. Round-trips, updates
  in place, and is renderer-visible (OHLC chart with hi-low whiskers + white/black
  up/down bars verified e2e). Remaining `CT_StockChart` fields (`extLst`)
  preserved-on-update.
- Data label `numFmt`. **— done**: `ChartDataLabels.number_format` (`c:numFmt`)
  builds, round-trips, updates in place, and is renderer-visible (pie labels
  format `0.42` → `42.0%` via `0.0%` verified e2e).

### Charts — P2

Trendlines, error bars, data table, 3D variants, surface, ofPie, manual layout,
plot-area/legend spPr+fonts, chartStyle/colorStyle companion parts, chartEx
authoring, per-point data labels.

### Styles — P1

- Cell protection `locked`/`hidden` — **done**: `StylePatch.protection`
  (`ProtectionPatch { locked, hidden }`, sdk `CT_CellProtection` 1:1) builds
  `x:protection` + sets `@applyProtection` on the cellXf (and on dxfs), deduped
  via the cell-format signature, and round-trips. Write-only for Excel (renderer
  doesn't consume cell-level protection).
- `FillPatch`: pattern type + fg/bg — **done**: `FillPatch.pattern`
  (`PatternType`, sdk `ST_PatternType` transliterated 1:1), `foreground`,
  `background`; builds `x:patternFill` with the chosen `@patternType` + fg/bg
  colors (solid stays the default when only `color`/`foreground` is set, with the
  legacy indexed-64 bg), deduped via the fill signature, round-trips, and is
  renderer-visible (all 16 pattern tiles verified e2e). Gradient fills
  (`CT_GradientFill`) — **done**: `FillPatch.gradient` (`GradientFillPatch
  { kind, degree, left, right, top, bottom, stops }` + `GradientStopPatch
  { position, color }` + `GradientType` = linear|path, sdk `CT_GradientFill`/
  `CT_GradientStop`/`ST_GradientType` distilled) builds `x:gradientFill` with
  `@type`/`@degree` (linear) or `@left`/`@right`/`@top`/`@bottom` convergence
  (path) + `x:stop`/`@position`/`x:color`, validates stop positions 0..=1,
  dedupes via the gradient-aware fill signature, round-trips, and is
  renderer-visible (45° linear gold→navy + radial path red→white verified e2e).
- `FontPatch`: `vertAlign` (sub/superscript), `family`, `scheme` — **done**:
  `FontPatch.vert_align` (`VertAlign` = baseline|superscript|subscript, sdk
  `ST_VerticalAlignRun` transliterated), `family` (`u32` 0..=5, `font/@family`),
  `scheme` (`FontScheme` = none|major|minor, sdk `ST_FontScheme`) build
  `font/vertAlign`+`family`+`scheme`, deduped via the font signature, round-trip,
  and `vert_align` is renderer-visible (superscript/subscript runs raised/lowered
  verified e2e); `family`/`scheme` round-trip for Excel.
- `BorderPatch`: `diagonal` + `diagonalUp`/`diagonalDown` — **done**:
  `BorderPatch.diagonal` (`BorderLinePatch`), `diagonal_up`, `diagonal_down` build
  `x:border` `<diagonal>` + the `@diagonalUp`/`@diagonalDown` attrs, deduped via the
  border signature, round-trip, and are renderer-visible (blue `\`, red thick `/`,
  green X verified e2e).
- `AlignmentPatch`: `shrinkToFit`, `justifyLastLine`, `readingOrder` — **done**:
  `AlignmentPatch.shrink_to_fit`/`justify_last_line` (bool, sdk `CT_CellAlignment`
  1:1) + `reading_order` (`ReadingOrder` = context|leftToRight|rightToLeft, sdk
  `@readingOrder` 0/1/2) build `x:alignment` `@shrinkToFit`/`@justifyLastLine`/
  `@readingOrder`, deduped via the alignment signature, round-trip, and surface in
  the layout (`CellFormat.shrink_to_fit`/`justify_last_line`/`reading_order`).
  Round-trip-only for Excel (renderer doesn't consume them yet).
- Named styles / `cellStyles` authoring — P2.

### Worksheet — P1

- Row/column outline grouping (`outlineLevel`, collapsed) — **done**:
  `group_rows`/`group_columns(sheet, start, end, level, collapsed)` in
  `xlcore-api` set `row/@outlineLevel`/`col/@outlineLevel` over the range, sync
  `sheetFormatPr/@outlineLevelRow`/`@outlineLevelCol` to the max, and (when
  collapsed) hide the grouped rows/cols + mark the summary row/col `@collapsed`
  (summary side read from `outlinePr/@summaryBelow`/`@summaryRight`, default
  below/right); `level: 0` ungroups. TS `groupRows`/`groupColumns` +
  `ungroupRows`/`ungroupColumns` forward only. Round-trips and is
  renderer-visible (gutter brackets + collapse buttons verified e2e).
- Tab color, zoom, `showZeros`, `rightToLeft`, default row height / col width —
  **done**: `SheetPropertiesPatch`/`SheetProperties` + `sheet_properties`/
  `set_sheet_properties(sheet, patch)` in `xlcore-api` set `sheetPr/tabColor`,
  `sheetView/@zoomScale`/`@showZeros`/`@rightToLeft`, and `sheetFormatPr/
  @defaultRowHeight`/`@defaultColWidth` (zoom validated 10..=400, dimensions
  non-negative; tab color via `parse_color` → AARRGGBB). Round-trips, updates in
  place, exposed as `worksheet.properties.get`/`.set`. `tab_color` +
  `default_row_height`/`default_col_width` are renderer-visible (taller rows /
  wider cols verified e2e); zoom/showZeros/rightToLeft round-trip for Excel.
- Print area + print titles (defined-name backed) and manual page breaks —
  **done**: `SheetPageSetupPatch`/`SheetPageSetup` carry `print_area` (A1 range,
  multi-area comma-ok), `print_title_rows`/`print_title_columns`, and
  `row_breaks`/`column_breaks` (1-based break-after indices). Print area/titles
  build sheet-scoped `_xlnm.Print_Area`/`_xlnm.Print_Titles` defined names
  (absolutized `'Sheet'!$A$1:$D$10`, titles as `cols,rows`); breaks build
  `x:rowBreaks`/`x:colBreaks` (`<brk man=1>`, full row/col span). Empty
  string/`vec![]` clears each; titles merge component-wise. All round-trip,
  update in place, and are read back de-absolutized. Round-trip-only for Excel
  (renderer consumes neither).
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
   (DTO lookup scans every `xlcore-types/src/*.rs` module — it was `lib.rs`-only and
   silently broken for every split-out DTO until fixed alongside the anchor work.)

   The diff now flattens one level (a struct/`Vec<choice>` field expands and matches
   its leaf tags, e.g. `scaling` → flattened via min/max/logBase/orientation), and
   reads declared exclusions: per-field statuses are covered / flattened (n/m) /
   derived / excluded / MISSING, where only MISSING is a gap. Deferrals are deliberate
   declarations, not a silent global skip — either a `schema-excluded:` line in the
   DTO doc comment (e.g. `spPr, txPr` on `ChartAxisPatch`) or a per-pair
   `derived`/`excluded`/`aliases`/`ns` entry in `scripts/schema_coverage.toml`
   (only `extLst` is intrinsic). The manifest lists every (SdkStruct, DtoStruct)
   pair we have opened up; `scripts/schema_diff.py --check` walks all of them and
   exits non-zero listing any undeclared MISSING field. **Run `--check` after
   touching any opened-up domain**; add the new field to its DTO/patch or declare
   the deferral (each entry is a decision). A single-pair invocation reuses the
   manifest's declarations for that pair, so the table shows the real statuses.
2. **Transliterate sdk enums verbatim** when a domain is opened up.
3. **Escape hatch** (openpyxl ≈ lxml access): raw part XML get/set on `Workbook`
   for anything we haven't modeled yet, so users are never hard-blocked. **—
   done**: `Workbook::part_names`/`get_part_xml`/`set_part_xml`/`remove_part_xml`
   in `xlcore-api` list/read/author/delete OPC parts by path (serialize the
   modeled graph to package bytes, edit the zip, re-open). Unmodeled parts
   (e.g. `customXml/item1.xml`) round-trip verbatim through save/open; existing
   modeled parts re-parse on set. wasm `partNames`/`getPartXml`/`setPartXml`/
   `removePartXml` + TS `Workbook.partNames`/`getPart`/`setPart`/`removePart`
   forward only. Verified e2e (set unmodeled part → save → reopen preserves it).
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

1. *(done)* **Verb soup.** `merges.add` / `hyperlinks.set` / `threadedNotes.add` +
   `removeThread` / `conditionalFormats.set` + `clear` / `autoFilter.set` +
   `setColumn` + `setColumnValues`. Naming drift: `AutoFilterApi`,
   `SheetPageSetupApi`, `SheetFreeze`, `WorkbookPropertiesApi` — the `Api`
   suffix appears on ~half. Fix: pick `list/get/set/add/remove/clear`
   semantics, write them down, audit every collection.
   **Resolved**: canonical verbs (`list/get/set/add/remove/clear`) + the
   sanctioned domain-verb exceptions (`update`/`preview`/`reply`/autofilter
   column ops) are written down in `docs/api-conventions.md`, and every
   collection was audited against it. The only method drifter,
   `ThreadedNotesCollection.removeThread`, is renamed `remove`. Class names are
   normalized to two cardinality-keyed suffixes — `<Concept>Collection` (sheet),
   `Workbook<Concept>` (workbook), `<Concept>Accessor` (singleton) — dropping the
   `Api` suffix entirely: `AutoFilterApi`→`AutoFilterAccessor`,
   `SheetFreeze`→`SheetFreezeAccessor`, `SheetPageSetupApi`→`SheetPageSetupAccessor`,
   `SheetPropertiesApi`→`SheetPropertiesAccessor`,
   `SheetProtection`→`SheetProtectionAccessor`,
   `WorkbookPropertiesApi`→`WorkbookPropertiesAccessor`,
   `CalcPropertiesApi`→`CalcPropertiesAccessor`,
   `WorkbookProtection`→`WorkbookProtectionAccessor`,
   `DefinedNamesCollection`→`WorkbookDefinedNames` (the `Accessor` suffix also
   resolves the DTO name clashes on `SheetProperties`/`WorkbookProperties`/
   `CalcProperties`/`SheetPageSetup`). Bindings stay marshaling-only.
2. *(done)* **`sheet` inside some patches, argument in others.** `ChartPatch.sheet`,
   `ShapePatch.sheet`, `PivotPatch.sheet` vs `setComment(qref, patch)`. The TS
   layer betrayed it — `Omit<ShapePatch, "sheet">` then re-inject.
   **Resolved**: `ChartPatch`/`ImagePatch`/`ShapePatch`/`PivotPatch`/
   `SparklineGroupPatch` dropped their `sheet` field; the facade fns
   (`set_chart`/`set_image`/`set_shape`/`set_pivot`/`pivot_preview`/
   `set_sparkline_group`) take `sheet` as the first arg uniformly, matching the
   existing `update_chart`/`update_pivot` shape. The TS `Omit<…, "sheet">` +
   re-inject plumbing is deleted; `Info` types still carry `sheet`.
3. *(done)* **`ChartCollection.update` is remove+set in TS** (`chartInfoToPatch` →
   `removeChart` → `setChart` → manual rollback). Three problems: not atomic
   (rollback can fail; success regenerates the rId so stored chart ids go
   stale); **violates the preservation principle** — any chart XML not modeled
   by `ChartPatch` (~95% of the schema, per above) is destroyed by an update
   touching one field; and it's mutation logic living in TS when Rust is
   supposed to own mutation. Fix: `update_chart` in Rust, mutating the existing
   `chart<n>.xml` in place, leaving unmodeled elements untouched. **Do this
   before growing `ChartPatch`** — the hand-copied field list in
   `chartInfoToPatch` drifts on every DTO addition.
   **Resolved**: `Workbook::update_chart` + the `ChartUpdate` DTO mutate
   `chart<n>.xml` in place (stable rId, atomic, unmodeled XML preserved). The TS
   `ChartCollection.update` is now pure forwarding; `chartInfoToPatch` deleted.
   `kind` is no longer updatable via `update` (remove + set to change type).
4. *(done, TS layer noted as follow-up)* **Triple hand-written glue.** Every
   feature = Rust facade fn + wasm binding (107 fns of serde_wasm_bindgen
   boilerplate) + TS collection method (pure forwarding + `as T` cast).
   Mechanical and hand-maintained. **Resolved (wasm)**: the wasm binding layer
   is now generated from a single declarative method table — the `api_methods!`
   `macro_rules!` in `xlcore-wasm/src/lib.rs`. Each row is
   `{ rust_name [as "jsName"] ( args ) -> ret }` (arg kinds `s`/`os`/`u32`/`u8`/
   `usize`/`f64`/`bool`/`de Name: Ty`/`deopt Name: Ty`; ret `json`/`unit`/`bool`),
   and a TT-muncher emits a marshaling-only `#[wasm_bindgen] impl` method per
   row (serde↔JsValue, `api_err_to_js`, zero branching). 100 of the 107 binding
   fns collapsed into the table; the 7 genuinely-custom ones (constructors,
   `set_value`/`set_range_values`/`append_rows`/`set_range_formulas` JS-array
   walks, `set_sheet_visibility` string match, slice-returning `warnings`/
   `take_warnings`, name-mismatched `save`) stay hand-written next to it. The
   generated `xlcore_wasm.d.ts` is byte-identical to the prior hand-written
   output (verified by diff); `cargo test --workspace` + the xlsx-preview suite
   stay green. The table is backend-agnostic: a future pyo3/napi emitter adds
   sibling `@build`/`@munch` arms (or a parallel macro consuming the same rows)
   for one table, N bindings. **Follow-up**: the TS forwarding layer
   (`api-collections.ts` pure-forwarding methods + `as T` casts) is still
   hand-written — generating it from the same table (or a shared data file the
   macro and a TS emitter both read) is the remaining drift surface.
   **Follow-up landed (machine-checkable manifest)**: `scripts/api_manifest.py`
   parses both the `api_methods!` table rows and the hand-written
   `#[wasm_bindgen]` `WorkbookHandle` methods out of `lib.rs` into a checked-in
   `scripts/api_methods.json` (`name`, `jsName`, `kind`, `args` with
   kinds/types, `ret`). `--check` regenerates and diffs against the checked-in
   JSON (fails if stale) and cross-checks the TS layer: every forwarded `jsName`
   must appear as a `handle.<jsName>(` call under `packages/xlsx-preview/src`
   (catches forwarding drift), and any `handle.<name>(` call absent from the
   manifest is reported (catches phantom methods). Wired into xlsx-preview
   `check` as `check:api` (sibling of `check:schema`). The JSON is the contract
   a future pyo3/napi emitter consumes.
5. *(done)* **Worksheet identity is fake.** `Worksheet` wraps a throwaway
   `{current: name}` ref; `wb.sheet("X")` twice gives two objects, and
   `rename()` on one strands the other (and any stored `Range`s). openpyxl's
   workbook owns its worksheet objects — same name, same object.
   **Resolved**: `Workbook` caches `Worksheet` per stable `SheetInfo.id`
   (`worksheetFor(info)`); `sheet`/`worksheets`/`activeSheet`/`addSheet` all
   resolve through the cache so the same id always yields the same object, and a
   cache hit re-syncs the worksheet's name to the live `SheetInfo.name` (covers
   id reuse after delete+add). `rename()` mutates the shared ref, so every
   handle (and any stored `Range`/collection) follows; `removeSheet` evicts.
   TS-only — Rust already exposes the stable id. Verified via
   `api.worksheetIdentity.test.ts` (`sheet()` twice is `toBe`-identical, rename
   propagates, add caches / remove evicts).
6. *(done)* **No bulk-data idiom.** openpyxl's most-used method is `ws.append(row)`.
   **Resolved**: `Workbook::append_rows(sheet, rows)` + `append_row` in
   `xlcore-api` write rows starting at column A after the last data-bearing row
   (max `row_index` among rows with cells, `+1`; empty sheet → row 1) and return
   the written block's `RangeInfo`. wasm `appendRows` + TS
   `Worksheet.appendRow`/`appendRows` forward only. (iter-rows is still served by
   existing range reads.)

### Priority

#3 is correctness — land before expanding `ChartPatch`. #1/#2 are breaking
renames, cheapest while the user count is ~1. #4 pays off across every future
domain. #1/#2/#3/#5/#6 are done; #4 is done for the wasm binding layer (TS
forwarding-layer codegen is the noted follow-up).

## Where to enforce the structure (Rust vs binding) — pyo3/napi readiness

Test: could someone write a pyo3 binding without reading `api-collections.ts`?
Today no — semantics have leaked into the TS frontend:

- ~~`charts.update` / `pivots.update` merge semantics + rollback~~ *(done: both now
  forward to Rust `update_chart`/`update_pivot`)*
- ~~dataBar `min`/`max` defaulting~~ *(done: optional in DTO, defaulted in Rust)*
- ~~autoFilter `setColumnValues/Top10/Custom` sugar + friendly `criteria.kind` error~~
  *(done: criteria booleans are optional + defaulted in Rust (`top`→true); the TS
  helpers carry no defaults and the `criteria.kind` guard is dropped — serde + Rust
  `validate_criteria` reject bad/unsupported kinds)*
- ~~`anchorA1` string→`ChartAnchor` parsing (no Rust counterpart at all)~~ *(done:
  `AnchorSpec` DTO + `resolve_anchor` in `xlcore-api`; patches accept A1 strings)*
- ~~`qref` sheet-qualification of unqualified refs~~ *(done: collection facades take
  `sheet: &str` + a possibly-unqualified `ref`; `qualify_ref` in `xlcore-api` prepends
  the sheet when absent. The TS `qref` helper is deleted; collections/`Range` forward
  `sheet` + raw ref. Workbook-level `allTables.set` passes an empty sheet to keep the
  default-sheet fallback for qualified-or-default refs.)*
- ~~`recalculate({errorsOnly})` filtering, `search` defaults~~ *(done: `recalculate`
  takes an `errors_only` flag in Rust/wasm and filters there; `search` defaults
  already lived in serde `Default` impls, so the TS now forwards options verbatim)*
- ~~matrix-shape validation duplicated in both layers (two error behaviors)~~ *(done:
  the TS `validateMatrixShape`/`rangeDims` are deleted; `setValues`/`setFormulas`
  rely on Rust `validate_matrix_shape` as the single source of shape errors)*

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
