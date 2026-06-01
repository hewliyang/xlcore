# Changelog

All notable changes to `@hewliyang/xlsx-preview` are documented here.
This project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- Blank workbooks (no `xl/styles.xml`) now expose `defaultFont="Calibri"` / `defaultFontSize=11` in layout instead of empty/0 — previewer renders cell text on freshly-created workbooks without requiring a `setStyle` call.
- Playground `render()` honors `layout.activeSheetIndex` when a script calls `worksheet.activate()` (no longer pinned to the previously-active tab).

### Added

- Workbook API redesigned as a hierarchical SpreadJS-style facade. `workbook.sheet(name)` / `workbook.worksheets()` / `workbook.activeSheet()` / `workbook.addSheet(name)` / `workbook.removeSheet(name)` return `Worksheet` objects. `Worksheet` owns per-sheet sub-collections (`merges`, `hyperlinks`, `comments`, `threadedNotes`, `dataValidations`, `conditionalFormats`, `autoFilter`, `tables`, `charts`, `images`, `sparklineGroups`) and leaf APIs (`freeze` / `pageSetup` / `protection`), plus structural verbs (`setRowHeight`, `insertRows`, …) and lifecycle (`rename`, `moveTo`, `remove`, `activate`, `setVisibility`). `sheet.range(addr)` / `sheet.cell(addr)` return `Range` / `Cell` with `.setValue` / `.setValues` / `.setFormula` / `.setStyle` / `.clear` / `.copyTo` / `.fillTo` / `.merge`. Workbook-scoped leafs: `workbook.definedNames` / `allTables` / `allCharts` / `allImages` / `allSparklineGroups` / `properties` / `calcProperties` / `protection`. `workbook.worksheets()` is the only sheet enumerator — all uniform `list/set/remove` or `get/set/remove`. Addresses accept A1 strings or `{row, column, rowCount?, columnCount?}`; sheet names are auto-quoted. `Worksheet` and its children share an internal `SheetRef` so `worksheet.rename(newName)` keeps existing handles valid against the new name.

- Chart authoring: `ChartSeriesPatch` / `ChartSeriesInfo` gain `dataLabels` for per-series data labels (overrides chart-level when set). Chart-level dl no longer duplicates onto every series in the OOXML.
- Image authoring: `ImagePatch` / `ImageInfo` gain `rotationDegrees` (°, normalized mod 360, written as `rot` in 60000ths on `<a:xfrm>`), `cropLeftPct`/`cropTopPct`/`cropRightPct`/`cropBottomPct` (% on `<a:srcRect>` l/t/r/b, 1000ths-of-a-percent units), and `flipHorizontal`/`flipVertical` (`<a:xfrm flipH/flipV>`). Round-trips through save/reopen. New `invalid_image` cases for non-finite rotation/crop values.
- Sparkline group authoring: `sparklineGroups(sheet?)` / `setSparklineGroup(patch)` / `removeSparklineGroup(sheet, id)` with `SparklineKind` (`line` | `column` | `stacked`), `SparklineEntry { location, dataRef }`, axis kinds + manual min/max, line weight, full color palette (series/negative/axis/markers/first/last/high/low). New `invalid_sparkline_group` error.
- Chart authoring: `ChartKind` gains `scatter` | `bubble` | `doughnut`. Scatter/bubble series take `xValuesRef` (required) + `valuesRef` (yVal); bubble adds `bubbleSizesRef` (required). Per-series solid color via `ChartSeriesPatch.color` (`RRGGBB` hex, `#` accepted). Axis titles via `ChartPatch.categoryAxisTitle` / `valueAxisTitle`. New `invalid_chart` errors for missing xVal on scatter/bubble, missing bubble sizes, and non-hex color.

### Changed

- Chart authoring now builds typed `c::ChartSpace` / `xdr::TwoCellAnchor` structs instead of raw XML string templates; reader path also uses typed `PlotAreaChoice` traversal. No behavior change to the public API or output OOXML shape.

### Added

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
