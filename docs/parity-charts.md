# Chart parity

Chart parity corpus: small, public fixtures in `tests/fixtures/charts/`.

- Private customer cases are represented only as minimal repro workbooks with synthetic data.
- Renderers compared: `xlsx-preview` vs `hsx` (SpreadJS screenshot).
- Note: `hsx` is not always ground truth for charts; side with Excel desktop/spec where known.

## Summary

| Area | xlsx-preview | hsx | Winner |
| --- | --- | --- | --- |
| Combo charts / dual axes | ✅ | ✅ | tie |
| Stacked columns with theme modifiers | ✅ | ✅ | tie |
| Waterfall authored via stacked bars + per-point noFill | ✅ | ✅ | tie |
| Missing chart title | ✅ omits | ❌ paints `Chart Title` | xlsx-preview |
| Anchor clipping | ✅ honors `xdr:to` | ❌ overflows | xlsx-preview |
| `stockChart` (HLC / OHLC) | ✅ hi-low + candle up/down | ❌ empty plot | xlsx-preview |
| Office theme series colors | ✅ | ✅ | tie |
| Legend positioning | ✅ | 🟡 clipped on overflow | xlsx-preview |
| Legend marker shapes | ✅ | ✅ | tie |
| Bar widths / gapWidth / overlap | ✅ Excel/spec | 🟡 often wider | xlsx-preview |
| Combo data labels | ✅ | ✅ | tie |
| Secondary axis auto floor | ✅ data-driven | ❌ zero-clamps | xlsx-preview |
| Axis titles | ✅ | ✅ | tie |
| Major gridline toggle | ✅ | ✅ | tie |
| Line outline color | ✅ | ✅ | tie |
| Marker `symbol="none"` | ✅ | ✅ | tie |
| Negative-range axes | 🟡 rough edges | 🟡 rough edges | tie |
| `dispUnits` tick scaling + caption | ✅ Excel/spec | ❌ dropped | xlsx-preview |
| chartEx (`cx:` waterfall) | ✅ (new) | ❌ empty bbox | xlsx-preview |
| chartEx (`cx:` funnel/treemap/sunburst) | ✅ | ✅ | tie |
| chartEx (`cx:` histogram) | ✅ auto-binned columns | ❌ renders raw values as bars | xlsx-preview |
| chartEx (`cx:` pareto) | ✅ bars + cumulative line | ❌ renders as clustered duplicate bars | xlsx-preview |
| chartEx (`cx:` boxWhisker) | ✅ quartile boxes + whiskers + mean | ❌ renders as clustered column | xlsx-preview |
| chartEx (`cx:` regionMap) | ✅ choropleth (Natural Earth 110m) | ❌ column-chart fallback | xlsx-preview |
| `<c:multiLvlStrRef>` category axis | ✅ hierarchical multi-row | ✅ hierarchical multi-row | tie |

## Fixture corpus

| Path | Contents |
| --- | --- |
| `tests/fixtures/charts/chart-*.xlsx` | named minimal chart regression fixtures |
| `tests/fixtures/charts/chart-*.ours.png` | checked-in xlsx-preview renders |
| `tests/fixtures/charts/chart-*.hsx.png` | checked-in hsx renders |
| `tests/fixtures/charts/chart-*.layout.json` | extracted layout snapshots for debugging |
| `tests/fixtures/charts/build-chart-regressions.sh` | rebuilds the minimal chart regression fixtures |

chartEx (`cx:`) fixtures need a real Office-grade authoring path —
the XML body uses opaque `_xlchart.vN.X` definedName aliases
(resolved through `workbook.xml`'s hidden `<definedName>` entries) and
pulls colors from a chartStyle/colorStyle part pair. Two production
paths are in use:

- Excel desktop directly (waterfall fixture).
- SpreadJS via `hsx eval`, scripted in
  `tests/fixtures/charts/build-chartex.sh` — verified for `funnel`,
  `treemap`, `sunburst` (the three layoutIds where SpreadJS's chartEx
  serializer round-trips cleanly through Excel and through itself).
  `paretoLine` / `boxWhisker` / `clusteredColumn` (histogram) /
  `regionMap` all have known SpreadJS export gaps (missing `<cx:axis>`
  blocks, no auto-binning, degenerate render-as-cluster) — see the
  rationale block in `build-chartex.sh`. Histogram, pareto, and
  box/whisker have Excel-desktop-authored fixtures + renderer support
  (see Bug #24); regionMap still requires a Bing map lookup
  confirmation during Excel authoring.

Example fixtures:

| Sheet | Chart | Range | Notes |
| --- | --- | --- | --- |
| `Sheet1` | Waterfall via no-fill stack | `F2:M18` | `chart-waterfall-nofill-stacked.xlsx`: percent-stacked column using an invisible spacer series |
| `Sheet1` | Stacked color modifiers | `F2:M18` | `chart-stacked-color-modifiers.xlsx`: two stacked column series with distinct fills |
| `Sheet1` | Combo with secondary axis | `F2:N19` | `chart-combo-secondary-axis.xlsx`: clustered column + line on secondary y-axis |
| `Sheet1` | Dual-axis lines | `F2:N19` | `chart-dual-axis-lines.xlsx`: two line series on primary/secondary y-axes |
| `Sheet1` | Radar (standard/marker/filled) | `F2:N20` | `chart-radar-{standard,marker,filled}.xlsx`: one fixture per `radarStyle` value |
| `Sheet1` | chartEx funnel | `A1:N22` | `chart-funnel-chartex.xlsx`: `cx:` `layoutId="funnel"`; single descending series, `numDim type="val"` |
| `Sheet1` | chartEx treemap | `A1:N22` | `chart-treemap-chartex.xlsx`: `cx:` `layoutId="treemap"`; region→country hierarchy, `numDim type="size"` |
| `Sheet1` | chartEx sunburst | `A1:N22` | `chart-sunburst-chartex.xlsx`: `cx:` `layoutId="sunburst"`; quarter→month hierarchy, `numDim type="size"` |
| `Sheet1` | chartEx histogram | `A1:N22` | `chart-histogram-chartex.xlsx`: `cx:` `layoutId="clusteredColumn"` with `<cx:binning>`; Excel-authored histogram fixture |
| `Sheet1` | chartEx pareto | `A1:N22` | `chart-pareto-chartex.xlsx`: `cx:` primary `layoutId="clusteredColumn"` plus owner `layoutId="paretoLine"`; Excel-authored pareto fixture |
| `Sheet1` | chartEx box and whisker | `A1:N22` | `chart-boxwhisker-chartex.xlsx`: `cx:` `layoutId="boxWhisker"`; Excel-authored box/whisker fixture |
| `2-color Map Chart` | chartEx region map | `A1:Y20` | `chart-regionmap-chartex.xlsx`: `cx:` `layoutId="regionMap"`; Microsoft "Map Chart samples.xlsx" template (slimmed — two ~19MB `<cx:binary>` Bing geoCache blobs stripped, our renderer doesn't consume them). Fixture cleared for use; renderer painter still TODO. |

## Bug catalog

| # | Issue | Side | Status | Notes / key fixtures |
| --- | --- | --- | --- | --- |
| 1 | Combo / dual-axis second series dropped | xlsx-preview | ✅ fixed | Extract all plot-area chart groups; render combo path. `chart-combo-secondary-axis`, `chart-dual-axis-lines`. |
| 2 | Two-series stacked column had indistinguishable colors | xlsx-preview | ✅ fixed | Apply `schemeClr` modifiers (`lumMod`, `lumOff`, `shade`, `tint`). `chart-stacked-color-modifiers`. |
| 3 | Waterfall-style stacked bar ignored per-point `noFill` | xlsx-preview | ✅ fixed | Extract point colors with `"none"` sentinel; skip paint but keep stack accumulator. `chart-waterfall-nofill-stacked`. |
| 4 | Literal `Chart Title` placeholder leaks | hsx | open | xlsx-preview correctly omits absent titles. |
| 5 | Chart bbox overflows anchor | hsx | open | xlsx-preview honors `xdr:to`. |
| 6 | Series color modifiers dropped | xlsx-preview | ✅ fixed | Same root as #2; also handles `srgbClr` modifiers. |
| 7 | Bar widths ignored `gapWidth` / `overlap` | xlsx-preview | ✅ fixed | `computeBarSlotMetrics`; Excel/spec defaults. |
| 8 | Combo-path data labels missing | xlsx-preview | ✅ fixed | Deferred label paint for combo bars/lines; secondary-axis format used. |
| 9 | Secondary axis auto floor zero-clamped | hsx | open | xlsx-preview follows data-driven Excel/spec behavior. `chart-combo-secondary-axis`. |
| 10 | Axis titles dropped | xlsx-preview | ✅ fixed | Extract/render x-axis, primary y-axis, secondary y-axis titles. |
| 11 | Secondary y-axis inherited primary bar zero-clamp | xlsx-preview | ✅ fixed | Per-axis `hasBaselineSeries`. |
| 12 | Horizontal gridlines drawn unconditionally | xlsx-preview | ✅ fixed | Honor `<c:majorGridlines>` presence and `<a:noFill/>`. |
| 13 | Negative-range y-axes rough | both | partial | Heavier zero baseline + shared `zeroAxisMetrics` fixed; cat-axis band drift remains. |
| 14 | Legend swatch always square | xlsx-preview | ✅ fixed | Line/scatter legend symbols now stroke/marker-aware. |
| 15 | Missing line points crashed to y(0) | xlsx-preview | ✅ fixed | Missing points break line path; area gaps still deferred. |
| 16 | `<c:dispUnits>` ignored | xlsx-preview | ✅ fixed | Tick labels scale by divisor; caption paints. |
| 17 | Phantom legend without `<c:legend>` | xlsx-preview | ✅ fixed | Preserve absent legend vs present-with-default-position. |
| 18 | Line outline color and marker suppression ignored | xlsx-preview | ✅ fixed | Use outline fill for line/scatter; honor `markerSymbol === "none"`. |
| 19 | 3D legacy chart variants emitted empty bbox | xlsx-preview | ✅ fixed | `Bar3D` / `Line3D` / `Area3D` / `Pie3D` / `ofPie` plot-area arms dispatch to the 2D painter; depth/perspective dropped. `chart-3d-*`. |
| 20 | `radarChart` emitted empty bbox | xlsx-preview | ✅ fixed | New `drawRadarChart` (polar painter); polygon gridlines, per-spoke category labels, top-spoke value-axis ticks. `radarStyle` selects standard / marker / filled. `chart-radar-{standard,marker,filled}.xlsx`. |
| 21 | `stockChart` emitted empty bbox | xlsx-preview | ✅ fixed | New `drawStockChart`; series-count infers subtype (3=HLC, 4=OHLC, 5=VOHLC). Honors `<c:hiLowLines/>` (vertical mark), `<c:upDownBars/>` (open→close rect; white-fill up, black-fill down), `<c:dropLines/>`. Volume sub-plot stub for VOHLC. hsx renders empty here. `chart-stock-{hlc,ohlc}.xlsx`. |
| 27 | `percentStacked` bar/column rendered as raw stacked (0–150 axis) | xlsx-preview | ✅ fixed | `drawBarColumnChart` collapsed `stacked` + `percentStacked` into one branch with no normalization. Now derives a separate `percent` flag, overrides axis to 0–100 with `25%`-step ticks, scales each segment by `100 / catTotal` while still passing the raw value to `buildLabelText`. Negative-sign normalization is positive-side only (mixed-sign categories deferred). Validated end-to-end against authored column/bar/line/area stacking via a throwaway `xlcore-api` example writing to `/tmp/stacking-e2e.xlsx` and reopening in Excel desktop. |
| 26 | `<c:multiLvlStrRef>` category axis crashed preview / lost categories | xlsx-preview | ✅ fixed | `CategoryAxisDataChoice` match dropped `CMultiLvlStrRef` through a wildcard, losing the formula ref so the resolver in `refs.rs` couldn't backfill from the sheet range either; combined with Rust eliding empty `categories` while ts-rs declared it required, the renderer threw `Cannot read properties of undefined (reading '0')` in `drawCategoryAxis`. **Phase 1**: extractor surfaces the formula ref, resolver backfills cats from the sheet range, schema always emits the field, renderer guards `chart.categories[i]`. **Phase 2 (multi-row label bands)**: `refs.rs::resolve_chart_refs` now reuses `cx_category_levels` for non-chartex charts when the formula range is `n_rows > 1 && n_cols > 1` — one inner Vec per source row, in source order (`levels[0]` = top row = outermost, `levels[last]` = bottom row = innermost, mirrors `chart.categories`). Single-column multi-row ranges keep the flat fallback (they're just vertically-laid-out categories, not hierarchical). Renderer: new `categoryAxisExtraRows` / `categoryAxisExtraHeight` helpers in `chartUtils.ts`; `drawAxisFrame` + `drawCategoryAxis` enlarge `xAxisH` and stack extra rows beneath the innermost band; new `drawCategoryAxisExtraRowsCentered` wired into the bar/column inline path (`chart.ts`), `chartCombo.ts`, and `chartStock.ts` (horizontal bar charts skip extras since categories live on the y-axis). Fixture: `multilvlstr-cat.xlsx` now matches hsx. Repro that originally triggered Phase 1: `12-month-budget-template.xlsx` (single-row `Budget!$B$2:$M$2`, still matches via flat fallback). |
| 25 | chartEx regionMap emitted placeholder | xlsx-preview | ✅ fixed | `chartExRegionMap.ts::drawRegionMapChartEx` using an embedded Natural Earth 110m countries dataset (`world110m.ts`); equirectangular projection, country-name index with alias fallbacks, two-stop near-white→accent1 color scale keyed on `<cx:numDim type="colorVal">`, gradient legend bar. Extractor: `parse_series_data` accepts `ColorVal` numDim type; `extract_chart_ex` picks the first non-`hidden="1"` series for `regionMap` (Excel ships 4 alternate-preset series, only the last visible). hsx falls back to a clustered column for this layout. Fixture: `chart-regionmap-chartex.xlsx`. |
| 24 | chartEx histogram / pareto / boxWhisker emitted placeholder | xlsx-preview | ✅ fixed | Three new painters in `chartExStats.ts` (split out of `chartEx.ts` to stay under the per-file LoC budget). **Extractor changes** — (1) `xmlns_normalize` rewrites `<cx:axisId val="N"/>` (Excel's attribute form for pareto secondary-axis assignment) into the `<cx:axisId>N</cx:axisId>` text-child form ooxmlsdk's chartEx schema expects, otherwise the entire chartEx parse fails with `invalid field 'cx_axis_id' while parsing Series: ""`; (2) `extract_chart_ex` now walks all `<cx:series>` (not just the first) and detects three multi-series / layoutPr-flagged compositions: `paretoLine` companion → `cx_layout="pareto"`, all-`boxWhisker` → `cx_layout="boxWhisker"`, single `clusteredColumn` with `<cx:binning>` → `cx_layout="histogram"`. **Renderer**: histogram auto-bins via Sturges + nice-width rounding with right-closed `(low, high]` bin labels; pareto paints primary `clusteredColumn` bars (left axis) plus a cumulative-% line on a synthesized right axis (the source paretoLine series has no own data); boxWhisker computes Q1/median/Q3/whiskers/outliers per QUARTILE.EXC and paints the box + median rule + whisker caps + mean (×) marker per series. Fixtures: `chart-{histogram,pareto,boxwhisker}-chartex.xlsx`. |
| 23 | chartEx funnel / treemap / sunburst emitted placeholder | xlsx-preview | ✅ fixed | New `chartEx.ts` module. Funnel: center-aligned horizontal bars scaled to max. Treemap: squarified layout (Bruls 2000); parents from `cxCategoryLevels[0]` get the accent, leaves share parent color. Sunburst: ring-per-level polar layout, DFS traversal keeps siblings angularly contiguous, per-branch accent with innermost-ring darken. Three extractor pieces: (1) accept `<cx:numDim type="size">` (treemap/sunburst use size not val); (2) materialize multi-column `categories_ref` ranges as `cxCategoryLevels`; (3) suppress the trivial single-series legend for these three layouts. Fixtures: `chart-{funnel,treemap,sunburst}-chartex.xlsx` (SpreadJS-authored). |
| 22 | chartEx (`cx:`) drawings emitted empty bbox | xlsx-preview | ✅ fixed for waterfall | Four-part fix: (1) `xmlns_normalize` textually unfolds `<mc:AlternateContent>` blocks in drawing parts to their first `<mc:Choice>` content — Excel always wraps chartEx in MC for old-Excel fallback, and ooxmlsdk's typed `two_cell_anchor_choice` never sees MC contents otherwise. (2) New `cx:` extractor in `charts.rs::extract_chart_ex` surfaces `chart_type="chartex"`, `cx_layout`, and `cx_subtotal_indices`. (3) Chart-ref resolver dereferences Excel's `_xlchart.vN.X` indirection — chartEx bodies use opaque alias formulas (`<cx:f>_xlchart.v1.4</cx:f>`) that resolve through `workbook.xml`'s `<definedName hidden="1">Sheet1!$A$2:$A$7</definedName>` entries. (4) New `chartAdvanced.ts::drawChartEx` dispatches on `cxLayout`; the `waterfall` painter draws cumulative bars (subtotals absolute from the floor), dashed connectors, per-bar value labels, theme-accent fills (accent1=Increase / accent2=Decrease / accent3=Total per the colorStyle part's default `cycle id="10"`), and a synthetic 3-swatch legend. Other layouts (funnel/treemap/sunburst/paretoLine/boxWhisker/regionMap) still fall through to the placeholder pending fixtures. `chart-waterfall-chartex.xlsx` (Excel-authored). |

## Chart-type coverage

| Namespace | Type | Status | Notes |
| --- | --- | --- | --- |
| `c:` | `barChart` | ✅ | column/horizontal; clustered/stacked/percentStacked; `gapWidth` + `overlap` |
| `c:` | `lineChart` | ✅ | standard/stacked/percentStacked; marker suppression |
| `c:` | `pieChart` | ✅ | per-slice colors |
| `c:` | `doughnutChart` | ✅ | pie path + center hole |
| `c:` | `areaChart` | ✅ | standard/stacked/percentStacked |
| `c:` | `scatterChart` | ✅ | marker/line/lineMarker/smooth/smoothMarker |
| `c:` | `bubbleChart` | ✅ | `bubbleScale`, `sizeRepresents` |
| `c:` | `radarChart` | ✅ | `radarStyle` standard / marker / filled; polygon gridlines + spokes + tick labels along top spoke |
| `c:` | `stockChart` | ✅ | HLC / OHLC; hi-low marks, up/down bars (white-up / black-down). xlsxwriter `<c:marker val="none"/>` on non-close series honored. `chart-stock-{hlc,ohlc}.xlsx`. |
| `c:` | `surfaceChart` / `surface3DChart` | ❌ | not wired |
| `c:` | `ofPieChart` | 🟡 | rendered as plain pie (no satellite split) |
| `c:` | `bar3DChart` / `line3DChart` / `area3DChart` / `pie3DChart` | ✅ | dispatched to 2D painters; 3D perspective/depth dropped |
| `cx:` | `waterfall` | ❌ | chartEx unsupported |
| `cx:` | `funnel` | ✅ | `chartEx.ts::drawFunnelChartEx`. Center-aligned horizontal bars; widths scaled to max value; per-bar value labels when they fit. Fixture: `chart-funnel-chartex.xlsx` |
| `cx:` | `treemap` | ✅ | `chartEx.ts::drawTreemapChartEx`. Squarified layout (Bruls et al. 2000); multi-level hierarchies grouped by `cxCategoryLevels[0]` with per-branch theme accent. Fixture: `chart-treemap-chartex.xlsx` |
| `cx:` | `sunburst` | ✅ | `chartEx.ts::drawSunburstChartEx`. Ring-per-level polar layout from `cxCategoryLevels`; per-branch accent (innermost ring darkened); tangentially-rotated slice labels. Fixture: `chart-sunburst-chartex.xlsx` |
| `cx:` | `histogram` | ✅ | `chartExStats.ts::drawHistogramChartEx`. Sturges bin count → nice-rounded width; right-closed `(low, high]` bin labels with the leftmost bin shown as `[low, high]`. Fixture: `chart-histogram-chartex.xlsx` |
| `cx:` | `pareto` | ✅ | `chartExStats.ts::drawParetoChartEx`. Primary clusteredColumn bars + cumulative-% line on a synthesized right-hand axis (0”100%). Fixture: `chart-pareto-chartex.xlsx` |
| `cx:` | `boxWhisker` | ✅ | `chartExStats.ts::drawBoxWhiskerChartEx`. Computes Q1/median/Q3/whiskers/outliers per QUARTILE.EXC; paints box + median rule + whisker caps + mean (×) marker per series. Fixture: `chart-boxwhisker-chartex.xlsx` |
| `cx:` | `regionMap` | ✅ | `chartExRegionMap.ts::drawRegionMapChartEx`. Equirectangular projection over an embedded Natural Earth 110m countries dataset (`world110m.ts`, ~170KB; coords rounded to 2dp); country-name index covers NAME / NAME_LONG / ISO_A2 / ISO_A3 plus a small alias table (`USA`, `UK`, `UAE`, `DRC`, `Côte d'Ivoire`, ...). Palette honors authored `<cx:valueColors>`: 3-stop diverging (min/mid/max, e.g. blue→red→green on the "3-color Map Chart" fixture) or 2-stop linear; falls back to a near-white → accent1 sequential ramp when the workbook doesn't author one. Unmatched countries paint UNMATCHED_FILL. Gradient legend bar on the right with min/max labels. Fixture: `chart-regionmap-chartex.xlsx` (carries both 2-color and 3-color sheets). |

## Unsupported chartEx

| Item | Detail |
| --- | --- |
| Minimal public coverage | chartEx fixtures still needed |
| Parts | `xl/charts/chartEx*.xml` (`cx:chartSpace`, Microsoft `c16`) |
| Relationship type | `http://schemas.microsoft.com/office/2014/relationships/chartEx` |
| xlsx-preview behavior | empty chart bbox |
| hsx behavior | renders natively |
| Impact | largest remaining chart parity gap |

## Priority order

1. ~~3D legacy chart variants: dispatch to 2D painters.~~ **shipped.** `chart-3d-{bar3d,line3d,area3d,pie3d}.xlsx`.
2. ~~`radarChart`.~~ **shipped.** Polygon spider painter in `chartAdvanced.ts::drawRadarChart`; honors `radarStyle` (`standard` / `marker` / `filled`). `chart-radar-{standard,marker,filled}.xlsx`.
3. ~~`ofPieChart` as plain pie first.~~ **shipped** (satellite split still deferred). `chart-3d-ofpie.xlsx`.
4. ~~`stockChart`.~~ **shipped.** `chartAdvanced.ts::drawStockChart`; HLC (3-series, hi-low marks) and OHLC (4-series, candlestick up/down bars). Volume sub-plot stub for 5-series VOHLC. `chart-stock-{hlc,ohlc}.xlsx`.
5. ~~`cx:` waterfall.~~ **shipped.** `chart-waterfall-chartex.xlsx`. Pipeline: `xmlns_normalize` unfolds `<mc:AlternateContent>` → chartEx schema parses → `drawChartEx` waterfall painter.
6. ~~`cx:` funnel / treemap / sunburst.~~ **shipped.** Painters in
   `packages/xlsx-preview/src/chartEx.ts`. Extractor changes: accept
   `<cx:numDim type="size">` alongside `type="val"`; surface multi-
   column `categories_ref` ranges as `cxCategoryLevels: Vec<Vec<String>>`
   in `refs.rs::resolve_chart_refs` (one inner Vec per nesting level,
   parallel to the values array). Renderer changes: per-layout dispatch
   from `drawChartEx`; squarified treemap; DFS sunburst with per-branch
   accent. `chart.ts` suppresses the trivial single-series legend
   ("Count" / "GDP" / "Sales") for these three layouts. `chartAdvanced.ts`
   split: `chartEx.ts` (~700 LOC) + `chartStock.ts` (~300 LOC) carved
   out to fit the per-file LOC budget. Fixtures:
   `chart-{funnel,treemap,sunburst}-chartex.xlsx`.
7. ~~`cx:` histogram / boxWhisker / paretoLine.~~ **shipped.**
   `chartExStats.ts` painters (histogram = Sturges-binned columns,
   pareto = bars + cumulative-% line on a synthesized right axis,
   boxWhisker = QUARTILE.EXC boxes + whiskers + mean marker).
   Extractor pipeline: `xmlns_normalize` rewrites `<cx:axisId val="N"/>`
   to the text-child form ooxmlsdk's chartEx schema requires (the
   attribute form Excel actually emits would otherwise crash the
   parse); `extract_chart_ex` walks all `<cx:series>` and detects
   pareto / boxWhisker / histogram via layoutId combination +
   `<cx:binning>` layoutPr. Fixtures:
   `chart-{histogram,pareto,boxwhisker}-chartex.xlsx`.
8. ~~`cx:` regionMap.~~ **shipped.** `chartExRegionMap.ts::drawRegionMapChartEx`.
   Bring-your-own world geometry: Natural Earth 110m admin_0 countries,
   slimmed + 2dp-rounded into `packages/xlsx-preview/src/world110m.ts`
   (~170KB; regeneration snippet in the painter file header). The
   Bing-encoded `<cx:binary>` geoCache blobs are deliberately ignored.
   Extractor changes: (1) `parse_series_data` accepts
   `<cx:numDim type="colorVal">` alongside `val` / `size`; (2)
   `extract_chart_ex` picks the first non-`hidden="1"` series for
   `regionMap` layouts (Excel ships up to 4 alternate-preset series);
   (3) new `extract_region_map_colors` parses `<cx:valueColors>` 2- or
   3-stop palettes, resolving `<a:srgbClr>` literals + `<a:schemeClr>`
   theme refs (with modifier-chain support reused from
   `apply_color_modifiers`) into `cx_region_map_{min,mid,max}_color`.
   Renderer: equirectangular projection with 1:1 lon/lat aspect, lat
   clamp to [-58, 84] so the world fills the rect; country-name lookup
   over NAME / NAME_LONG / ISO_A2 / ISO_A3 plus a small alias table
   (USA, UK, UAE, DRC, Czechia, Burma → Myanmar, Côte d'Ivoire, ...);
   palette honors authored 3-stop diverging (e.g. blue→red→green) or
   2-stop linear from the schema, falling back to near-white → accent1
   when no `<cx:valueColors>` was authored; gradient legend bar with
   min/max labels on the right; unmatched countries paint a neutral
   gray base layer. hsx falls back to a clustered column chart for
   this layout, so xlsx-preview now wins it outright. Fixture:
   `chart-regionmap-chartex.xlsx` (covers both 2-color sequential and
   3-color diverging palettes via its two sheets).
9. `surfaceChart`.

## Open items

- Rotated-along-axis `<c:dispUnitsLbl>` placement.
- Negative-range category labels at value-axis crossing (`tickLblPos="nextTo"`).
- Stacked bars crossing zero: polygon-aware overlap handling.
- Area chart gaps for missing points.
- Live recalc for formula-only chart source cells; blocked on `xlcore-engine`.

## Reproduction

```bash
# Render one minimal chart regression fixture with both tools
F=tests/fixtures/charts/chart-combo-secondary-axis.xlsx
pnpm --filter @hewliyang/xlsx-preview build:ts
node packages/xlsx-preview/dist/cli.js "$PWD/$F" -o "$PWD/${F%.xlsx}.ours.png" --sheet Sheet1 --range F2:N19 --scale 2
hsx --timeout 60 screenshot "$F" 'Sheet1!F2:N19' -o "${F%.xlsx}.hsx.png"
```
