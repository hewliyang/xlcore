# Changelog

All notable changes to `@hewliyang/xlsx-preview` are documented here.
This project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `WorkerWorkbook` range ops: `setRangeValues`, `setRangeFormulas`, `copyRange`, `clearRange`.
- `WorkerWorkbook.setImage(sheetName, patch)` inserts/anchors an image; bytes transferred across the worker boundary.
- Pure `clipboardModel` helpers: `serializeRange`/`parseClipboard` (TSV + HTML table with internal `data-xlcore` payload preserving formulas).
- Range copy/cut (Cmd/Ctrl+C/X) in editable previewer: writes `text/plain`+`text/html`, emits `rangecopy`/`rangecut` (cut overlay deferred).
- Range paste (Cmd/Ctrl+V) in editable previewer: emits `rangepaste`; internal copies preserve formulas via `copyRange`, external uses `setRangeValues`, cut source cleared (single→multi tiling deferred).
- Fill handle drag-to-fill in editable previewer: drag the bottom-right handle to copy-fill (tile) along the dominant axis; pure `projectFill` helper, emits `rangefill` (linear/date series deferred).
- Name box shows the defined-name when the selection matches a named range.
- "+" tab button (editable mode) creates a new sheet via `WorkerWorkbook.addSheet`; wired into the example app.
- Example app starts with a blank workbook and gains a "New" button to create one from scratch.
- Category axis labels auto-rotate (-45°, then -90°) when horizontal labels overflow instead of being dropped; band height reserves the rotated space. Explicit XML `rot` stays authoritative.

### Fixed

- Axis number format with a quoted literal (e.g. `0.0"%"`) no longer triggers the percent ×100 operator; quoted `"..."` segments are emitted as literal text.

### Changed

- Chart fonts (axis/title/legend/axis-title/data labels) now auto-scale with plot-area size like Excel, centralized in `chartUtils`; explicit `sz` overrides stay authoritative.
- Recalibrated chart font auto-scale (reference 360→200, cap 2→2.2) so typical embedded charts get a visible ~1.5× bump matching Excel weight.
- Chart value-axis min/max now uses Excel's data-driven 5/6 zero-clamp rule (axis hits zero only when data is close enough), replacing the per-chart-type `zeroClamp` boolean.
- Drawing move is now a single gesture: pressing and dragging any drawing selects and moves it in one motion (no separate select-first click); `drawingmoved` only fires when the anchor actually changes.
- Recalc now reuses a resident calc engine across recalcs; cell value/formula edits route into it, other mutations invalidate it for a clean rebuild.
- Cell edits now re-extract + redraw only the active sheet (`applyEdit` returns a single-sheet layout) merged via `previewer.patchSheetLayout`, instead of reserializing/repainting the whole workbook.
- Formula bar now shows clean display formulas (e.g. `CONCAT` instead of `_xlfn.CONCAT`) by stripping OOXML `_xlfn.`/`_xlws.` decorations at extract time.
- Data-validation dropdown arrows now render in a gutter just outside the cell's right edge (no longer overlap cell text); interactively only the active cell's arrow shows, static previews still show all.

### Added

- `worksheet.images.update(id, ImageUpdate)` (WASM `updateImage`) updates an existing image's anchor/name/rotation/flips in place, so image moves/resizes persist on `save()`.
- Chart anchor round-trip test: `charts.update` move persists through `save`/`open`.
- Example app persists chart moves: wires `drawingmoved` → `recalcWorkbook.moveDrawing` → `patchSheetLayout`.
- `WorkerWorkbook.moveDrawing` + editWorker `moveDrawing` op persist chart and image moves via `resolveDrawingId` anchor-match (charts→`charts.update`, images→`images.update`).
- `"drawingmoved"` previewer event dispatched on drawing move/resize/nudge with sheet name, kind, index, and ChartAnchor before/after.
- `wireAnchorToChartAnchor`/`chartAnchorToWireAnchor` pure helpers for drawing anchor round-tripping.
- Drag a selected drawing's resize handle to resize it, with live redraw and re-anchoring.
- Hovering a selected drawing's resize handle shows the matching directional cursor.
- Arrow keys nudge a selected drawing by 1px (10px with Shift) with live redraw and re-anchoring.
- Drag a selected drawing's body to move it, with live redraw and re-anchoring via `rectToAnchor`.
- `rectToAnchor(rect, grid, template)` inverts `anchorToRect`, preserving the template's two-cell vs absolute anchor style.
- Hovering a selected drawing's body shows a `move` cursor; selection clears on sheet change, range select, and scroll-to-cell.
- Selected drawings now render selection chrome (1px box + 8 square handles) in the grid accent color.
- Click a drawing (chart/image/shape) to select it (top-most wins); Escape or a cell/empty click clears it. Visual selection chrome lands next.
- `previewer.patchSheetLayout(singleSheetLayout)` merges one sheet into the resident layout, preserving scroll/selection.
- Interactive list data-validation dropdowns: clicking a dropdown arrow (editable previewer) opens a popover of the validation's options and writes the chosen value via `celledit`.
- Render schema exposes `Sheet.validationDropdowns` + `Sheet.validationLists` (list-type data-validation cells and their resolved options).
- Render list data-validation dropdown arrows on the canvas previewer.

### Fixed

- Worker bundle (`editWorker.js`) now ships its `drawingResolve.js` dependency, so `WorkerWorkbook.open` no longer throws — restores cell editing and data-validation dropdowns in the example app (both silently no-op when the worker fails to load). `check-dist-imports` now also imports the worker bundles to catch missing-sibling regressions.
- Writing formulas with post-2007 functions (e.g. `MAXIFS`, `TEXTJOIN`, `XLOOKUP`) now stores the canonical `_xlfn.`-prefixed form, so saved files stay valid OOXML instead of triggering Excel repair. Covers `setFormula`, range formulas, and expression-valued defined names.
- `WorkerWorkbook`: tolerate defined names that fail to replay into the shadow workbook so opening still succeeds (e.g. array-constant names); edits no longer silently no-op.

### Added

- Example app: pivot/table filtering routed through `WorkerWorkbook` async ops.
- `WorkerWorkbook` pivot/table ops (`pivotMetas`/`distinctValues`/`updatePivot`/`tableSetFilter`/`tableSetSort`); pivot/table filter controllers now accept async returns.
- `./worker` entry: `WorkerWorkbook` proxy runs recalc/layout/save off the main thread via a persistent edit worker, with a sync shadow-backed `engine` for highlights/autocomplete.
- Example app: editing/recalc/save routed through `WorkerWorkbook`, keeping the main thread responsive on commit.
- Previewer: optional `onDownload` callback renders a Download button in the toolbar.

- Point mode: click/drag/shift-click the grid while editing a formula inserts or extends A1 references (pure `formulaPointMode`; `InteractOptions.isPointModeActive`/`onPointModeRef`).
- Function autocomplete dropdown in the editor/formula bar (pure `autocompleteState`; keyboard nav, Enter/Tab insert `NAME(`, Esc closes dropdown only).
- Function signature ScreenTip in the editor/formula bar (pure `formulaSignature`; shows args + summary with the active argument highlighted, covers all 345 engine functions).
- Previewer `engine?: PreviewerEngine` option drives live precedent highlighting from the editor/active formula (same-sheet refs only, boxes only).
- Render `highlights?: HighlightRange[]` option draws colored precedent boxes beneath the selection.
- `Workbook.parseFormulaReferences(sheet, anchor, formula)` and `Workbook.functionNames()` (uncommitted-formula refs + English function catalog).
- Example app: wire cell editing via `celledit` + recalc modes (Auto recalc checkbox, F9 manual recalc).
- Previewer `editable` option: writable formula bar emitting a `celledit` event on Enter (Escape restores).
- Inline cell editing overlay (F2/double-click/typing) over the active cell, emitting `celledit` with `commitMove`.
- `table_engine::compute_hidden_rows`: pure Excel autofilter view engine (Values/Custom/Top10 + multi-column AND).
- Authored autofilter now applies `row.hidden` from criteria after `set/remove_auto_filter_column`.
- AutoFilter sort authoring: `autoFilter.setSort/clearSort` (wasm `setAutoFilterSort/removeAutoFilterSort`) writes `sortState` + physically reorders data rows (numbers numerically, text case-insensitive, blanks last); formula/merge refs not fixed up.
- Export emits sheet-level `TableFilterArrow { r, c, columnOffset, columnName, rangeRef }` carrying column identity for autofilter/table-autofilter headers.
- `interact.ts` hit-tests table filter arrows (`onTableFilter` → `{ field, columnOffset, rangeRef, rect }`, pointer cursor on hover).
- `tableFilterPopover.ts` interactive table-header dropdown (Sort A→Z/Z→A + value checklist + clear) wired via `tableController` option and `tablefilter` event.
- Examples (react-vite + xlsx-app.html) wire a `TableFilterController` (distinct values + autoFilter set/setColumnValues/removeColumn/setSort/clearSort).
- xlcore-api color resolver (`resolve_color_hex`): rgb/indexed/theme+tint → `RRGGBB`.
- `CellInfo.style` read-back: `get_cell` resolves a cell's xf to a flat `StylePatch`.
- Style read-back round-trip corpus test enforcing `resolve(get(cell)) ⊇ set(cell, patch)`.

- Renderer draws pseudo-3D surface charts (`c:surfaceChart`/`c:surface3DChart`): a mesh over the (category × series) z-grid via the bar3D oblique projection + floor/back/side walls, painter-sorted quads filled by a height-band color ramp, or `c:wireframe` grid lines. Decoded into wire `Chart.type` `"surface"` + `Chart.wireframe`.
- Renderer draws pseudo-3D bar3D/column3D geometry: oblique-projection boxes (front + lighter top + darker side faces) over a floor + back wall + side wall, sizing depth from `c:view3D` (rotX/rotY/depthPercent) + `c:gapDepth` and filling walls from `c:floor`/`c:sideWall`/`c:backWall` spPr. Decoded into the wire `Chart.is3d`/`view3d`/`gapDepth`/`floorFill`/`sideWallFill`/`backWallFill` (`ChartView3D`); line3D/area3D/pie3D/surface still flat.
- Renderer consumes manual layout (`c:layout/c:manualLayout`) for the plot area (inner/outer `layoutTarget`), legend, and title, positioning them at the explicit fractional rectangle (edge/factor `*Mode`) of the chart area. Decoded into the wire `Chart.plotAreaLayout`/`legendLayout`/`titleLayout` (`ChartManualLayout`).
- Renderer draws ofPie satellite split (`c:ofPieChart` pieOfPie/barOfPie): main pie with an aggregated "Other" slice plus a secondary pie or vertical stacked bar of the split-off points, connector series-lines, honoring `splitType`/`splitPos`/`secondPieSize`/`serLines`. Decoded into the wire `Chart.ofPieType`/`splitType`/`splitPos`/`secondPieSize`/`seriesLines`.
- Renderer draws plot-area + legend styling: plot-area `c:plotArea/c:spPr` background fill + border, legend `c:legend/c:spPr` fill + border, and `c:legend/c:txPr` font (size/bold/italic/color/typeface). Decoded into the wire `Chart.plotAreaFill`/`plotAreaBorder`/`legendFill`/`legendBorder`/`legendFont` (`ChartStyleBorder`/`ChartStyleFont`).
- Renderer draws per-point gradient + pattern fills (`c:dPt` `a:gradFill`/`a:pattFill`): linear-gradient and preset-pattern (`ST_PresetPatternVal`) fills for bar/column and pie/doughnut points, precedence gradient > pattern > solid. Decoded into the wire `ChartSeries.pointFills`.
- Renderer draws the chart data table (`c:dTable`): a grid below the plot with one row per series and one column per category showing the formatted values, optional legend-key swatches (`showKeys`), and horizontal/vertical/outline border toggles. Decoded into the wire `Chart.dataTable`; non-stacked column/line/area only.
- Renderer draws rotated axis tick-labels (`txPr bodyPr rot`): category/value-axis labels drawn at the authored angle (`Chart.catAxisLabelRotation`/`valAxisLabelRotation`, decoded from `bodyPr/@rot` 60000ths→degrees) with the axis band auto-resized to avoid clipping.
- Renderer draws series error bars (`c:errBars`): vertical I-beams (errDir=y) with fixedVal/percentage/stdDev/stdErr/cust magnitudes, both/plus/minus sides + noEndCap; line, column and scatter charts. Decoded into the wire `ChartSeries.errorBars`.
- Renderer draws series trendlines (`c:trendline`): linear/poly/exp/log/power fits + moving average, with forward/backward projection; line, column and scatter/bubble charts. Decoded into the wire `ChartSeries.trendlines`.
- Per-point `c:dPt` extras on `ChartDataPoint`: `invertIfNegative` (bar/bubble), `marker` (reuses `ChartMarker`; line/scatter/radar), and structured `gradientFill` (`a:gradFill` linear, `ChartGradientFill`/`ChartGradientStop`) / `patternFill` (`a:pattFill`, `ChartPatternFill` + `ChartPatternPreset` = `ST_PresetPatternVal`) additive fields alongside the existing solid `fill` string (gradient > pattern > solid precedence). Round-trips + in-place update; renderer still draws only solid per-point fills. `bubble3D`/`pictureOptions`/`extLst` excluded.
- Named cell styles (`styles.xml` `cellStyles` + `cellStyleXfs`): workbook-scoped `Workbook.namedStyles` collection (`namedStyles`/`setNamedStyle`/`removeNamedStyle`) defining `NamedStylePatch` (name + `StylePatch` baked into a master xf + optional `builtinId`); apply to cells via `StylePatch.namedStyle` (sets the cellXf `@xfId` to the named master + copies its format so the cell renders the style and can layer direct formatting on top). `setNamedStyle` is an in-place upsert; `Normal` cannot be removed. Round-trips; the renderer resolves named styles through `@xfId`. `CT_CellStyle` outline_level/hidden/custom_builtin/extLst excluded.
- chartEx authoring (modern `cx:` charts in a separate `chartEx{N}.xml` part): `Worksheet.chartsEx` collection (`setChartEx`/`chartExs`/`removeChartEx`) authoring all 8 renderer-visible `ChartExKind` layouts — waterfall, funnel, treemap, sunburst, histogram, pareto, boxWhisker, regionMap. `ChartExPatch` carries kind, title, anchor, `categoriesRef` (`cx:strDim`), series (`cx:numDim`; dim type derived: val/size/colorVal) + per-kind knobs (`subtotals` waterfall, `binCount`/`binSize` histogram, `quartileMethod` boxWhisker). Writes the chartEx graphicFrame (c-namespace `a:graphicData` + `cx:chart` r:id) + part + rels; pareto/histogram emit `clusteredColumn`/`paretoLine`. Build + list + remove + round-trip; renderer draws all 8 from resolved cell data.
- chartStyle/colorStyle companion parts: `ChartPatch/Update/Info.styleXml`/`colorStyleXml` write `style{N}.xml` (`cs:chartStyle`) + `colors{N}.xml` (`cs:colorStyle`) verbatim as companion parts + rels (opaque escape hatch; not modeled). Empty string on update removes the part. Supplied XML must be a schema-valid part (e.g. copied from Excel). Round-trip-only; renderer ignores them. Rides on `setChart`/`updateChart`.
- 3D chart extras: `ChartPatch/Update/Info.gapDepth` (`c:gapDepth`, 0..=500; bar3D/column3D only), per-series 3D shape `ChartSeriesPatch/Info.shape` (`c:ser/c:shape`, `Bar3DShape`; bar3D/column3D only), and floor/wall formatting `.floor`/`.sideWall`/`.backWall` (`ChartSurfaceWall`: fill + border → `c:floor`/`c:sideWall`/`c:backWall`/`c:spPr`; 3D charts only, built after view3D before plotArea). Round-trips for Excel; the renderer draws 3D flat. `c:thickness`/`c:pictureOptions`/`extLst` excluded. Rides on `setChart`/`updateChart`.
- Chart manual layout (`ChartManualLayout` → `c:layout/c:manualLayout`: `layoutTarget` inner/outer (plot area only), `xMode`/`yMode`/`wMode`/`hMode` edge/factor, `x`/`y`/`w`/`h` fractions) on `ChartPlotArea.layout`, `ChartLegend.layout`, and `ChartPatch/Update/Info.titleLayout`. Built in schema order (plot-area layout before the charts; legend layout after legendPos). Round-trips for Excel; the renderer ignores manual layout. Rides on `setChart`/`updateChart`.
- `ChartPatch/Update/Info.plotArea` (`ChartPlotArea`: fill + border → `c:plotArea/c:spPr`) and `.legend` (`ChartLegend`: fill + border → `c:legend/c:spPr`, font `ChartTextStyle` → `c:legend/c:txPr/a:p/a:pPr/a:defRPr` size/bold/italic/color/typeface). Reuses `ChartLine` for borders; fill accepts `RRGGBB`/`AARRGGBB`/`"none"`. Round-trips for Excel; the renderer draws plot area/legend with its own styling. Rides on `setChart`/`updateChart`.
- ofPie chart kinds (`ChartKind` `pieofpie`/`barofpie` → `c:ofPieChart` with `c:ofPieType val=pie|bar`; single series, no axes) plus `ChartPatch/Update/Info.splitType` (`ChartSplitType`: cust/percent/pos/val), `.splitPos`, `.secondPieSize` (5..=200), `.seriesLines` (`c:serLines`), and existing `.gapWidth`. Round-trips for Excel; the renderer draws ofPie as a plain pie (split/secondPieSize/serLines not drawn). `c:custSplit` excluded. Rides on `setChart`/`updateChart`.
- Surface chart kinds (`ChartKind` `surface3d`/`surface` → `c:surface3DChart`/`c:surfaceChart`; emit the `c:serAx` third axis like 3D cartesian) plus `ChartPatch/Update/Info.wireframe` (`c:wireframe`, lines vs filled bands). Reuses `ChartView3D` for `c:view3D`. Round-trips for Excel (renderer doesn't draw surface); `c:bandFmts` excluded. Rides on `setChart`/`updateChart`.
- 3D chart kinds (`ChartKind` `bar3d`/`column3d`/`line3d`/`pie3d`/`area3d` → `c:bar3DChart`/`c:line3DChart`/`c:pie3DChart`/`c:area3DChart`) plus `ChartPatch/Update/Info.view3d` (`ChartView3D`: rotX/rotY/perspective/rightAngleAxes/depthPercent/heightPercent) and `.barShape` (`Bar3DShape`, `c:shape`, bar3D/column3D only). 3D cartesian emit the required `c:serAx` third axis (deleted); pie3D needs no axes. The export renderer draws 3D charts flat as their 2D equivalent; view3D/shape round-trip for Excel. Rides on `setChart`/`updateChart`.
- `ChartPatch.dataTable`/`ChartUpdate.dataTable`/`ChartInfo.dataTable` (`ChartDataTable`): chart data table (`c:dTable` in `c:plotArea`, after the axes) — showHorzBorder/showVertBorder/showOutline/showKeys; cartesian charts only (rejected on pie/doughnut/scatter/bubble/radar/stock); round-trips for Excel (renderer doesn't draw). Rides on `setChart`/`updateChart`.
- `ChartAxisPatch.labelRotation`: tick-label rotation in whole degrees (-90..=90), stored as axis `c:txPr`/`a:bodyPr/@rot` (60000ths of a degree) on cat+val axes; round-trips for Excel (renderer draws labels horizontally). Rides on `setChart`/`updateChart`.
- `ChartDataLabels.perPoint` (`ChartDataLabel`): per-point data labels (`c:dLbl`) — per data-point `index` delete flag or overrides of showValue/showCategoryName/showSeriesName/showPercent/showLegendKey/position/numFmt/separator; renderer-visible. Rides on `setChart`/`updateChart`.
- `ChartSeriesPatch.errorBars`/`ChartSeriesInfo.errorBars` (`ChartErrorBars`/`ChartErrorDirection`/`ChartErrorBarType`/`ChartErrorValueType`): per-series error bars (`c:errBars`) — errDir x/y, errBarType both/minus/plus, errValType fixedVal/percentage/stdDev/stdErr/cust, value, noEndCap, custom plus/minus refs or inline values; bar/column/line/area/scatter/bubble only; round-trips for Excel (renderer doesn't draw). Rides on `setChart`/`updateChart`.
- `ChartSeriesPatch.trendline`/`ChartSeriesInfo.trendline` (`ChartTrendline`/`TrendlineKind`): per-series regression trendline (`c:trendline`) — linear/poly/movingAvg/exp/log/power, dispEq/dispRSqr, forward/backward, intercept, custom name; bar/column/line/area/scatter/bubble only; round-trips for Excel (renderer doesn't draw). Rides on `setChart`/`updateChart`.
- `Cell.setRichText(runs)`/`Cell.richText()` + `CellInfo.richText` (`RichText`/`RichTextRun`): author and read multi-run inline-string cells (`CT_RElt`), each run carrying a per-run `FontPatch`; renderer-visible. Semantics in Rust (`Workbook::set_rich_text_in`).
- `Workbook.partNames()`/`getPart(name)`/`setPart(name, xml)`/`removePart(name)` — escape hatch for raw OPC part XML (read/author/delete unmodeled schema); unmodeled parts round-trip verbatim. Semantics in Rust (`Workbook::part_names`/`get_part_xml`/`set_part_xml`/`remove_part_xml`).
- `Worksheet.appendRow(values)`/`appendRows(rows)` (openpyxl `ws.append` idiom): write rows starting at column A after the last data-bearing row; returns the written block's `RangeInfo`. Semantics in Rust (`Workbook::append_rows`).
- `SheetPageSetupPatch.printArea`/`printTitleRows`/`printTitleColumns`/`rowBreaks`/`columnBreaks` (defined-name-backed `_xlnm.Print_Area`/`_xlnm.Print_Titles` + `x:rowBreaks`/`x:colBreaks`): print area, repeating row/column titles, and manual page breaks; authored via `setPageSetup`, merged/cleared component-wise, and round-tripped (round-trip-only for Excel).
- `Worksheet.properties.get`/`.set` (`SheetPropertiesPatch`): tab color, zoom (10..=400), `showZeros`, `rightToLeft`, default row height / col width; round-tripped, with tab color + default row/col sizes rendered by the previewer.
- `AlignmentPatch.shrinkToFit`/`justifyLastLine`/`readingOrder` (`ReadingOrder` context|leftToRight|rightToLeft, sdk `CT_CellAlignment` 1:1): authored via `setStyle`, deduped, round-tripped, and surfaced in the layout (round-trip-only for Excel).
- `FontPatch.vertAlign`/`family`/`scheme` (`VertAlign` baseline|superscript|subscript, `FontScheme` none|major|minor, sdk-transliterated; `font/vertAlign`+`@family`+`scheme`): authored via `setStyle`, deduped, round-tripped, and `vertAlign` rendered as raised/lowered runs by the previewer.
- `Worksheet.groupRows`/`groupColumns` (+ `ungroupRows`/`ungroupColumns`) — row/column outline grouping (`outlineLevel`, optional collapsed hide + summary `collapsed` flag, `sheetFormatPr/@outlineLevelRow`/`@outlineLevelCol`); authored, round-tripped, and rendered as gutter brackets by the previewer.
- `FillPatch.pattern`/`foreground`/`background` (`PatternType`, sdk `ST_PatternType` transliterated, `x:patternFill`): non-solid pattern fills with fg/bg colors; authored via `setStyle`, deduped, round-tripped, and rendered by the previewer.
- `StylePatch.protection` (`ProtectionPatch { locked, hidden }`, `x:protection` + `@applyProtection`): cell-level lock/hide for sheet protection; authored via `setStyle`, deduped, and round-tripped (write-only for Excel).
- `ChartDataPoint.explosion` (`c:explosion/@val`, 0..=400 percent of radius): per-point pie/doughnut slice offset; authored on every series kind, round-tripped, updated in place, and rendered as offset slices by the previewer.
- `ChartPatch.varyColors` (`c:varyColors`, chart-level) and `ChartSeriesPatch.invertIfNegative` (`c:invertIfNegative`, bar/column + bubble series), on the respective `Patch`/`Update`/`Info` types; authored, round-tripped, and updated in place (round-trip-only for Excel).
- `ChartPatch.dispBlanksAs` (`DispBlanksAs` span|gap|zero, `c:dispBlanksAs`) on `ChartPatch`/`ChartUpdate`/`ChartInfo`; how blank source cells are plotted, default `gap`. Authored, round-tripped, and updated in place (round-trip-only for Excel).
- `ChartSeriesPatch.line` (`ChartLine { widthEmu, dash, none }` + `LineDash` enum, `spPr/a:ln`): per-series line/outline width, preset dash, or hidden line; authored on every series kind, round-tripped, updated in place, and `widthEmu`/`dash` rendered by the previewer.
- `ChartAxisPatch.displayUnits` (`c:dispUnits`, value axis only): builtin `ST_BuiltInUnit` name (`"millions"`) or custom divisor number; authored, round-tripped, updated in place, and rendered (scaled axis labels + unit label band).
- `ChartSeriesPatch.smooth` (`c:smooth`) for line/scatter series; on scatter charts a smoothed series sets `c:scatterStyle` to `smoothMarker`. Authored, round-tripped, updated in place, and rendered as curved splines by the previewer.
- `ChartPatch.holeSize` (10..=90, `c:holeSize`) for doughnut charts and `ChartPatch.firstSliceAngle` (0..=360, `c:firstSliceAng`) for pie/doughnut charts, on `ChartPatch`/`ChartUpdate`/`ChartInfo`; authored, round-tripped, updated in place, and rendered by the previewer.
- `ChartSeriesPatch.dataPoints` (`ChartDataPoint { index, fill }`, `c:dPt`): per-point fill overrides — hex `RRGGBB`/`AARRGGBB` or `"none"` (`a:noFill`) for the waterfall idiom; authored on every series kind, round-tripped, updated in place, and rendered by the previewer.
- `ChartDataLabels.numberFormat` (`c:numFmt`) on chart data labels; authored, round-tripped, updated in place, and rendered by the previewer.
- `ChartKind` `radar` + `RadarStyle` (`standard`|`marker`|`filled`) on `ChartPatch`/`ChartUpdate`/`ChartInfo`; authored as `c:radarChart`, read back, updated in place, and rendered by the previewer.
- `scripts/schema_diff.py <SdkStruct> [DtoStruct]`: coverage diff of an ooxmlsdk `CT_*` struct's fields against an `xlcore-types` DTO; emits a markdown table for the DTO doc comment. Run when opening a domain.
- Combo charts / secondary axis: per-series `kind` + `axis` (`ChartAxisGroup` primary|secondary) on chart series; mixing column/bar/line/area emits multiple plot groups and synthesizes a right-hand value axis. Authored, round-tripped, and rendered by the previewer.
- `marker` (`ChartMarker { style, size }` + `MarkerStyle` enum) on chart series; authored on line/scatter series, read back, and rendered by the previewer.
- `gapWidth` (0..=500) + `overlap` (-100..=100) on `ChartPatch`/`ChartUpdate`/`ChartInfo` for bar/column charts; authored, read back, and rendered by the previewer.
- `ChartAxisPatch` on `ChartPatch`/`ChartUpdate`/`ChartInfo` as `categoryAxis`/`valueAxis`: author axis `title`, `hidden`, `min`/`max`, `logBase`, `reversed`, `majorUnit`/`minorUnit`, `major`/`minorGridlines`, `major`/`minorTickMark` (`TickMark`), `tickLabelPosition` (`TickLabelPosition`), `numberFormat`, `crossBetween` (`CrossBetween`), `crossesAt`. `min`/`max`/`majorUnit`/`majorGridlines`/`numberFormat` render in the previewer; the rest round-trips for Excel. The legacy `categoryAxisTitle`/`valueAxisTitle` remain as sugar (axis `title` wins).

- Interactive pivot filter dropdowns: clicking a canvas-painted filter arrow opens a built-in (framework-agnostic, vanilla-DOM) item popover wired via the new `pivotController` option (`items`/`hiddenValues`/`setHidden`); `setHidden` returns a fresh `WorkbookLayout` (e.g. `pivots.update(...)` + `workbook.layout()`) that the previewer swaps in place via the new `replaceLayout()` — no Blob reload or wasm reparse. A lower-level `onPivotFilter` event (`{ pivot, field, axis, rect }`) is also emitted for custom UIs. Engine pivot metadata now carries `filterArrowCells: PivotFilterArrow[]` (`{ r, c, field, axis }`); `distinctValuesFor` is exported from both `./react` and `./api` for populating the dropdown. The vanilla `examples/xlsx-app.html` demo (served by `pnpm preview`) wires a `pivotController` backed by a main-thread `Workbook`, so the dropdown works there with no React.

- Pivot per-item filtering: `PivotPatch`/`PivotInfo` gain optional `hiddenItems` (`PivotFieldFilter[]` — `{ field, hide }`). Authoring marks the matching `pivotField` items `h="1"`; both `pivots.set` (in-sheet render) and `pivots.preview` honor it, and the getter reverse-maps hidden items so `update()` round-trips.

- Pivot table rendering: axis members now follow the stored `pivotField` item order (honors manual sorts), falling back to value sort for items absent from that list.

- Pivot table editing: `worksheet.pivots.update(id, partial)` merges a partial patch over the existing pivot and re-authors it (remove + re-set with rollback), mirroring `charts.update`. `PivotInfo` now exposes `anchorCell` (top-left of the location, page-field offset removed) so updates round-trip in place.
- Pivot live preview: `worksheet.pivots.preview(patch)` (WASM `pivotPreview`) aggregates a `PivotGrid` in-memory without authoring any parts, enabling drag/recompute UIs with no save/reopen round-trip.
- Pivot table rendering: empty `pivotCacheRecords` now fall back to aggregating the `worksheetSource` range, so caches stripped before save (common with `refreshOnLoad`) still render.
- Pivot table rendering: nested column headers (two column fields, single data field) now materialize with per-outer-group leaf columns, `{outer} Total` subtotal columns, a grand-total column, and a 3-row header; verified vs SpreadJS.
- Pivot table rendering: multiple data fields combined with a single column field now materialize (3-row header, each column group expands into one sub-column per data field, grand-total group emits `Total <dataname>` per field); verified vs SpreadJS.
- Pivot table filtering: the render engine now honors hidden `pivotField` items (`items[@h="1"]`), dropping filtered records before aggregation so hidden items disappear from row/column keys and all totals.
- Pivot table filtering: `pageField/@item` single-select page filters are now honored (resolves the selected item index to its shared-item index and drops non-matching records).
- Pivot table data-field number formats: `PivotDataField.numberFormat` is authored onto the `dataField` `numFmtId` (interned into workbook styles) and read back; the render engine applies it to value + total cells so currency/percent pivots match SpreadJS.
- Pivot table styling: the materialized grid now carries `style_index` so the header band (bold white on accent fill) and grand-total row/column (bold) render, matching the SpreadJS look.
- Pivot table rendering: the layout extractor now runs a self-contained aggregation engine over `pivotCacheRecords` (group-by + sum/count/avg/max/min/product/countNums/stdDev/var, with row/column/grand totals and Excel-style item sort) and materializes the value grid into the sheet, so the preview component renders pivot values/labels/totals instead of an empty grid. Supports one+ row fields, 0-1 column fields, one data field; no formula engine required.
- Pivot table authoring: `worksheet.pivots.set/list/remove` + `workbook.allPivots` (and Rust `set_pivot`/`pivots`/`remove_pivot`, WASM `setPivot`/`pivots`/`removePivot`) create a worksheet-source pivot with row/column/filter fields and sum/count/avg/etc. data fields; values compute in Excel/SpreadJS from the authored cache + materialized row/col item layout.
- `charts.set`/`charts.update`/`images.set` now accept a 1-based A1 range string (e.g. `"B20:H36"`) for `anchor`, normalized via `anchorA1`, alongside the 0-based `ChartAnchor`.
- `recalculate()` now returns only cells with an engine `fallback` (dropping error-free sheets) for cheap agent verification; pass `{ errorsOnly: false }` for the full cell-by-cell report.
- Export `colLetter`, `cellA1`, `rangeA1` ref helpers from the public API/entrypoints.
- `ChartAnchor` rustdoc + generated TS JSDoc now state the 0-based row/column convention.
- `Worksheet` row/column width/height/visible/insert/delete methods now document their 1-based row/column indexing in JSDoc.
- `ChartLegendPosition` rustdoc + generated TS JSDoc now document `"none"` and the `TopRight` ("Overlay Legend at Right") variant.
- `Range.setStyle` / `Cell.setStyle` / `Worksheet.setStyles` JSDoc now call out that merged ranges only persist style on the top-left anchor.
- `Workbook.search` accepts `includeHidden` (defaults to `true` for backward compatibility) to opt out of searching hidden / very-hidden sheets; documented in JSDoc + rustdoc.
- `Worksheet.setStyles(map)` applies a `{ reference: StylePatch }` map in one call for bulk styling.
- `NumberFormat` const exposes ECMA-376 §18.8.30 built-in format codes (`General`, `Percent2`, `Scientific2`, `DateTime`, `Accounting`, …) so agents can write `setStyle({ numberFormat: NumberFormat.Percent2 })` instead of handwriting `"0.00%"`.

- Workbook API redesigned as a hierarchical SpreadJS-style facade. `workbook.sheet(name)` / `workbook.worksheets()` / `workbook.activeSheet()` / `workbook.addSheet(name)` / `workbook.removeSheet(name)` return `Worksheet` objects. `Worksheet` owns per-sheet sub-collections (`merges`, `hyperlinks`, `comments`, `threadedNotes`, `dataValidations`, `conditionalFormats`, `autoFilter`, `tables`, `charts`, `images`, `sparklineGroups`) and leaf APIs (`freeze` / `pageSetup` / `protection`), plus structural verbs (`setRowHeight`, `insertRows`, …) and lifecycle (`rename`, `moveTo`, `remove`, `activate`, `setVisibility`). `sheet.range(addr)` / `sheet.cell(addr)` return `Range` / `Cell` with `.setValue` / `.setValues` / `.setFormula` / `.setStyle` / `.clear` / `.copyTo` / `.fillTo` / `.merge`. Workbook-scoped leafs: `workbook.definedNames` / `allTables` / `allCharts` / `allImages` / `allSparklineGroups` / `properties` / `calcProperties` / `protection`. `workbook.worksheets()` is the only sheet enumerator — all uniform `list/set/remove` or `get/set/remove`. Addresses accept A1 strings or `{row, column, rowCount?, columnCount?}`; sheet names are auto-quoted. `Worksheet` and its children share an internal `SheetRef` so `worksheet.rename(newName)` keeps existing handles valid against the new name.

- Chart authoring: `ChartSeriesPatch` / `ChartSeriesInfo` gain `dataLabels` for per-series data labels (overrides chart-level when set). Chart-level dl no longer duplicates onto every series in the OOXML.
- Image authoring: `ImagePatch` / `ImageInfo` gain `rotationDegrees` (°, normalized mod 360, written as `rot` in 60000ths on `<a:xfrm>`), `cropLeftPct`/`cropTopPct`/`cropRightPct`/`cropBottomPct` (% on `<a:srcRect>` l/t/r/b, 1000ths-of-a-percent units), and `flipHorizontal`/`flipVertical` (`<a:xfrm flipH/flipV>`). Round-trips through save/reopen. New `invalid_image` cases for non-finite rotation/crop values.
- Sparkline group authoring: `sparklineGroups(sheet?)` / `setSparklineGroup(patch)` / `removeSparklineGroup(sheet, id)` with `SparklineKind` (`line` | `column` | `stacked`), `SparklineEntry { location, dataRef }`, axis kinds + manual min/max, line weight, full color palette (series/negative/axis/markers/first/last/high/low). New `invalid_sparkline_group` error.
- Chart authoring: `ChartKind` gains `scatter` | `bubble` | `doughnut`. Scatter/bubble series take `xValuesRef` (required) + `valuesRef` (yVal); bubble adds `bubbleSizesRef` (required). Per-series solid color via `ChartSeriesPatch.color` (`RRGGBB` hex, `#` accepted). Axis titles via `ChartPatch.categoryAxisTitle` / `valueAxisTitle`. New `invalid_chart` errors for missing xVal on scatter/bubble, missing bubble sizes, and non-hex color.

- Workbook API: `charts(sheet?)` / `setChart(patch)` / `removeChart(sheet, id)` with `ChartInfo` + `ChartPatch` (+ `ChartKind` `column|bar|line|pie|area`, `ChartLegendPosition`, `ChartAnchor`, `ChartSeriesInfo`, `ChartSeriesPatch`) DTOs. Creates a chart part + drawings part if missing, anchors via two-cell (col/row + optional EMU offsets), authors title/legend/categories ref/series (literal name or `name_ref`, `values_ref`), and round-trips through save/reopen. New `invalid_chart` `ApiError` for empty series or empty `values_ref`.
- `Workbook.batch` now returns a `BatchOutcome { value, warnings, error }` envelope; `Workbook.warnings()` / `Workbook.takeWarnings()` expose an ambient warnings buffer. New `ApiWarning` DTO and `unsupportedFormula` / `unsupportedObject` / `lossyOperation` error codes round out the diagnostics contract.
- Conditional format authoring now covers color scales, data bars, and icon sets via new `colorScale` / `dataBar` / `iconSet` rule kinds and `ColorScalePatch` / `DataBarPatch` / `IconSetPatch` + `CfValueObject` / `CfValueObjectKind` (`num` | `percent` | `max` | `min` | `formula` | `percentile`) + `CfIconSetKind` (16 OOXML icon-set families) DTOs. 2- and 3-stop color scales, min/max data bars with min/max length + show-value, and 3/4/5-icon icon sets with show-value/percent/reverse round-trip through save/reopen. Validation enforces matching values/colors lengths, required cfvo values for num/percent/formula/percentile, and per-iconSet arity.
- Workbook API: `conditionalFormats(sheet)` / `setConditionalFormat(ref, patch)` / `clearConditionalFormats(ref)` with `ConditionalFormatRuleInfo` + `ConditionalFormatRulePatch` DTOs and `CfRuleKind` (`expression` | `cellIs` | `top10` | `duplicateValues` | `uniqueValues` | `containsText` | `notContainsText` | `beginsWith` | `endsWith` | `containsBlanks` | `notContainsBlanks` | `containsErrors` | `notContainsErrors` | `timePeriod` | `aboveAverage` | `colorScale` | `dataBar` | `iconSet`) + `CfOperator` enums. Dxf font/fill/border/numFmt/alignment patches author and intern into `<x:dxfs>`; new rules auto-assign priority (max+1). New `invalid_conditional_format` `ApiError` for missing required formula/operator/text or non-positive priority.
- Threaded notes now also write a legacy classic-comment shadow into `xl/comments<n>.xml` (author `tc=<guid-no-braces-lower>` per `<threadedComment id>`, text mirroring the note) so older Excel viewers see the thread. `comments()` filters `tc=`-authored entries out, `set_comment` / `remove_comment` preserve shadows, and `remove_threaded_thread` drops shadows at the affected refs. Comments/threaded-comments parts are now properly deleted (via `ws_part.delete_part_by_id`) when emptied, instead of being left as empty XML files.
- Workbook API: `setAutoFilterColumn(sheet, patch)` / `removeAutoFilterColumn(sheet, columnOffset)` with `AutoFilterColumnInfo` + `AutoFilterColumnPatch` + `AutoFilterCriteria` (`values` | `top10` | `custom` | `unsupported`) + `AutoFilterCustomCriterion` + `AutoFilterOperator` DTOs. `AutoFilterInfo` now exposes a `columns` list. `Top10` and `Custom` (≤2 criteria, AND/OR) round-trip; `Values` authoring is currently limited to blank-only (`<filters blank="1"/>`) because the underlying ooxmlsdk maps `<filters><filter/></filters>` and `<customFilters>` choice arms ambiguously across the `x` and `x14` namespaces — multi-value list filters should be expressed as `Custom` for now. New `invalid_auto_filter` `ApiError` for out-of-range column offsets, missing filter range, empty/unsupported criteria, and non-positive Top10 values.
- Workbook API: `pageSetup(sheet)` / `setPageSetup(sheet, patch)` / `removePageSetup(sheet)` with `SheetPageSetup` + `SheetPageSetupPatch` DTOs bundling `PageSetupSettings` (orientation, paper size, scale, fit-to-width/height, page order, copies, DPI, cell-comment + error printing), `PageMarginsInfo`, `PrintOptionsInfo` (centering, gridlines, headings), and `HeaderFooterInfo` (odd/even/first header+footer with `differentOddEven`/`differentFirst`/`scaleWithDoc`/`alignWithMargins`). New `invalid_page_setup` `ApiError` for out-of-range scale, zero copies, and negative/non-finite margins.
- Workbook API: `sheetProtection(sheet)` / `setSheetProtection(sheet, patch)` / `removeSheetProtection(sheet)` and `workbookProtection()` / `setWorkbookProtection(patch)` / `removeWorkbookProtection()` with `SheetProtectionInfo` + `SheetProtectionPatch` + `WorkbookProtectionInfo` + `WorkbookProtectionPatch` DTOs. Reads/writes `<sheetProtection>` and `<workbookProtection>` (legacy hex password + modern `algorithmName`/`hashValue`/`saltValue`/`spinCount`, all lock flags). New `invalid_protection` `ApiError` for non-hex passwords and empty credential fields.
- Workbook API: `tables(sheet?)` / `setTable(patch)` / `removeTable(name)` with `TableInfo` + `TablePatch` (+ `TableColumnInfo`/`TableColumnPatch`/`TableStyleSettings`/`TableStylePatch`) and a `TableTotalsFunction` enum. Upserts by workbook-unique name, creates `xl/tables/tableN.xml` + `<tableParts>`, infers/de-dupes column names from the header row, supports resize, header/totals rows, per-column totals function/label/formula, calculated column formulas, and `tableStyleInfo`. Existing tables still shift through row/col inserts/deletes. New `invalid_table` `ApiError` for missing/overlapping ranges, invalid names, geometry issues, and column-count mismatches.
- Workbook API: `dataValidations(sheet)` / `setDataValidation(ref, patch)` / `removeDataValidation(ref)` with `DataValidationInfo` + `DataValidationPatch` DTOs and `DataValidationType` (`list` | `custom` | `whole` | `decimal` | `date` | `time` | `textLength`), `DataValidationOperator`, and `DataValidationErrorStyle` enums. Writes `<dataValidations>`, merges sqref ranges (overlapping ranges on `set` are dropped from existing rules; rules emptied of all ranges are removed), supports input/error messages, prompts, and formula1/formula2. New `invalid_data_validation` `ApiError` for missing formulas/operator or incompatible field combos.
- Workbook API: `autoFilter(sheet)` / `setAutoFilter(ref)` / `removeAutoFilter(sheet)` with `AutoFilterInfo` DTO. Writes/clears the worksheet-level `<autoFilter>` range; filter criteria authoring deferred.
- Workbook API: `dependencies(ref)` / `precedents(ref)` / `dependents(ref)` with `DependencyInfo` + `DependencyReference` DTOs.
- Workbook API: `comments(sheet)` / `setComment(ref, patch)` / `removeComment(ref)` with `CommentInfo` + `CommentPatch` DTOs. Lazily creates the worksheet's `commentsN.xml` part (with the spreadsheetml namespace), upserts authors, replaces a comment on the same cell, deletes the part when emptied, and adds `invalid_comment` `ApiError` for empty text. Round-trips through save/reopen.
- Workbook API: `properties()` / `setProperties(patch)` with `WorkbookProperties` + `WorkbookPropertiesPatch` DTOs covering core file properties (`title`, `subject`, `creator`, `keywords`, `description`, `lastModifiedBy`, `category`, `contentStatus`, `identifier`, `language`, `revision`, `version`, `created`, `modified`, `lastPrinted`). Lazily creates `docProps/core.xml` (with `cp`/`dc`/`dcterms`/`xsi` namespaces) on first write, round-trips through save/reopen, and adds `invalid_property` `ApiError` for malformed timestamps. Also adds `calcProperties()` / `setCalcProperties(patch)` with `CalcProperties` + `CalcPropertiesPatch` DTOs and a `CalcMode` enum (`auto` | `autoNoTable` | `manual`) covering `calcMode`, `fullCalcOnLoad`, `forceFullCalc`, `calcOnSave`, `concurrentCalc`, `iterate`, `iterateCount`, `iterateDelta`, `fullPrecision`, `calculationId`.
- Workbook API: `definedNames()` / `setDefinedName(patch)` / `removeDefinedName(name, scope?)` with `DefinedNameInfo` + `DefinedNamePatch` DTOs; supports workbook- and sheet-scoped names, upsert by `(name, scope)`, hidden/comment metadata, OOXML round-trip via `<definedNames>`, and new `invalid_defined_name` `ApiError` for empty/whitespace/cell-ref-shaped names or empty formulas.
- Workbook API: `hyperlinks(sheet)` / `setHyperlink(ref, patch)` / `removeHyperlink(ref)` with `HyperlinkInfo` + `HyperlinkPatch` DTOs. Targets dedupe to existing worksheet relationships, overlapping hyperlinks on `set` are replaced, orphaned hyperlink relationships are deleted from `sheetN.xml.rels`, and `invalid_hyperlink` `ApiError` covers empty/missing target+location patches. OOXML `<hyperlinks>` block round-trips on save/reopen.
- Workbook API: `search(query, options)` across one sheet or all sheets, with `SearchOptions` (sheet/range scope, target = values/formulas/both, mode = substring/exact/wildcard/regex, case sensitivity, max results) and `SearchMatch` DTOs (`hit: value | formula`, matched substring, cell value, formula). Substring + case-insensitive defaults match `hsx search`. New `invalid_search_query` `ApiError` for empty queries and bad regex/wildcard patterns.
- `examples/xlsx-playground.html` browser mutation harness: open → mutate via a small JS editor (with snippet presets) → recalc → re-render via `createWorkbookPreviewer` → download `.xlsx`. Wired into the site build at `/playground`.
- Workbook API: `copyRange(src, dst)` / `fillRange(src, dst)` with relative-formula translation; copy supports same-shape, single-cell, or whole-multiple destinations (tiles source); fill requires dst to contain src and be a whole multiple; absolute markers and cross-sheet refs preserved; out-of-bounds collapses to `#REF!`.
- Workbook API: `insertRows` / `deleteRows` / `insertColumns` / `deleteColumns` with cell/merge shifting and formula reference rewriting (cross-sheet refs, absolute markers, ranges, column/row-only refs); deleted refs collapse to `#REF!`. Structural shifts now also rewrite workbook-level defined names (global + `localSheetId`-scoped), conditional formatting `sqref` ranges and rule formulas (cross-sheet aware), table `ref`/`autoFilter` ranges, and the worksheet-level `autoFilter` range.
- Workbook API: `clear(ref, mode?)` / `clearRange(ref, mode?)` accept a `ClearMode` (`all` | `values` | `formulas` | `styles`); `all` now also clears the cell's style index.
- Workbook API: `setRowHeight` / `setRowVisible` / `setColumnWidth` / `setColumnVisible` / `setFreeze` / `getFreeze` with `FreezeInfo` DTO and OOXML round-trip for row/col size+hidden and frozen panes.
- Workbook API: `moveSheet(name, toIndex)` / `setSheetVisibility(name, visibility)` / `setActiveSheet(name)` with `SheetVisibility` DTO (`visible` | `hidden` | `veryHidden`), active-tab tracking across moves/deletes, and refusal to hide the last visible sheet or activate a hidden one.
- Workbook API: `merges(sheet)` / `addMerge(range)` / `removeMerge(ref)` with `MergeInfo` DTO, overlap diagnostic (`merge_overlap` `ApiError`), and OOXML `<mergeCells>` round-trip.
- Workbook API: `setStyle(range, patch)` with font/fill/border/alignment/number-format sub-patches, interned styles, and `unsupported_style` diagnostic for invalid colors.
- Workbook API: `getRange` / `setRangeValues` / `setRangeFormulas` / `clearRange` with A1 range refs (`A1:B3`, sheet-qualified, absolute, reversed corners), shape validation (new `shape_mismatch` `ApiError`), and a `RangeInfo` DTO (row-major `values` + `formulas`).

### Fixed

- Recalc: genuine formula errors (#DIV/0!, #REF!, #NAME?, etc.) now surface as `{type:"error"}` cell values and are written as `t="e"` in the doc XML; engine-limitation kinds (#N/IMPL, #ERROR!) still fall back to cached values.
- Recalc: key fallback on cached value (not error kind) — cells with a non-blank file-cached value always fall back to it, fixing clobber of unsupported-function caches.

- Saving: inject missing `Default Extension="rels"`/`"xml"` content types (ooxmlsdk 0.7.0 omits them), fixing Excel repair-on-open for created workbooks.
- Styles: emit `<font>` children in CT_Font schema order.
- Editor: auto-close unbalanced `(` on commit (e.g. point-mode `=SUM(A1:A3`), avoiding corrupt formulas Excel rejects on open.

- Point mode: arrow keys (Shift to extend) pick/move a reference while editing a formula at a ref-accepting caret.
- Point mode: draw the in-progress candidate range box while dragging a reference (before the formula parses).
- Inline edit overlay: arrow keys commit and move the selection when typing a plain value (enter mode); F2/double-click keep arrows as caret movement.
- Inline edit overlay now positions in content coordinates inside the scroll spacer, fixing a scroll jump to the bottom-right of the sheet when focusing/typing.
- Example app: use `cell(addr).setValue/setFormula` instead of the nonexistent `range(addr)` single-cell setters.

- Table-header filter/sort dropdowns now act on the table's own sheet: `TableFilterArrow.rangeRef` is sheet-qualified (`TableSheet!A1:C5`), so controllers no longer fall back to `activeSheet()` and sort/filter the wrong sheet.
- `pivots.update()` no longer fails (`pivot field not found in source header: Values`) for multi-data-field pivots with a column field; the synthetic `-2` "Values" marker is stripped from the `PivotInfo` field read-back so the remove+reset round-trip is clean.
- `pageSetup.set` no longer corrupts a sheet-qualified `printArea`/`printTitleRows`/`printTitleColumns` (`"Sheet1!A1:C5"` was fused into `MAINA1:C5`); the existing qualifier is now stripped before re-prefixing.
- Single-sheet render (`--sheet`/`--sheet-index`) now resolves cross-sheet chart/sparkline refs by extracting cells-only grids of referenced sheets; previously such charts rendered as placeholders.
- Sparkline `dataRef`s are now sheet-qualified on write (`Sheet!B2:E2`); bare refs caused Excel to drop the whole sparkline group on open ("Removed Feature: Sparklines").

- `pivots.remove`/`pivots.update` no longer leak orphaned pivot cache parts + workbook `pivotCaches` registrations; removing a pivot now GCs its now-unreferenced cache.
- `Range.copyTo(otherSheet.range(...))` now returns a destination-scoped `Range` instead of a stale source-sheet handle.
- A failed default wasm initialization no longer poisons later `Workbook.create/open` attempts.
- Keep the browser entry (`dist/index.js`) free of `node:fs`: the default wasm resolver is now injectable via `registerDefaultWasmInputResolver`, registered by `@hewliyang/xlsx-preview/node` (which also re-exports `Workbook`/`NumberFormat`). Node consumers needing default-wasm bootstrap should import `Workbook` from `./node`.

- `Worksheet.shapes.set` now warns (`take_warnings`) when an anchor offset exceeds its referenced cell, since Excel clamps such offsets to the cell while our renderer treats them as absolute.
- Explicit column `width` → px now uses Excel's MDW rounding formula instead of a flat `width * pxPerChar`, so non-default column widths line up with Excel.
- Rotated/flipped shapes now emit `<a:off>`/`<a:ext>` on the shape `<a:xfrm>` (derived from the anchor), so Excel honors `rotationDegrees` instead of ignoring it. Rotation keeps the anchor footprint and rotates geometry inside it (matching Excel; fixture: `shapes_rotation_keeps_anchor_footprint`).
- Setting `font.name` now also drops any inherited theme `scheme` (minor/major), so an explicit font is no longer overridden by the theme body/heading font in Excel (e.g. "Aptos" no longer renders as "Aptos Narrow (Body)").
- `Workbook.create()` now ships a default `xl/theme/theme1.xml` (modern Office "Aptos" theme), so `scheme`/`theme`-indexed fonts and colors resolve correctly instead of falling back to Excel's app defaults.
- Pivot extract no longer paints the computed grid on top of the file's static cell values: cells inside a pivot's range are cleared before merging the engine-computed cells, so a filtered pivot that shrinks no longer leaves stale rows/overlapping text (regression test: `pivot_cells_do_not_duplicate_static_worksheet_cells`).
- Bar/column and line chart series `color` was silently dropped on write (only pie/scatter/bubble/doughnut/area persisted it); now authored and read back for all chart kinds.
- Chart series `color` now accepts 8-hex `AARRGGBB` (alpha stripped) in addition to 6-hex `RRGGBB`, matching every other color field.
- `Workbook.create()` / `Workbook.open()` from `@hewliyang/xlsx-preview/api` now load the bundled wasm via `readFileSync` on Node (auto-detected) instead of a `file://` URL that Node `fetch` rejects; no more manual `wasmBinaryUrl` plumbing required.
- `setHyperlink({ display })` now auto-populates the top-left cell's value with `display` when that cell is blank (no value/formula), matching Excel's Insert-Hyperlink behavior; renderers no longer show hyperlinked blank cells.

- ironcalc `DEFAULT_NUM_FMTS` rewritten against canonical ECMA-376 §18.8.30 (cleared OCR corruption like `"0.00E + 00"`, `"h:mm AM / PM"`, `"#,##0;()#,##0)"`) and switched from positional indexing to ID-keyed lookup so the spec's `numFmtId` gaps (5–8, 23–36, 41–44) are honored; new custom numFmtIds now start at 164 per Excel convention.
- `ChartCollection.update(id, partial)` merges a partial patch onto an existing chart so callers don't have to round-trip `ChartInfo` → `ChartPatch` and re-send every series to tweak one field.
- `AutoFilterApi.setColumnValues` / `setColumnTop10` / `setColumnCustom` typed helpers so agents don't have to hand-author the `criteria` discriminated union. `setColumn` now pre-validates `criteria.kind` and throws a clear error pointing at the helpers instead of the opaque wasm `missing field 'kind'`.

- Defined names are now wired into the recalc engine — formulas referencing ranges/cells via defined names resolve correctly instead of returning `#NAME?`. Non-reference defined-name formulas (scalars, expressions) emit a `LossyOperation` warning at `setDefinedName` since the engine only supports reference shapes.
- `Workbook.renameSheet()` now rewrites cross-sheet formula references in cells, conditional-formatting formulas, and defined names so renames no longer produce `#REF!`.
- `Workbook.save()` now auto-recalculates so cached formula values are always written; previously a `save()` without a prior `recalculate()` produced an xlsx whose formula cells rendered blank.
- `threadedNotes.add` no longer panics with `unreachable` on wasm — root cause was `std::time::SystemTime::now()` panicking on `wasm32-unknown-unknown` (`time not implemented on this platform`); replaced with `js_sys::Date::now()` on wasm. Panics elsewhere previously poisoned the `WorkbookHandle` (`recursive use of an object detected`); `console_error_panic_hook` is now installed so any future panic surfaces a real stack trace.
- `TableCollection.set` now qualifies unqualified `patch.reference` with the scoped worksheet name, instead of silently falling back to the workbook's first sheet (which was usually a hidden lookup sheet and produced wrong column names).
- `ClearMode` accepts `"formats"` as an alias for `"styles"` (matches Excel's "Clear Formats" wording).
- `ConditionalFormatCollection.set` defaults `dataBar.min` / `dataBar.max` to `{ kind: "min" }` / `{ kind: "max" }` when omitted (rust deserializer applies the same defaults).
- `Range.setValues` / `Range.setFormulas` now validate matrix shape (rectangular, non-empty, matches range dims when bounded) and throw `RangeError`/`TypeError` early instead of silently drifting.
- `recalculate()` now populates `RecalcCell.fallback` for every engine-produced error (`#REF!`, `#DIV/0!`, `#VALUE!`, `#NUM!`, `#N/A`, …), not just load-time misses; previously these errors arrived as plain `{type:"string",value:"#REF!"}` and health-check loops silently passed broken workbooks.
- Blank workbooks (no `xl/styles.xml`) now expose `defaultFont="Calibri"` / `defaultFontSize=11` in layout instead of empty/0 — previewer renders cell text on freshly-created workbooks without requiring a `setStyle` call.
- Playground `render()` honors `layout.activeSheetIndex` when a script calls `worksheet.activate()` (no longer pinned to the previously-active tab).

### Changed

- Internal: quote-aware sheet-reference qualification removed from the TS frontend; `Range`/`Cell`/`Worksheet` ops now pass `(sheet, ref)` to new Rust-owned `*_in` facade fns that qualify internally, so bindings are marshaling-only (pyo3/napi readiness). `api-refs.ts` no longer carries `qualify`/`hasSheetPrefix`/`quoteSheetName`; `refOnly`/`findUnquotedBang` deleted.
- Internal: `scripts/schema_diff.py` gains recursive one-level flattening, declared exclusions/derived (DTO `schema-excluded:` doc annotations + per-pair `scripts/schema_coverage.toml`), and a `--check` mode (non-zero on any undeclared MISSING field) over the opened-up (SdkStruct, DtoStruct) pairs. Serialization plumbing (`xmlns`/`xmlHeader`/`xmlOtherAttrs`) is globally ignored; non-chart writer pairs added (Table, DataValidation, ConditionalFormattingRule, Hyperlink, DefinedName) for writer action-space parity (44 pairs).
- Internal: CSV/Parquet option semantics (delimiter `tab`/single-byte coercion+validation, field defaulting) moved out of the wasm binding into `xlcore-tabular`; `CsvOptions`/`ParquetOptions` now `serde::Deserialize` directly (camelCase, string delimiter), so the bindings are marshaling-only and pyo3/napi get the same behavior for free.
- Internal: `setSheetVisibility` moved to the `api_methods!` table (`de` arg); serde owns the `SheetVisibility` parse/error instead of a hand-written match in the binding.
- Internal: wasm binding layer generated from a declarative `api_methods!` method table (~100 of 107 hand-written serde_wasm_bindgen fns); generated `.d.ts` is byte-identical, no behavior change. TS forwarding-layer codegen is a noted follow-up.
- Internal: `scripts/api_manifest.py` emits a checked-in `scripts/api_methods.json` method manifest (from the `api_methods!` table + hand-written `WorkbookHandle` methods); `--check` (wired as `check:api`) diffs the manifest and cross-checks that every forwarded `jsName` is called as `handle.<jsName>(` in TS and flags phantom `handle.<name>(` calls. The JSON is the contract a future pyo3/napi emitter consumes.
- API naming audit (see `docs/api-conventions.md`): drop the inconsistent `Api` class suffix and normalize wrapper class names to two cardinality-keyed suffixes — `<Concept>Collection`, `Workbook<Concept>`, `<Concept>Accessor`. Renames: `AutoFilterApi`→`AutoFilterAccessor`, `SheetFreeze`→`SheetFreezeAccessor`, `SheetPageSetupApi`→`SheetPageSetupAccessor`, `SheetPropertiesApi`→`SheetPropertiesAccessor`, `SheetProtection`→`SheetProtectionAccessor`, `WorkbookPropertiesApi`→`WorkbookPropertiesAccessor`, `CalcPropertiesApi`→`CalcPropertiesAccessor`, `WorkbookProtection`→`WorkbookProtectionAccessor`, `DefinedNamesCollection`→`WorkbookDefinedNames`. Method `ThreadedNotesCollection.removeThread`→`remove`. Instance accessors (`ws.freeze`, `wb.properties`, …) are unchanged.

- Sheet-scoped patches no longer carry a `sheet` field: `ChartPatch`, `ImagePatch`, `ShapePatch`, `PivotPatch`, `SparklineGroupPatch` lose `sheet`; the wasm/Rust facade fns (`set_chart`/`set_image`/`set_shape`/`set_pivot`/`pivot_preview`/`set_sparkline_group`) take `sheet` as their first argument uniformly. The TS `Omit<…, "sheet">` + re-inject plumbing in the shape/pivot collections is gone; `Info` types still report `sheet`.
- Sheet-qualification of unqualified refs moved into the Rust facade: collection/`Range` methods (`merges`, `hyperlinks`, `comments`, `threadedNotes`, `dataValidations`, `conditionalFormats`, `autoFilter`, `tables`) take `sheet` + a possibly-unqualified `ref` and qualify internally (`qualify_ref`); the TS `qref` helper is deleted so bindings stay marshaling-only.
- `autoFilter` criteria booleans (`blank`, `top10.top`, `top10.percent`, `custom.logicalAnd`) are now optional in the DTO and default in the Rust facade (`top` → `true`, the rest → `false`); the TS `setColumnValues/Top10/Custom` helpers stop applying their own defaults and `setAutoFilterColumn` returns the resolved criteria. The redundant `setColumn` `criteria.kind` guard is removed (serde + Rust `validate_criteria` already reject bad/unsupported kinds).
- `Range.setValues`/`setFormulas` matrix-shape validation now lives only in the Rust facade (`validate_matrix_shape`); the duplicate TS `validateMatrixShape`/`rangeDims` (which had divergent error messages) are removed, so shape errors come from a single source.
- `Workbook.recalculate({ errorsOnly })` filtering moved into the Rust/wasm facade (`recalculate(errors_only)`); the TS no longer post-filters the report. `search` likewise forwards options verbatim now that its defaults live in the serde `Default` impls.
- `conditionalFormats.set` dataBar `min`/`max` are now optional and default to `min`/`max` cfvo in the Rust facade (was defaulted in the TS collection); the returned info reflects the resolved values.
- `scripts/schema_diff.py` resolves DTOs across all `xlcore-types/src/*.rs` modules (was `lib.rs`-only, broken since the module split).
- Chart/image/shape `anchor` now accepts a two-cell A1 range string (`"D2:H15"`, optionally sheet-qualified) as well as an explicit `ChartAnchor`. The string→anchor resolution moved into the Rust facade (new `AnchorSpec` DTO), so the TS `normalizeAnchor`/`anchorA1` plumbing is gone from the collections and any future binding gets it for free.
- `Worksheet.pivots.update(id, partial)` is now pure forwarding to a Rust `update_pivot` + `PivotUpdate` DTO; the merge/remove/rollback logic moved out of TS so bindings stay marshaling-only.
- `Worksheet.charts.update(id, patch)` now mutates the existing `chart<n>.xml` in place (new Rust `update_chart` + `ChartUpdate` DTO) instead of remove+`setChart`. The chart's `rId`/id is now stable across updates, and chart XML not modeled by `ChartPatch` (rounded corners, manual layout, per-point styling, etc.) survives an update that only touches one field. Series/stacking/data-label/categories changes still rebuild the plot node; chart-level title/legend/axes and unmodeled siblings are preserved. Changing `kind` via `update` is no longer supported (use `remove` + `set`).

- `absoluteAnchor(x, y, w, h, { colWidthPx?, rowHeightPx? })` helper (exported from `./api` next to `anchorA1`) converts an absolute pixel rect into a two-cell `ChartAnchor` with in-cell EMU offsets, replacing hand-rolled px → (col, row, offset) math on the default 64×20 grid. Offsets are always strictly inside their cell, so results never trip the engine's anchor-overflow warning.
- CLI: `--no-headers` (cell content only — headerless renders no longer require a custom `node.ts` script), `--no-gridlines` (force gridlines off regardless of the sheet view flag, via the new `RenderOptions.renderGridLines` override), and `--width`/`--height` (explicit viewport in px; with `--no-headers` they are exact output dimensions).
- `renderToCanvas`/`renderToPng` accept `width`/`height` and an `onWarning` callback; the default viewport now auto-grows beyond the old hard 1244×822 cap to fit drawing extents (up to 4096px) and warns instead of silently clipping large charts/shapes. Headerless renders are cropped to the grid origin, removing the stray white header band.

- `Worksheet.shapes` collection (`list`/`set`/`remove`) to author DrawingML preset shapes (any of the 187 `prstGeom` presets) with solid fill, outline color/width, multiline text (color/size/bold/italic), rotation, and flip.
- `ShapePatch` gains `align`/`verticalAlign` (text alignment + body anchor), `underline`, and `headEnd`/`tailEnd` (`{type,w,len}`) for line arrowheads; shape arrowheads now render in the previewer.
- `Worksheet.setShowGridLines(visible)` / `getShowGridLines()` to toggle the per-sheet on-screen gridlines view flag.

- Sparkline color fields now accept `#RRGGBB` (or 8-hex `AARRGGBB`) in addition to `RRGGBB`; canonical stored form remains 6-hex uppercase.
- `DefinedNamePatch` / `DefinedNameInfo` rename `formula` → `reference` (defined names only support cell/range refs, so the old name was misleading). Legacy payloads using `formula` are still accepted at runtime via a serde alias on `DefinedNamePatch`; `DefinedNameInfo` now emits `reference`.

- Chart authoring now builds typed `c::ChartSpace` / `xdr::TwoCellAnchor` structs instead of raw XML string templates; reader path also uses typed `PlotAreaChoice` traversal. No behavior change to the public API or output OOXML shape.

## [0.0.10] - 2026-06-05

### Fixed

- Rows whose `<row>` element omits the `r` (row index) attribute — emitted by some producers such as SpreadJS — are no longer dropped. The extractor now infers the missing index sequentially from the last seen row, so all cells survive with correct row coordinates. Covered by the `producer-quirks/spreadjs-implicit-row-index.xlsx` fixture.

## [0.0.9] - 2026-05-28

### Added

- `tests/fixtures/charts/multilvlstr-cat.xlsx` regression fixture (hsx + Python zip-patch).
- CSV and Parquet preview support. New `format` / `csvOptions` /
  `parquetOptions` loader options route tabular files through the same
  `WorkbookLayout` renderer as XLSX. Browser and CLI sniff Parquet/XLSX byte
  signatures, then fall back to filename/MIME. Node gains
  `loadWorkbookFromCsv*` / `loadWorkbookFromParquet*` under the
  `@hewliyang/xlsx-preview/node` subpath. Parquet handles primitives,
  temporals, decimals, binary, and nested list/struct/map columns (large
  integers fall back to precision-preserving strings); CSV preserves
  leading-zero identifiers as strings and reports truncation. Backed by a new
  `xlcore-tabular` Rust crate.
- Package test/build hardening: `pnpm test` now checks for stale wasm, rebuilds
  TS from a clean `dist`, verifies built public entries can be imported, and
  exercises committed CSV/Parquet fixtures. Added `smoke:csv` and
  `smoke:parquet` scripts for shipped-build PNG smoke tests.
- Structured load-error envelope and `LoadReport`. The wasm extractor now returns `{ layout, report }`; `loadWorkbookFromFileWithReport` / `loadWorkbookFromArrayBufferWithReport` / `loadWorkbookFromXlsxWithReport` expose the report alongside the layout. Failures throw `XlsxLoadError` with `code` (`Zip` | `Schema` | `MissingPart` | `Io` | `Other`), `part`, `schemaKind`, `ty`, `field`, `value` and a `diagnosticsText()` helper. CLI gains `--verbose` (print fixes/warnings) and `--strict` (exit `2` if the loader had to coerce attributes _or_ skip a fixer).
- React: `ExcelPreviewer` renders a default error card on load failure and a dismissible "leniency" chip when the load report is non-clean. Override via `renderError`, hide via `hideErrorUI` / `showLeniencyChip={false}`.

### Changed

- All examples (build-based and the pinned no-build CDN demo) now accept
  `.csv`, `.tsv`, `.parquet`, and `.pqt` in their file pickers and target
  `@hewliyang/xlsx-preview` `0.0.9`.
- **Breaking (internal worker protocol):** the bundled extraction worker now posts `{ type: "loaded", layout, report }` instead of `{ type: "layout", layout }`, and errors are posted as `{ type: "error", payload: XlsxLoadErrorPayload }` instead of `{ type: "error", message }`. The wasm-bindgen export `extract_xlsx` now returns `{ layout, report }` instead of `layout`. Consumers using the public `loadWorkbookFromFile` / `createWorkbookPreviewerFromFile` APIs are unaffected; anyone embedding the worker or calling `extract_xlsx` directly must update.

### Fixed

- React `useWorkbookPreviewer` reloads when load-affecting options change for
  the same `File` (`format`, sheet selectors, tabular options, worker/wasm
  URLs, initial sheet/zoom/hidden-sheet options) while callback-only changes
  still update through refs without forcing reloads.
- Built package subpaths no longer miss runtime files: `dist/sourceFormat.js`
  and `dist/node.js` are emitted and covered by import checks.
- The root package entrypoint no longer re-exports Node-only helpers, keeping
  `@hewliyang/xlsx-preview` browser-safe. Server-side rendering and file
  loading helpers are available from `@hewliyang/xlsx-preview/node`.
- When extracting a single sheet by name or index, `defined names` with a
  `local_sheet_id` are now filtered to the selected sheet and remapped to
  local id `0`, so chart series referencing local names resolve correctly.
- Chart category axes that use `<c:multiLvlStrRef>` (emitted by Microsoft Office budget/dashboard templates when the category source spans multiple rows, often with a malformed cache that lists N levels each with `ptCount=1` and one `pt idx=0`) no longer produce a borked preview. The `CategoryAxisDataChoice` match in `xlcore-export` used to drop `CMultiLvlStrRef` through a wildcard arm, losing both the categories *and* the formula reference; the resolver in `refs.rs` then had nothing to backfill from. We now surface the formula ref so the resolver can read categories from the actual sheet range, and the renderer guards `chart.categories[i]` in `drawCategoryAxis`. Repro: `12-month-budget-template.xlsx` (Microsoft template) previously threw `Cannot read properties of undefined (reading '0')` and dismissing the error left a half-rendered preview.
- Hierarchical multi-row `<c:multiLvlStrRef>` category bands now render in the preview, matching hsx / Excel desktop. `refs.rs::resolve_chart_refs` reuses `cx_category_levels` for non-chartex charts when the formula range is `n_rows > 1 && n_cols > 1` — one inner Vec per source row, in source order so `levels[0]` is the outermost (top) band and `levels[last]` mirrors the innermost (bottom) row that `chart.categories` already carries. Single-column multi-row ranges keep the flat fallback. The renderer gained `categoryAxisExtraRows` / `categoryAxisExtraHeight` / `drawCategoryAxisExtraRowsCentered` helpers in `chartUtils.ts`; `drawAxisFrame` + `drawCategoryAxis` enlarge `xAxisH` and stack outer rows underneath the innermost band, and the bar/column inline path in `chart.ts`, `chartCombo.ts`, and `chartStock.ts` all paint the extras centered on each category slot. Horizontal bar charts skip extras since categories live on the y-axis. Covered by the `multilvlstr-cat.xlsx` fixture.
- `Chart.categories` is now always emitted in extracted JSON (previously elided when empty), matching the ts-rs `Array<string>` binding. The schema drift was the proximate cause of the multiLvlStrRef crash above and a latent footgun for any future code path that produces empty categories.
- CLI now prints `error.stack` (not just `error.message`) for non-`XlsxLoadError` failures, so future opaque errors like `Cannot read properties of undefined (reading '0')` arrive with a usable stack frame.

## [0.0.8] - 2026-05-20

### Added

- Multi-segment elbow connectors `bentConnector2`, `bentConnector4`, and `bentConnector5`, in addition to the existing `bentConnector3`. `ShapeNode` gains an `adj3` slot, and the extractor reads `<a:avLst><a:gd name="adj3">`. Path geometry follows ECMA-376 Appendix D; `flipH`/`flipV` and arrowheads honored.
- Full `<a:lstStyle>` cascade for DrawingML shape text. Resolves `<a:lstStyle><a:defPPr>` → `<a:lstStyle><a:lvl{N+1}pPr>` → paragraph `<a:pPr>` → run `<a:rPr>` for `algn`, `marL`/`indent`/`lvl`, `<a:lnSpc>` (spcPct + spcPts), `<a:spcBef>`/`<a:spcAft>`, and bullet choices (`<a:buNone/>`, `<a:buChar>`, `<a:buAutoNum>` with `<a:buFont>`/`<a:buClr>`/`<a:buSzPct>`/`<a:buSzPts>`). Run cascade now uses tristate semantics so explicit `u="none"`/`strike="noStrike"` disable inherited values. `<a:rPr kern>` and `<a:rPr baseline>` (super/sub at 65% scale) are honored. Multi-line text past the body rect now clips at the body bottom.
- All 186 ECMA-376 Appendix D preset shapes render through a generic spec-driven evaluator (`presetShapeEval.ts`) instead of collapsing to a plain rectangle. The spec XML is converted at build time to a generated TS table; the runtime evaluates §20.1.9.6/§20.1.9.7 builtins, av defaults and overrides, all 17 formula ops, and traces `moveTo`/`lnTo`/`arcTo`/`quadBezTo`/`cubicBezTo`/`close`. Existing hand-rolled fast paths are retained.
- Shape-level click hyperlinks (`<xdr:cNvPr>/<a:hlinkClick>` on `<xdr:sp>`, `<xdr:pic>`, `<xdr:grpSp>`, `<xdr:cxnSp>`). Extractor surfaces `DrawingHyperlink { target, tooltip? }`; the renderer hit-tests shape bboxes before cell hyperlinks, sets a pointer cursor, opens external links via `window.open`, and dispatches `"xlcore-hyperlink-jump"` for internal targets.
- Drawing absolute anchors (`<xdr:absoluteAnchor>`) supported in both the Rust exporter and the TS/WASM previewer. The previewer maps absolute coordinates into the grid layout space.
- DrawingML `<a:blipFill>` on `<xdr:sp>/<xdr:spPr>` (shape-as-image-fill, distinct from `<xdr:pic>`) plus the modern-Office `asvg:svgBlip` SVG sidecar. `ShapeNode.fill_blip` carries `{ dataUri, srcRect?, kind? }`; the SVG sidecar wins over the raster fallback when present. `<a:srcRect>` honored; `tile` is parsed but painted as `stretch` for v0. The painter clips to the preset path before drawing.
- DrawingML text overflow (`<a:bodyPr vertOverflow=… horzOverflow=…>`). `vertOverflow=overflow` (the spec default) paints every line whose top starts inside the body rect; `clip` and `ellipsis` clip to the inner rect; `ellipsis` additionally rewrites the last fully-visible line's tail with `…`.
- DrawingML text autofit (`<a:normAutofit fontScale lnSpcReduction>` / `<a:spAutoFit/>`). New `ShapeNode.textAutofit`, `textFontScale`, `textLineSpaceReduction` fields; the painter scales every run's font size and line height when `textAutofit === "norm"`. `spAutoFit` is recorded for round-trip but is a no-op at paint time (shape `ext` already reflects the author-time fit).
- DrawingML `<a:avLst>` adjust values honored on `roundRect`, the four cardinal arrows (`leftArrow`/`rightArrow`/`upArrow`/`downArrow`), and `leftRightArrow`. `roundRect` corner offset = `min(w,h) * clamp(adj1, 0..50000) / 100000` (spec default 16667). Cardinal arrows use `adj1` for tail thickness and `adj2` for head length; `leftRightArrow` caps each head at `w/2`.
- DrawingML line `cap`/`join`/`prstDash` honored on non-connector shape outlines. New `ShapeNode.lineCap`/`lineJoin`; the painter maps `flat`/`sq`/`rnd` to canvas `butt`/`square`/`round`. Brace-like presets keep their forced `round` cap+join only when no explicit value is set.
- Shape style-ref matrix walk. The theme's `<a:fmtScheme>` (`<a:fillStyleLst>`/`<a:lnStyleLst>`/`<a:effectStyleLst>`) is threaded through `resolve_style_refs`, so `fillRef idx≥1` resolves to the themed solid/gradient (idx=2 subtle, idx=3 strong on the standard Office theme), `lnRef idx≥1` picks up per-style width + dash, and `effectRef idx≥1` resolves a themed `<a:outerShdw>`. The `phClr` placeholder is substituted with the shape's `<*Ref>` color and modifiers (tint/shade/lumMod/lumOff/satMod/satOff/alpha).
- DrawingML `<a:effectLst><a:outerShdw>` on shapes. Parses `blurRad`/`dist`/`dir` plus color (`srgbClr`/`schemeClr`/`prstClr`/`sysClr` with `<a:alpha>` modifier). Painter maps `dist`/`dir` to canvas `shadowOffsetX/Y` and `blurRad` to `shadowBlur`; the shadow is cleared before stroking so the outline isn't double-shadowed. `algn` and `rotWithShape` ignored; `effectDag` deferred.
- DrawingML `<a:gradFill>` for shape fills. Reads `gsLst` stops (with theme color resolution and modifiers), `lin@ang` for linear gradients (1/60000 deg), or `path@path` + `fillToRect` for path gradients. Painter materializes them via `createLinearGradient` / `createRadialGradient`.
- Worksheet-level `<autoFilter ref="…">` chrome: surfaces as `Sheet.autoFilterRange`, paints header dropdown chevrons, honors row `hidden` flags.
- Browser previewer follows in-workbook hyperlinks: navigation buttons switch sheets, select / scroll to the target cell, and resolve bare workbook/sheet defined-name targets. External links still open in a new tab.
- DrawingML `<xdr:cxnSp>` connectors + bare `prstGeom=line`/`lineInv`. Honors `flipH`/`flipV`, `prstDash` patterns, five arrowhead kinds (triangle / stealth / diamond / oval / open) with `w`/`len` sizing, and straight / `bentConnector3` Z / diagonal routing.
- `<xdr:cxnSp>` `<a:stCxn>` / `<a:endCxn>` endpoint resolution against target shape bboxes (cardinal indices `0..=3` only). New `ShapeNode.elbowAxis` lets `bentConnector3` pick the correct bend orientation when multiple connectors share an endpoint.
- DrawingML brace/bracket presets (`leftBrace`, `rightBrace`, `leftBracket`, `rightBracket`) with quadratic-bezier corner arcs; reads `adj1` (corner curl) and `adj2` (tip Y).
- `adj2` extraction on every `xdr:sp` (previously connectors + `adj1` only).
- DrawingML shape text honors `<a:bodyPr lIns/tIns/rIns/bIns/>` insets via new `ShapeNode.textInsetsEmu`, replacing the previous 4%-of-shape default. Fixes single-character vertical-strip text in narrow autoshapes.
- DrawingML `<a:lstStyle>` + paragraph `<a:pPr><a:defRPr>` cascade for run + paragraph properties in spec precedence order (initial scope: size, bold, italic, underline, strike, solidFill, latin font; paragraph `algn`).
- DrawingML `<a:fld>` (text field) runs extracted alongside `<a:r>`, going through the same property cascade. Field runs are how Excel caches values for shape text bound to a cell via `textlink`.

### Changed

- Split `crates/xlcore-export/src/shapes.rs` into `shapes` / `shapes_style` / `shapes_text` and `packages/xlsx-preview/src/shape.ts` into `shape.ts` + `shapePaths.ts`. Pure code motion.

### Fixed

- `body_wrap_token` was matching the Debug repr of `TextWrappingValues` against the string `"None_"` (no such variant), so `<a:bodyPr wrap="none">` always fell through to the default `square` wrap. Replaced with a direct enum match.
- DrawingML preset dash tokens for the long-variant family (`lgDash`, `lgDashDot`, `lgDashDotDot`, and the `sysDash*` siblings) were emitted as `"largeDash"` / `"systemDash*"` (lowercased Rust enum names) and silently fell through the painter's `dashPattern` switch. Replaced with an explicit `prst_dash_token` matcher over the SDK enum, shared between `shapes.rs` and `fmt_scheme.rs`.
- Browser preview virtualization now keeps merged-cell extents in the grid even when the merge runs beyond the current viewport. Fixes wrapped text disappearing in visible merged cells at high zoom.
- Shapes with no `<a:xfrm>` (or `<a:xfrm>` carrying only flip/rot attrs and no off+ext) were silently dropped. `shape_world` / `connector_world` now fall back to a unit-box outer normalized to the anchor rect.
- Non-connector `flipH` / `flipV` honored end-to-end: the extractor reads the xfrm attrs on every `<xdr:sp>` (previously hardcoded `None`), and the painter applies `ctx.scale(±1,±1)` around the shape centre and unflips before text so captions stay readable.
- Group rotation (`<a:xfrm rot>` on `<xdr:grpSpPr>`) propagates to children as a rigid body. `GroupFrame` was replaced with a 2D affine `Frame` that maps a group's child-coord-space directly to world EMU and composes through nested groups; parent rotation is merged into each node's `rotation`. Single-level rotation is exact; nested rotated groups are approximate (composition collapsed to a single rotation around the inner pivot). Group `flipH`/`flipV` is parsed but not yet propagated.
- Shape text body rect follows `flipH`/`flipV`: `drawShapeText` now mirrors `presetTextRect` within the shape bbox before placing paragraphs, so captions on asymmetric presets (right-arrow, pentagon, callouts) sit over the visually-correct half after a flip. Glyphs themselves stay un-mirrored by design.
- Cut per-frame redraw cost on large sheets with conditional formatting by ~17× (89ms → 5ms median on a 10k-row workbook with 5 CF rules). Viewport-independent CF work (`iterAllCells` passes, full-sheet predicate evaluation, color-scale stop resolution, data-bar bounds, merge-map construction) is now memoized per `(sheet, layout)` (and per `CfRule` for color-scale / data-bar precomputes) via `WeakMap`s. Full-map scans of `cfDxfs` and `cfIconDraw` were replaced with visible-rect iteration + `Map.get`.
- Cell `left` / `right` borders no longer cut through the source cell's own overflowing centered/aligned text. New `computeOverflowSuppressedSides` pre-pass; `drawCellBorders` takes an optional suppressed set. Merged / rotated / wrapped / multi-line cells unaffected.
- DrawingML preset names now come from the SDK's `as_xml_str()` instead of Rust enum variant debug names, so `roundRect`, `lineInv`, `homePlate`, `hexagon`, `star5`, `leftRightArrow`, `flowChartDecision`, etc. reach the renderer instead of falling through to plain rect.
- `shape.ts::pathForPreset` adds paths for `roundRect`, `chevron`, `homePlate`/`pentagon`, `hexagon`/`octagon`, `star4`/`star5`/`star6`/`star8`, `leftRightArrow`, and `flowChartDecision`.
- Shape text uses a per-preset text rect (`presetTextRect`) for non-rect shapes so labels sit inside the painted region.
- `wrapParagraph` falls back to char-by-char breaking when a single non-space token exceeds the line width — needed for narrow shapes (chevron, hexagon, triangle, decision).
- Wrapped shape text past line 1 was dropped on centered short boxes whose two lines were ~1px taller than the body. Replaced the strict clip with spec-default `vertOverflow="overflow"`: a line paints as long as its top starts inside the body rect.
- `<a:rPr u="…"/>` and `strike="…"` treated as enums, not bools. `u="none"` / `strike="noStrike"` no longer render underlined / struck-through.
- Explicit OOXML column-width conversion no longer adds ~5px of padding, removing horizontal drift in button-heavy / instruction worksheets.
- Centered / right-aligned text overflow stays anchored to the source cell/box; the clip region may still grow into empty neighbours, but alignment is no longer recentered inside the expanded band.
- Centered single-line labels with literal leading/trailing spaces paint using the trimmed visible text, matching Excel-style nav buttons.

## [0.0.7] - 2026-05-17

### Added

- chartEx (`cx:`) `regionMap` ("Filled Map") painter
  (`chartExRegionMap.ts::drawRegionMapChartEx`). Bring-your-own world
  geometry: Natural Earth 110m admin_0 countries, slimmed +
  2-decimal-rounded into `packages/xlsx-preview/src/world110m.ts`
  (~170KB; regeneration snippet in the painter file header). The
  Bing-encoded `<cx:binary>` geoCache blobs Excel ships are
  deliberately ignored. Three pieces:
  - **Rust extractor** (`crates/xlcore-export/src/charts.rs`):
    (1) `parse_series_data` accepts `<cx:numDim type="colorVal">`
    alongside `val` / `size`; (2) `extract_chart_ex` picks the
    first non-`hidden="1"` series for `regionMap` layouts (Excel
    ships up to 4 alternate-preset series, only the last is
    visible); (3) new `extract_region_map_colors` parses
    `<cx:valueColors>` 2- or 3-stop palettes, resolving
    `<a:srgbClr>` literals + `<a:schemeClr>` theme refs (with
    modifier-chain support reused from `apply_color_modifiers`)
    into `cx_region_map_{min,mid,max}_color`.
  - **Schema**: `Chart` gains three optional fields
    (`cx_region_map_{min,mid,max}_color`). TS bindings
    regenerated via `scripts/regen-schema.sh`.
  - **TS renderer**: equirectangular projection with 1:1 lon/lat
    aspect; lat clamped to `[-58, 84]` so the world fills the
    rect; country-name lookup over NAME / NAME_LONG / ISO_A2 /
    ISO_A3 plus a small alias table (USA, UK, UAE, DRC, Czechia,
    Burma → Myanmar, Côte d'Ivoire, ...); palette honors authored
    3-stop diverging (e.g. blue→red→green) or 2-stop linear from
    the schema, falling back to a near-white → accent1 sequential
    ramp when no `<cx:valueColors>` was authored; gradient legend
    bar on the right with min/max labels; unmatched countries
    paint a neutral gray base layer. hsx falls back to a
    clustered column chart for this layout, so xlsx-preview now
    wins it outright. Fixture:
    `tests/fixtures/charts/chart-regionmap-chartex.xlsx` (covers
    both 2-color sequential and 3-color diverging palettes via
    its two sheets).
- DrawingML shape parity: word-wrap inside shape text bodies +
  nested pictures inside group shapes. Previously the shape painter
  emitted single-line runs that overflowed the box on anything
  longer than a step number, and `<xdr:pic>` children of
  `<xdr:grpSp>` were silently dropped — the Microsoft Map Chart
  template's NOTE paragraph ran off-right and the Maps-ribbon /
  `+`-button / arrow / columns-collapsed thumbnails inside its
  grouped callouts were missing entirely. Three pieces:
  - **Rust schema**: `ShapeNode` gains `text_wrap` (from
    `<a:bodyPr wrap="square|none"/>`), `image_data_uri`, and
    `image_src_rect` (4-int `<a:srcRect l t r b/>` crop in 1/1000
    percent of the source image).
  - **Rust extractor** (`shapes.rs`): new `visit_picture` arm in
    the group walker dereferences the picture's `r:embed` through
    a pre-built `rid → data:` URI map (constructed once per
    drawing in `charts.rs::extract`) and emits a leaf `ShapeNode`
    with the data URI plus the optional crop array. Top-level
    pictures still route through `AnchorTarget::Image`. Also
    surfaces `<a:bodyPr wrap="...">` via a new `body_wrap_token`
    helper.
  - **TS renderer**: new `imageCache.ts` extracted from
    `drawings.ts` so `shape.ts` can share the decoded-image cache;
    `drawShapeNode` dispatches image-bearing nodes to a new
    `drawShapeImage` (honors `srcRect` via the 9-arg
    `drawImage(s, sx, sy, sw, sh, dx, dy, dw, dh)` form);
    `drawShapeText` rewritten with proper paragraph word-wrap —
    tokenizes runs into `\S+\s*|\s+` atoms, measures with the
    active font, breaks atoms that would overflow inner width,
    preserves hard `\n` breaks, vertically anchors the wrapped
    block via `textAnchor`, trims trailing whitespace for
    center/right alignment. Wrap policy from `node.textWrap`:
    `square` (Excel default, absent attr) wraps; `none` lets text
    run on. `preloadDrawingImages` walks shape nodes too so the
    Node `renderToPng` path sees embedded thumbnails on first
    paint.
  - Verified on the existing
    `tests/fixtures/charts/chart-regionmap-chartex.xlsx` fixture
    (no new fixture needed — the Microsoft Map Chart template
    already exercises every code path). `docs/PARITY.md` Shapes
    row updated; remaining deferred items are gradient/blip/
    pattern shape fills, `<xdr:cxnSp>` connectors, and `avLst`
    adjust-value overrides on preset arrows (none triggered by
    current fixtures).

- Resolved custom `<tableStyles>` definitions. Previously the
  renderer only understood Excel's built-in style names
  (`TableStyleMedium2`, etc.) and inferred the accent color from the
  trailing digit; workbooks authored with a custom-named style — e.g.
  Microsoft's `Excel_TipsTableStyle` from the public Map Chart
  template — fell back to accent1 (blue) regardless of what the style
  actually pointed at, so a green-themed header rendered blue. Three
  pieces:
  - **Rust schema**: new `WorkbookLayout.tableStyles:
    Vec<CustomTableStyle>`. Each entry carries the style `name` plus
    `dxfId` references for the bands we paint (`wholeTable`,
    `headerRow`, `totalRow`, `firstRowStripe`, `secondRowStripe`,
    `firstColumn`, `lastColumn`). Bands we don't render yet (column
    stripes, subtotal rows, page-field cells) drop on the floor; add
    fields as the renderer grows.
  - **Rust extractor**: new `extract_table_styles` in `styles.rs`
    walks `<x:tableStyles>/<tableStyle>/<tableStyleElement>` and
    populates the named slots. Wired into `lib.rs` alongside
    `extract_dxfs`.
  - **TS renderer**: `computeTableState` now takes the workbook layout
    and resolves custom styles by name. Resolution order is (1) custom
    `<tableStyles>` lookup → dxf overlay via the existing `cfDxfs`
    pipeline, (2) built-in name heuristic fallback. Each band falls
    back independently — a custom style that only defines `headerRow`
    still gets synthesized row stripes. New helpers: `mergeDxf`
    stacks `wholeTable` underneath the band-specific overlay per
    ECMA-376 §18.8.40.
  - Fixture: `tests/fixtures/charts/chart-regionmap-chartex.xlsx`
    (slimmed copy of Microsoft's public "Map Chart samples.xlsx";
    two ~19MB `<cx:binary>` Bing geoCache blobs stripped — our
    renderer doesn't consume them). The fixture is primarily there
    to unblock the chartEx regionMap painter (still TODO) but the
    table-header bug surfaced as collateral while staging it.
  - Not yet honored: per-table direct overrides on `<table>` itself
    (`headerRowDxfId=""`, `dataDxfId=""`). These stack on top of the
    table style and would be a small follow-up.

- Surfaced a `regionMap` chartEx fixture. Microsoft's public "Map
  Chart samples.xlsx" template, slimmed from 40MB → 77KB by
  stripping the two `<cx:binary>` Bing geoCache blobs (our pipeline
  doesn't decode Bing's proprietary polygon encoding; the geometry
  will come from an embedded world-countries dataset when we ship
  the painter). The extractor already routes the layout through
  `drawChartEx` with `cx_layout="regionMap"`, but the painter still
  falls through to `drawPlaceholderPlot`. Notably hsx also falls back
  here — it renders the regionMap as a clustered column chart, not
  as a real map — so a future choropleth painter would beat hsx on
  this layout, not just match it. Fixture lives at
  `tests/fixtures/charts/chart-regionmap-chartex.xlsx`; painter design
  notes in `docs/parity-charts.md` priority #8.

- Added chartEx (`cx:` namespace) histogram / pareto / boxWhisker
  painters. All three are clear wins over hsx, which mis-renders each
  of them (histogram as raw bars, pareto as duplicated clustered
  columns, boxWhisker as a clustered column chart). Three pieces:
  - **Pre-parse normalization**: `xmlns_normalize` now rewrites
    `<cx:axisId val="N"/>` (the attribute form Excel desktop emits in
    chartEx parts to bind series to primary/secondary axes) into the
    `<cx:axisId>N</cx:axisId>` text-child form ooxmlsdk's chartEx
    schema expects. Without this, the pareto fixture crashes the
    chartEx parse entirely (`invalid field 'cx_axis_id' while parsing
    Series: ""`).
  - **Rust extractor**: `extract_chart_ex` now walks every
    `<cx:series>` (not just the first) and detects three multi-series
    / layoutPr-flagged compositions: any `paretoLine` companion
    promotes the chart to `cxLayout="pareto"`; an all-`boxWhisker`
    series list becomes `cxLayout="boxWhisker"`; a single
    `clusteredColumn` series whose `<cx:layoutPr>` carries
    `<cx:binning>` becomes `cxLayout="histogram"`.
  - **TS renderer**: new `chartExStats.ts` module (carved out of
    `chartEx.ts` to stay under the per-file LoC budget).
    `drawChartEx` dispatches on the new `cxLayout` values:
    - **Histogram**: Sturges bin count (`ceil(log2 n) + 1`),
      width rounded up to a nice `1/2/5 × 10^k` number so labels
      read as 10/20/50 rather than 9.7-and-change. Bars touch
      (`gapWidth=0`); right-closed `(low, high]` bin labels with the
      leftmost bin shown as `[low, high]` to flag its left-closed
      corner.
    - **Pareto**: primary `clusteredColumn` bars on the left value
      axis (accent1) plus a cumulative-% line on a synthesized right
      0–100% axis (accent2). The line series carries no own data in
      OOXML — cumulative % is computed from the primary series's
      values at render time, with the first point anchored at the
      origin so the line visually starts from the axis baseline.
    - **boxWhisker**: per-series quartiles computed with
      `QUARTILE.EXC` semantics (the chartEx default
      `quartileMethod="exclusive"`), 1.5×IQR whisker fences, outlier
      dots, median rule, and an × mean marker (default-on for
      chartEx). Each series renders as one vertical box centered in
      its slot with the series name as the category label.
  - Fixtures: `chart-{histogram,pareto,boxwhisker}-chartex.xlsx`
    (Excel-desktop-authored; SpreadJS round-trip is unreliable for
    these three layouts — see `build-chartex.sh`).

- Added chartEx (`cx:` namespace) funnel / treemap / sunburst painters
  alongside the existing waterfall arm. Three pieces:
  - **Rust extractor**: `extract_chart_ex` now accepts
    `<cx:numDim type="size">` alongside `type="val"` (treemap /
    sunburst encode rectangle / ring area in `size`, not `val`).
  - **Chart-ref resolver**: multi-column `categories_ref` ranges
    (e.g. `Sheet1!$A$2:$B$10` for a region→country hierarchy) are
    materialized into a new `cxCategoryLevels: string[][]` schema
    field — one inner array per nesting level, parallel to the
    values vector. The legacy 1D `categories` field gets the
    innermost (leaf) column for backward compat.
  - **TS renderer**: new `chartEx.ts` module (carved out of
    `chartAdvanced.ts` once chartEx surface area passed the per-file
    LOC budget; `chartStock.ts` likewise split). `drawChartEx`
    dispatches on `cxLayout`:
    - **Funnel**: center-aligned horizontal bars, widths scaled to
      the max value, per-bar value labels (suppressed when they'd
      overflow), category labels in a left gutter.
    - **Treemap**: squarified layout (Bruls/Huijsen/van Wijk 2000).
      Hierarchical mode groups leaves by `cxCategoryLevels[0]`,
      lays out parents across the full plot, then squarifies
      children inside each parent rect. Each branch gets one theme
      accent color; children share parent color, separated by white
      borders. Parent labels sit in the top-left of each group rect.
    - **Sunburst**: ring-per-level polar layout (innermost ring =
      level 0). DFS keeps sibling wedges angularly contiguous; per-
      branch theme accent with innermost-ring darken; tangentially-
      rotated slice labels with overflow suppression.
  - `chart.ts` suppresses the trivial single-series legend
    (`"Count"` / `"GDP"` / `"Sales"`) for these three layouts —
    Excel/hsx hide the legend too. Waterfall's synthetic three-swatch
    legend (Increase / Decrease / Total) is unchanged.
  - Fixtures: `tests/fixtures/charts/chart-{funnel,treemap,sunburst}-
    chartex.xlsx`, authored via SpreadJS (`hsx eval`) per
    `tests/fixtures/charts/build-chartex.sh`. ChartEx pareto /
    boxWhisker / clusteredColumn (histogram) / regionMap still need
    Excel-desktop authoring (SpreadJS export round-trip is unreliable
    for those four — missing `<cx:axis>` blocks, no auto-binning,
    degenerate render-as-cluster). See `docs/parity-charts.md`.
- Added chartEx (`cx:` namespace, Office 2016+) waterfall support —
  previously the largest remaining chart-parity gap. End-to-end pipeline:
  - **Rust I/O**: enabled ooxmlsdk's `mce` feature and added a textual
    `<mc:AlternateContent>` unfold in `xlcore-io::xmlns_normalize` for
    drawing parts (Excel always wraps chartEx graphic frames in MC for
    old-Excel fallback, and ooxmlsdk's typed `two_cell_anchor_choice`
    never sees MC contents otherwise).
  - **Rust extractor**: new `xlcore-export::charts::extract_chart_ex`
    resolves `drawings_part.extended_chart_parts()` and surfaces
    `chart_type="chartex"`, `cxLayout` (waterfall / funnel / treemap /
    sunburst / paretoLine / boxWhisker / regionMap), and
    `cxSubtotalIndices`.
  - **Chart-ref resolver**: dereferences Excel's `_xlchart.vN.X`
    indirection — chartEx bodies use opaque alias formulas
    (`<cx:f>_xlchart.v1.4</cx:f>`) that resolve through
    `workbook.xml`'s `<definedName hidden="1">Sheet1!$A$2:$A$7
    </definedName>` entries.
  - **TS renderer**: new `chartAdvanced.ts::drawChartEx` dispatches on
    `cxLayout`; waterfall painter draws cumulative bars (subtotal bars
    are absolute from the floor), dashed connectors between consecutive
    bars, per-bar value labels, and a synthetic 3-swatch legend
    (Increase / Decrease / Total) keyed to the workbook theme accents
    (accent1 / accent2 / accent3, matching the chartEx color-style
    part's default `cycle id="10"` palette). Other layouts still fall
    through to the placeholder plot pending fixtures.
  - Fixture: `tests/fixtures/charts/chart-waterfall-chartex.xlsx`
    (Excel-authored). hsx renders waterfall similarly; xlsx-preview was
    previously empty-bbox.
- Added a time-period conditional-formatting fixture plus a schema-drift CI
  guard, and documented the local schema regeneration / PNG fixture
  comparison workflow.
- Added `radarChart` support (ECMA-376 §21.2.2.155 / §21.2.2.176). New polar
  painter in `chartAdvanced.ts::drawRadarChart` honors `radarStyle`
  (`standard` / `marker` / `filled`) with polygon gridlines, per-spoke
  category labels, and value-axis tick labels along the top spoke. Per-series
  `<c:marker><c:symbol val="none"/>` still overrides marker visibility.
  Fixtures: `tests/fixtures/charts/chart-radar-{standard,marker,filled}.xlsx`.
- Added `stockChart` support (ECMA-376 §21.2.2.207). New painter in
  `chartAdvanced.ts::drawStockChart` infers the subtype from series count
  (3 → HLC, 4 → OHLC, 5 → VOHLC) and honors `<c:hiLowLines/>` (vertical mark
  from category low to high), `<c:upDownBars/>` (candlestick-style open→close
  rect; white-filled for up days, black-filled for down days), and
  `<c:dropLines/>`. Volume sub-plot stub for VOHLC carves off the bottom 22%
  of the plot rect. Legend swatches reflect what's actually painted: series
  with `markerSymbol === "none"` (hi-low envelope contributors) render a thin
  vertical bar in the hi-low ink color; series with markers render a colored
  dot. hsx (SpreadJS) currently renders stock charts as an empty plot, so
  xlsx-preview is the clear winner here. Fixtures:
  `tests/fixtures/charts/chart-stock-{hlc,ohlc}.xlsx`.

## [0.0.6] - 2026-05-16

### Fixed

- Avoid freezing/OOMing on sheets whose conditional-formatting ranges extend
  to full Excel row/column bounds (for example `XFD` / `1048576`) by scanning
  actual populated/numeric cells and clipping range expansion to the sheet's
  effective bounds.

## [0.0.5] - 2026-05-16

### Added

- Hidden / very-hidden sheets and tab colors (`Sheet.state`,
  `Sheet.tabColor`). `veryHidden` stays off the tab strip; `hidden`
  can be revealed with `PreviewerOptions.showHidden` / `?showHidden=1`.
  Fixture: `tests/fixtures/sheets/hidden-and-tabcolor.xlsx`.
- Node CLI `--all` skips hidden sheets; explicit `--sheet` targets still work.
- Expanded chart support for combo and dual-axis charts, including secondary
  value axes, per-series axis groups/chart types, secondary formats/scaling,
  axis titles, display units/labels, and secondary gridline metadata.
- Added bubble chart schema/rendering support with bubble sizes, bubble scale,
  and size-representation handling.
- Added per-data-point chart data-label overrides (`PointDataLabel`) with
  literal text, delete/suppress, position, number-format, and show-field
  inheritance/overrides.
- Added chart fixtures/builders for bubble charts, per-point data labels,
  no-fill stacked waterfall bars, stacked color modifiers, combo secondary
  axes, and dual-axis lines.
- Added chart utility tests for bar slot metrics, display-unit axis formatting,
  and zero-baseline helpers.
- Added `<c:majorUnit>` extraction on primary + secondary value axes
  (`Chart.majorUnit` / `majorUnitSecondary`). When authored, the renderer
  steps ticks by exactly the authored unit and walks the cadence down to
  zero for positive-only data with no `<c:min>` (capped at 14 ticks to
  avoid pathological expansion), so workbooks pinning a `<c:max>` +
  `<c:majorUnit>` get Excel's authored cadence (e.g. 0/9/18/27/36/45)
  instead of niceTicks (10/20/30/40/45). Wired through `bar/column`,
  `line`, `area`, and `combo` painters.
- Added unit tests for `resolveAxisRange` with `majorUnit` cadence,
  including the walk-to-zero heuristic, forced-min anchoring, tick-count
  cap on tiny steps, and dataMin straddling zero.
- Added Rust unit tests for `theme_scheme_color` covering all twelve
  ECMA-376 §20.1.6.2 scheme slots (accents, bg/tx, lt/dk, hlink) plus
  workbook-theme overrides on the lt1/bg1 slot, and for
  `built_in_unit_default_label` /​ `built_in_unit_factor` consistency.

### Changed

- Improved chart axis range resolution to honor explicit scaling bounds,
  avoid unintended zero-clamping, apply display-unit divisors to tick labels,
  and draw a heavier zero baseline when axes straddle zero.
- Improved bar/column geometry to follow OOXML `gapWidth` and `overlap` for
  clustered, stacked, and percent-stacked charts.
- Improved legends to reflect series style with filled swatches, line strokes,
  markers, or line+marker combinations.
- Improved line, scatter, combo, pie/doughnut, and bar rendering for marker
  suppression, blank-point gaps, sorted scatter line paths, per-point colors,
  no-fill point overrides, and per-point labels.

### Fixed

- Fixed scheme-color resolution on chart `<c:spPr>` and `<c:dPt>` blocks to
  handle every ECMA-376 §20.1.6.2 SchemeColor variant — not just
  `accent1..accent6`. `bg1`/`tx1`/`bg2`/`tx2`, `lt1`/`dk1`/`lt2`/`dk2`,
  `hlink`/`folHlink`, and the `windowText`/`window` system aliases now
  route through the workbook theme (with the ECMA-default `<a:clrMap>`
  fallback `bg1↔lt1`, `tx1↔dk1`, `bg2↔lt2`, `tx2↔dk2`). Fixes the
  "fake-waterfall" idiom where invisible stack segments are painted with
  `<a:schemeClr val="bg1"/>` (white-on-white) instead of `<a:noFill/>`;
  those segments used to inherit their parent series's accent color and
  break the floating-bar illusion. Refactored into a single shared
  `theme_scheme_color()` helper used by both fill and outline resolvers.
- Fixed chart title auto-generation from the series name. Per ECMA-376
  §21.2.2.211 + §21.2.2.4, when `<c:title>` is present without an explicit
  `<c:tx>` and `<c:autoTitleDeleted val="0"/>` (or the element is absent,
  which defaults to false) and the chart has exactly one series, Excel
  auto-uses the series name as the title; we used to render no title.
  `<c:autoTitleDeleted val="1"/>` continues to suppress.
- Fixed `<c:dispUnitsLbl>` default caption resolution. When the label
  element is present without an inner `<c:tx>` and the unit is a built-in
  (e.g. `<c:builtInUnit val="thousands"/>`), the extractor now falls back
  to the localized unit name ("Thousands", "Millions", … per
  `built_in_unit_default_label`) instead of dropping the caption. Excel
  paints this default even though the XML carries no text node.
- Fixed value-axis gridline rendering so gridlines only paint when authored
  and do not double-paint the zero line.
- Fixed chart data labels across bar, line, area, pie/doughnut, scatter, and
  combo renderers to respect per-point delete/text/format/position overrides.
- Fixed generated TypeScript schema exports for the new chart and data-label
  fields.
- Fixed bar, column, line, and area charts to clip series geometry to the
  plot rectangle when data exceeds a workbook-pinned `<c:scaling><c:max>`
  (or falls below `<c:min>`). Stacked column totals larger than the axis
  max, line strokes crossing an outlier, and area fills with a peak past
  the topmost gridline now match Excel and SpreadJS instead of painting
  past the plot frame. Added `chart-stacked-overflow-clip` and
  `chart-line-area-overflow-clip` regression fixtures.

## [0.0.4] - 2026-05-13

### Fixed

- Cross-origin worker URLs now work. When `workerUrl` resolves to a
  different origin (e.g. a jsDelivr or unpkg CDN), the loader wraps the
  script in a same-origin Blob shim before constructing the module worker,
  so the documented `jsDelivrUrls()` / `unpkgUrls()` flow renders instead
  of throwing `Failed to construct 'Worker': ... cannot be accessed from
  origin`.
- Workbooks from producers that use alternate threaded-comment namespace
  prefixes, including Google Sheets, now load without
  `unexpected tag while parsing PersonList` errors.
- Data-bar conditional formats that use Excel's `<x14:color>` fill-color
  element now load without `unexpected tag while parsing DataBar` errors.
- Charts anchored with `<xdr:oneCellAnchor>`, including Excel's
  "move but don't size with cells" drawings, are now rendered.
- Chart data resolution now ignores text cells, so shared-string indexes are
  no longer treated as numeric series values.
- Chart series backed by padded array-formula ranges are trimmed at the last numeric value instead of rendering an empty zero-value tail.
- Pie and doughnut legends now render one entry per category, using the same per-slice colors as the chart (`c:dPt` overrides, otherwise theme accents).
- Dense line and area chart category labels are thinned to avoid overlap.
- Numeric category-axis labels, including date serials, now use the chart cache or source cell number format.
- Text format `@` applied to a numeric cell (e.g. a formula result with `numFmtId=49`) now renders the value via general formatting instead of an empty string.
- Rotated text no longer clips to its cell rect, so huge fonts in narrow cells (e.g. a 220pt vertical "2026" in a 21px-wide merged column) render instead of vanishing. Stacked text (`textRotation=255`) is still clipped — its glyphs always fit by construction.
- Rotated text with `halign=center`/`left`/`right` now positions the rotated glyph bounding box rather than its baseline, fixing horizontal placement at 90° + large font sizes where ascender/descender asymmetry shifted the glyph noticeably off the column center.

### Changed

- Legends now honor the chart's `legendPos` value, including vertical
  left and right legends.

### Added

- `DrawingAnchor.extEmuCx` and `extEmuCy` expose explicit
  `oneCellAnchor` extents to the renderer.
- `Chart.categoriesFormat` exposes the number format used for category-axis
  labels.

## [0.0.3] - 2026-05-12

### Fixed

- `@hewliyang/xlsx-preview/browser` and the example HTML files now resolve
  against the actual emitted file. In 0.0.2 the loader was emitted as
  `dist/browserLoader.js` (matching the source name and the existing
  `.d.ts`), but `package.json` `exports["./browser"]` and the demo HTML
  pages still pointed at the legacy `dist/browser-loader.js` path.

## [0.0.2] - 2026-05-12

### Fixed

- Browser and React entry points now work in Vite and webpack 5 without
  manual asset configuration. The worker and wasm binary are shipped as
  discoverable ESM assets instead of being hidden inside a pre-bundled file.
- The browser worker initializes wasm from the resolved binary URL provided
  by the loader.
- Corrected the Node `renderXlsxToPng` README example. The function returns
  a `Buffer`; callers write it to disk themselves.

### Added

- `@hewliyang/xlsx-preview/cdn`, with `jsDelivrUrls(version)` for plain
  ESM pages and other non-bundled environments.

### Changed

- Renamed the browser loader option `wasmUrl` to `wasmBinaryUrl`; it now
  points directly at `xlcore_wasm_bg.wasm`. `workerUrl` is unchanged.
- Declared `engines.node >= 20`.

## [0.0.1] - 2026-05-12

- Initial release: canvas renderer + Node CLI + React/browser entry points.
- Rust extractor (`xlcore-export`) → `WorkbookLayout` JSON shared via `ts-rs`.
- Self-contained wasm extractor bundled into `dist/` for the browser entry.
- See [`docs/PARITY.md`](../../docs/PARITY.md) for the feature scoreboard.
