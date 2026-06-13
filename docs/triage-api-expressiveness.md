# Triage: API expressiveness vs the OOXML schema space

Question: our curated DTOs (`xlcore-types` → ts-rs → `api-schema/*.ts`) cover a thin
slice of what OOXML can express. Should we copy/derive types from ooxmlsdk instead?
Reference point: openpyxl's object model.

## Verdict: don't expose ooxmlsdk types directly

1. **No serde.** ooxmlsdk structs derive `Clone/Debug/Default/PartialEq + SdkType`
   only — everything crossing the wasm JSON boundary needs a serde mirror anyway.
2. **1:1 XML shape is agent-hostile.** `Scaling.max` is
   `Option<MaxAxisValue { val: DoubleValue }>` behind a `Box`; choice enums,
   `extLst`, and required-but-defaultable children leak everywhere.
3. **openpyxl doesn't either.** Its win is *naming discipline*: classes mirror
   OOXML tagnames, but `CT_Double/@val` wrappers flatten to plain floats and
   defaults are filled in. The schema is the documentation; the API stays flat.

What we **do** copy from ooxmlsdk: its field inventory. When opening up domain X,
walk the corresponding `CT_*` struct and make a deliberate include/exclude call per
field. `SdkEnum`s are transliterated 1:1 into serde enums.

## Conventions (adopted, openpyxl-style)

- One DTO per OOXML element, same name where sane (`Scaling`, not `AxisBounds`).
- Flatten `val`-wrapper children to scalars.
- Group exactly as the schema groups (axis has `scaling`, chart has `axes`).
- `Info`/`Patch` symmetry; patches partial, infos total.
- Intentionally-excluded schema fields declared per DTO (doc-comment
  `schema-excluded:` line or `scripts/schema_coverage.toml`) so gaps are
  decisions, not accidents.

## Workflow when opening up a domain

1. `scripts/schema_diff.py <SdkStruct> [DtoStruct]` — emits a markdown coverage
   table (covered / flattened / derived / excluded / MISSING). The manifest
   (`scripts/schema_coverage.toml`) lists every opened-up (SdkStruct, DtoStruct)
   pair; `--check` walks all of them and fails on undeclared MISSING fields.
   **Run `--check` after touching any opened-up domain** (wired into xlsx-preview
   `check` as `check:schema`); add the field or declare the deferral.
2. Transliterate sdk enums verbatim.
3. Verify renderer-visible features e2e via xlsx-preview.

Escape hatch for anything unmodeled: raw OPC part XML get/set on `Workbook`
(`partNames`/`getPart`/`setPart`/`removePart`).

## Architecture: three layers (pyo3/napi readiness)

Test: could someone write a pyo3 binding without reading `api-collections.ts`?

1. **`xlcore-api` (Rust): all semantics.** Defaults, validation, ref
   qualification, merge-update, error messages. Facades take `sheet: &str` +
   possibly-unqualified `ref` and qualify internally; anchors accept A1 strings.
2. **Bindings: marshaling only.** Generated from the declarative `api_methods!`
   table in `xlcore-wasm/src/lib.rs` (one table, N bindings). The checked-in
   `scripts/api_methods.json` is the contract a future pyo3/napi emitter
   consumes; `scripts/api_manifest.py --check` (xlsx-preview `check:api`)
   catches staleness and TS forwarding drift.
3. **Per-language wrapper: idiom only.** Classes, chaining, properties — zero
   decisions, so a new language is a weekend rewrite. Don't hoist the object
   model into Rust.

API surface conventions live in `docs/api-conventions.md` (canonical verbs
`list/get/set/add/remove/clear`, class suffixes `Collection`/`Workbook<X>`/
`Accessor`).

## Shipped (summary)

- **Charts P0**: axis object (`ChartAxisPatch`: min/max/log/reversed, units,
  ticks, gridlines, numFmt, crossBetween/crossesAt, dispUnits, labelRotation
  (txPr bodyPr rot; round-trip-only, renderer draws labels horizontally), hidden), combo
  charts + secondary axis, series styling (marker, line, smooth, varyColors,
  invertIfNegative, gapWidth/overlap), per-point fills + explosion (`c:dPt`),
  pie/doughnut holeSize + firstSliceAngle, dispBlanksAs, radar + stock kinds,
  data label numFmt, series trendlines (`c:trendline`:
  linear/poly/movingAvg/exp/log/power, dispEq/dispRSqr, forward/backward,
  intercept, custom name; round-trip-only, renderer doesn't draw), series error
  bars (`c:errBars`: errDir x/y, errBarType both/minus/plus, errValType
  fixedVal/percentage/stdDev/stdErr/cust, val, noEndCap, custom plus/minus
  refs/values; round-trip-only, renderer doesn't draw), per-point data labels
  (`c:dLbl` via `ChartDataLabels.perPoint`: per-index delete flag or overrides of
  showValue/showCategoryName/showSeriesName/showPercent/showLegendKey/position/
  numFmt/separator; renderer-visible), data table (`c:dTable` via
  `ChartPatch.dataTable`: showHorzBorder/showVertBorder/showOutline/showKeys;
  cartesian only, round-trip-only, renderer doesn't draw), 3D chart kinds
  (`ChartKind` bar3d/column3d/line3d/pie3d/area3d → `c:bar3DChart`/`c:line3DChart`/
  `c:pie3DChart`/`c:area3DChart`; 3D cartesian emit the required `c:serAx` third
  axis (deleted), pie3D none), `ChartView3D` (`c:view3D`:
  rotX/rotY/perspective/rightAngleAxes/depthPercent/heightPercent; plot order
  rotX→rotY preserved) + bar3D `c:shape` (`Bar3DShape`, ST_Shape transliterated);
  the export renderer draws 3D charts flat as their 2D equivalent, view3D/shape
  round-trip-only. Surface kinds (`ChartKind` surface3d/surface →
  `c:surface3DChart`/`c:surfaceChart`; emit the `c:serAx` third axis like 3D
  cartesian) + `ChartPatch.wireframe` (`c:wireframe`, lines vs filled bands) +
  reuse `ChartView3D`; renderer doesn't draw surface, round-trip-only
  (bandFmts excluded). ofPie kinds (`ChartKind` pieofpie/barofpie →
  `c:ofPieChart` with `c:ofPieType val=pie|bar`; single series, no axes;
  rejected on multi-series) + `ChartPatch.splitType` (`ChartSplitType`,
  ST_SplitType transliterated — SDK omits the schema's `auto`)/splitPos/
  secondPieSize/seriesLines (`c:serLines` toggle) + reuse gapWidth; renderer
  draws ofPie as a plain pie, ofPie-specific knobs round-trip-only (custSplit
  excluded). Plot-area + legend styling (`ChartPatch/Update/Info.plotArea`:
  `ChartPlotArea` fill + border → `c:plotArea/c:spPr`; `.legend`: `ChartLegend`
  fill + border → `c:legend/c:spPr` + font `ChartTextStyle` →
  `c:legend/c:txPr/a:p/a:pPr/a:defRPr` size/bold/italic/color/typeface; reuses
  `ChartLine` for borders, fill is `RRGGBB`/`AARRGGBB`/`none`; renderer draws
  plot-area + legend fill/border and the legend font; CT_PlotArea spPr only +
  CT_Legend with declared deferrals). Manual layout (`ChartManualLayout` → `c:layout/
  c:manualLayout`: layoutTarget inner/outer (plot area only), xMode/yMode/wMode/
  hMode edge/factor (ST_LayoutMode/ST_LayoutTarget transliterated), x/y/w/h
  fractions; on `ChartPlotArea.layout`, `ChartLegend.layout`, `ChartPatch.
  titleLayout`; built in schema order — plot-area layout before the plot charts,
  legend layout after legendPos; CT_Layout/CT_ManualLayout, extLst excluded;
  round-trip-only, renderer ignores manual layout). 3D extras (`ChartPatch/
  Update/Info.gapDepth` → `c:gapDepth` bar3D/column3D only, per-series 3D shape
  `ChartSeriesPatch/Info.shape` → `c:ser/c:shape`, floor/wall formatting
  `.floor`/`.sideWall`/`.backWall` → `c:floor`/`c:sideWall`/`c:backWall`/`c:spPr`
  via `ChartSurfaceWall` fill+border reusing the plot-area spPr builders, built
  after view3D before plotArea; 3D charts only; thickness/pictureOptions/extLst
  excluded; round-trip-only). chartStyle/colorStyle companion parts
  (`ChartPatch/Update/Info.styleXml`/`colorStyleXml`: raw `style{N}.xml`
  (`cs:chartStyle`) + `colors{N}.xml` (`cs:colorStyle`) written verbatim as
  `chartStyle`/`chartColorStyle` companion parts + rels via
  `add_new_part_auto_id` + `set_data`; opaque escape hatch, the ~40-entry
  styleEntry / colorStyle schemas are not modeled; empty string on update
  removes the part. The SDK eager-parses these parts on open, so supplied XML
  must be a schema-valid part (e.g. copied from an Excel-authored file);
  renderer ignores them, round-trip-only). Per-point `c:dPt` extras
  (`ChartDataPoint`: `invertIfNegative` bar/bubble, `marker` reusing
  `ChartMarker` line/scatter/radar, structured `gradientFill` (`a:gradFill`
  linear via `ChartGradientFill`/`ChartGradientStop`) / `patternFill`
  (`a:pattFill` via `ChartPatternFill` + `ChartPatternPreset` =
  ST_PresetPatternVal transliterated) additive fields beside the existing solid
  `fill` string, precedence gradient > pattern > solid; CT_DPt bubble3D/
  pictureOptions/extLst excluded; renderer draws solid + gradient + pattern
  per-point fills, the rest round-trip-only). In-place `update_chart`
  (atomic, preserves unmodeled XML).
- **chartEx authoring** (modern `cx:` charts, separate `chartEx{N}.xml` part +
  `.../2014/relationships/chartEx` rel, referenced from the drawing via a
  graphicFrame whose `a:graphicData uri=.../2014/chartex` wraps `<cx:chart
  r:id>` as `GraphicDataChoice::XmlAny`): `ChartExPatch`/`ChartExInfo` +
  `chart_exs`/`set_chart_ex`/`remove_chart_ex` (TS `Worksheet.chartsEx`)
  authoring all 8 renderer-visible `ChartExKind` layouts — waterfall, funnel,
  treemap, sunburst, histogram, pareto, boxWhisker, regionMap. Patch carries
  kind, title, anchor, `categoriesRef` (`cx:strDim type=cat`; multi-column range
  → hierarchy levels for treemap/sunburst), `series` (`cx:numDim`; dim type
  derived val/size/colorVal), per-kind knobs (`subtotals` waterfall,
  `binCount`/`binSize` histogram, `quartileMethod` boxWhisker), legendPosition.
  Histogram/pareto emit `clusteredColumn`/`paretoLine` under the hood (collapsed
  back on read); cartesian kinds emit two `cx:axis`. No cached `cx:lvl` values —
  the renderer resolves the formula refs against sheet cells (same path as
  legacy charts). Build + list + remove + reopen round-trip; the SDK can't
  parse cx's `office2016, qname` attr form so these pairs aren't in
  `schema_coverage.toml` (deferrals live in the DTO `schema-excluded:` lines).
  Per-point fills, valueColors, data labels, axis styling, in-place update
  excluded for now (follow-up).
- **Rich text in cells**: `setRichText`/`richText` (inline-string `CT_RElt`
  runs with per-run `FontPatch`); `CellInfo.richText`, renderer-visible.
- **Styles P1**: cell protection, pattern + gradient fills, font
  vertAlign/family/scheme, diagonal borders, alignment
  shrinkToFit/justifyLastLine/readingOrder.
- **Named cell styles** (workbook-scoped `wb.namedStyles`): define
  (`NamedStylePatch`: name + `StylePatch` → a `cellStyleXfs` master xf + a
  `cellStyles` entry, optional `builtinId`) and apply (`StylePatch.namedStyle`
  sets the cellXf `@xfId` to the named master + copies its format so the cell is
  renderer-visible and can still layer direct formatting on top). `setNamedStyle`
  is an in-place upsert; `Normal` is protected. CT_CellStyle outline_level/hidden/
  custom_builtin/extLst excluded; the master xf maps via `StylePatch` (same path
  as cellXfs). Renderer resolves named styles through `@xfId`.
- **Worksheet P1**: row/col outline grouping, sheet properties (tab color, zoom,
  showZeros, rightToLeft, default row height/col width), print area/titles +
  manual page breaks, `appendRows` bulk idiom.
- **Architecture**: semantics moved out of TS into `xlcore-api` (qref, anchor
  parsing, autofilter sugar, matrix validation, recalc/search defaults,
  update merge), wasm bindings generated from `api_methods!`, worksheet identity
  cached per stable sheet id, raw part XML escape hatch, verb/naming audit.

## Remaining gaps

### P2 — backlog

remaining 3D extras (floor/sideWall/backWall thickness/pictureOptions,
per-series 3D invertIfNegative/marker), chartEx follow-ups (in-place
`update_chart_ex`, per-point `cx:dataPt` fills, `cx:valueColors` authoring,
data labels, region-map geo cache, axis/title styling), remaining `c:dPt`
fields (per-point bubble3D, blip/picture fills).

### Follow-up

- **Renderer parity for the write-only chart features.** Much of the P2 chart
  work above round-trips for Excel but the xlsx-preview renderer doesn't draw it
  yet — confirmed valid by opening authored files in real Excel. Teach the
  renderer to consume: trendlines, error bars, axis label rotation, data table,
  3D geometry (currently flattened to 2D) + view3D + floor/walls, surface
  (currently a v0 stub), ofPie split/secondary pie (currently drawn as a plain
  pie), plot-area/legend spPr fills + fonts, manual layout, per-point
  marker/gradient/pattern fills, and named-style xfId resolution edge cases. The
  read-side schema already models most of these; the gap is the draw path.
- **TS forwarding-layer codegen** from `api_methods.json` — drift already caught
  by `check:api`, so only worth doing when a second binding (pyo3/napi) lands.

### Already adequate

Tables, autofilter, data validation, CF (incl. colorScale/dataBar/iconSet),
sparklines, comments/threaded notes, hyperlinks, merges, defined names, page
setup/margins/header-footer, protection, images, pivots (v1 scope), search,
structural ops.
