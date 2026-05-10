# PARITY.md

Where xlcore stands against Excel / SpreadJS-via-`hsx`. Updated whenever a
feature lands or a bug rewrites the table.

Three layers, lowest first:

1. **Data parity** — does the extractor surface the OOXML feature in
   `WorkbookLayout` JSON? (Or is the bit silently dropped?)
2. **Schema parity** — does the JSON the renderer reads match what the
   extractor writes? (Field names, types, optionality.) Auto-checked by
   `ts-rs`-generated TS — see [Schema sync](#schema-sync).
3. **Visual parity** — does our canvas render look like SpreadJS for the
   same workbook? (Pixel diff against `hsx screenshot`.)

`hsx` (SpreadJS) is the ground truth for layers 1 and 3. When `hsx`
disagrees with desktop Excel we record the divergence in the relevant
fixture's README and pick a side.

## Status table

Legend: ✅ done · 🟡 partial · ❌ missing · n/a not in scope for v0.

### Cell content & formatting

| Feature                         | Extract | Render | Notes                                                                                           |
| ------------------------------- | ------- | ------ | ----------------------------------------------------------------------------------------------- |
| Shared strings                  | ✅      | ✅     |                                                                                                 |
| Inline strings                  | ✅      | ✅     |                                                                                                 |
| Rich-text runs (`<r>`)          | ✅      | ✅     | bold/italic/color/font/size per run; both SST and inline.                                       |
| `\n` hard line breaks           | ✅      | ✅     |                                                                                                 |
| `wrapText` soft wrap            | ✅      | ✅     | word-wrap respecting per-run font metrics.                                                      |
| Text overflow into empties      | n/a     | ✅     | left/right/center, supports merged cells.                                                       |
| Indent (`textIndent`)           | ✅      | ✅     | Each unit ≈ 3 character widths (`round(defaultFontSize * 0.75)` px); applied on the alignment-anchor side (left for `left`/general-resolved-to-left, right for `right`; center is unaffected). Reduces `innerW` and biases the overflow-into-empties threshold so indented text behaves correctly under wrap and overflow too. Fixture: `tests/fixtures/text/indent.xlsx`. |
| Text rotation                   | ✅      | ✅     | OOXML `textRotation` 1–180 (CCW + CW) and 255 (stacked) all painted. CCW anchors at cell bottom-left + extends up-right; CW anchors top-left + extends down-right; stacked draws each char upright on its own line, horizontally centered. `halign=center`/`right` shifts the baseline anchor along the cell width. No overflow into neighbors and no wrap support for rotated text (matches Excel — author-time row height already accounts for the rotated extent). Fixture: `tests/fixtures/text/rotation.xlsx`. Open: vertical placement on slanted angles is bottom-anchored where hsx keeps the rotated bounding-box vertically centered (cosmetic). |
| Strikethrough                   | ✅      | ✅     | painted as a 1px stroke through the visual middle (baseline − 30% font-size).                                                           |
| Underline (single)              | ✅      | ✅     | painted as a 1px stroke at baseline+12% font-size; honors per-run, dxf-overlay, and hyperlink underlines.                                                                                                 |
| Underline (double / accounting) | ✅      | 🟡     | All 4 OOXML `ST_UnderlineValues` (`single` / `double` / `singleAccounting` / `doubleAccounting`) extracted into a new `underlineStyle` field on `Font` / `TextRun` / `Dxf` (absent = `single`, the OOXML default). Renderer paints `double` / `doubleAccounting` as two parallel strokes (gap = `max(2, fontSizePx * 0.1)`); accounting variants currently render exactly like their non-accounting siblings — Excel's "line extends across the full cell width" semantics aren't honored yet (`paintTextDecorations` only knows the text segment's measured width, not the cell rect). Fixture: `tests/fixtures/text/underline.xlsx`. Hsx divergence: SpreadJS paints all four variants as identical single thin underlines; we match Excel desktop on the double-line variants. |
| `<i val="0"/>` boolean unset    | ✅      | ✅     | (bug fix: was treated as `true`).                                                               |
| Theme XML colors                | ✅      | ✅     | parsed from `xl/theme/theme1.xml`. Spreadsheet `theme="N"` indexing (lt1/dk1/lt2/dk2 swap) is correct. Cell + chart-series accents resolve against the workbook palette. All five OOXML color-choice variants resolved: `srgbClr`, `sysClr.lastClr`, `scrgbClr` (RGB percentages → 0..255 bytes), `hslClr` (HSL → sRGB), and `prstClr` (190-entry preset table covering CSS3/X11 names + `dk*`/`lt*`/`med*` abbreviations + 2010 aliases). Office defaults remain only as a last-resort fallback when the theme part is missing entirely. Unit tests in `crates/xlcore-export/src/theme.rs`. |
| Indexed-color palette           | ✅      | ✅     | Full ECMA-376 §18.8.27 default `indexedColors` table baked into `INDEXED_PALETTE` (`render.ts`) and the parallel 1-based `COLOR_BY_INDEX` (`numfmt.ts`, used by `[ColorN]` format codes). Covers all 56 legacy slots + 64/65 specials. Open: workbook-level palette override via `<colors><indexedColors>` in styles.xml (vanishingly rare — Excel only writes that block when the user customizes the palette through Office 2003-era dialogs). |
| Tint                            | ✅      | ✅     | proper OOXML HLS-space tint (`L' = L*(1+t)` for `t<0`, `L' = L*(1-t)+t` for `t>0`); preserves hue + saturation. Verified against Excel "Accent1, Lighter/Darker N%" reference values. Unit tests in `render-ts/src/render.test.ts`. |
| Number formats: built-ins       | ✅      | ✅     | All ECMA-376 §18.8.30 built-ins (IDs 0–49) wired through the new `numfmt.ts` evaluator. Verified against `tests/fixtures/numfmt/date-time-formats.xlsx`. |
| Number formats: custom code     | ✅      | ✅     | Multi-section `pos;neg;zero;text` + `[Red][>0]` conditional gates + `[$€-407]` currency tags + trailing-comma scaling all handled. Color from `[Red]` propagates to the cell's text. Triage: `tests/fixtures/numfmt/custom-section-conditions.xlsx`. |
| Number formats: fractions       | ✅      | ✅     | Variable-denom (`# ?/?`, `# ??/??`) via Stern–Brocot, fixed-denom (`?/8`, `?/16`). Fixture: `tests/fixtures/numfmt/fraction-and-scientific.xlsx`. |
| Number formats: scientific      | ✅      | ✅     | `0.00E+00` classic + `##0.0E+0` engineering shift (mantissa fits exactly N integer digits). Fixture: `tests/fixtures/numfmt/fraction-and-scientific.xlsx`. |
| `applyFont` / `applyFill` etc.  | ✅      | ✅     | `<cellStyleXfs>` (named-style parents) parsed alongside `<cellXfs>`; cell xfs with `apply*="0"` inherit `fontId` / `fillId` / `borderId` / `numFmtId` / alignment from `cellStyleXfs[xfId]`. ECMA-376 §18.8.45 semantics: missing or `"1"` → use the xf's own value, `"0"` → walk back to the parent. Fixture: `tests/fixtures/styles/named-inheritance.xlsx`. Hsx divergence: SpreadJS ignores `apply*="0"` entirely and renders unflattened cells as plain Calibri 11; we match Excel desktop. |

### Borders & fills

| Feature                      | Extract | Render | Notes                                                                |
| ---------------------------- | ------- | ------ | -------------------------------------------------------------------- |
| Per-side border + style      | ✅      | ✅     | All 14 ECMA-376 §18.18.3 `ST_BorderStyle` values painted: `thin` / `medium` / `thick` / `hair` / `double` / `dotted` / `dashed` / `mediumDashed` / `dashDot` / `mediumDashDot` / `dashDotDot` / `mediumDashDotDot` / `slantDashDot`. Each gets its own width × dash-pattern combo in `cellPaint.ts` (`borderWidth` + new `borderDash`). Fixture: `tests/fixtures/borders/every-style.xlsx`. Divergence: `hsx` paints all `*DashDot*` variants as solid lines; we paint the proper Excel-desktop dash cadences (kept the more-correct side per PARITY's hsx-vs-Excel rule). Bug fix: extractor's `border_style_str` ordered substring match used to mis-identify `slantDashDot` as `dashDot` because the longer name's lowercase form contains `dashdot` and was tested after; now `slantdashdot` is checked first. |
| Diagonal borders             | ✅      | ✅     | OOXML `<border diagonalUp="1" diagonalDown="1"><diagonal style="..." color=...>` parsed; both diagonals share one style+color (matches the OOXML model). Renderer's `drawDiagonalBorders` clips strictly to the cell rect so wide strokes don't bleed into neighbors; for merged regions the diagonal spans the full merge. Fixture: `tests/fixtures/borders/diagonal.xlsx`. |
| Borders around merged ranges | ✅      | ✅     | (bug fix: perimeter cells of merge now paint their border segments). |
| Pattern fills (solid)        | ✅      | ✅     |                                                                      |
| Pattern fills (gray, hatch)  | ✅      | ✅     | All 18 OOXML `PatternValues` types extracted via a real `match` (was a Debug-string scan). Renderer paints each via an 8x8 binary tile (`PATTERN_TILES_8X8` in `render.ts`) drawn into an offscreen canvas + fed to `ctx.createPattern(_, "repeat")`; bg paints first, fg paints the marks on top. Pattern cache keyed by `(type|fg|bg)`. Fixture: `tests/fixtures/fills/patterns.xlsx` (built via Python zip-patch — SpreadJS doesn't surface hatches on its public style API). |
| Gradient fills (linear)      | ✅      | ✅     | Multi-stop linear with arbitrary `degree` (0° = L→R, 90° = T→B, 180° = R→L, 270° = B→T) and intermediate angles via the rotated-axis projection of the cell rect onto `(cosθ, sinθ)`. Stop positions and colors round-trip through the new `GradientStop { position, color }` schema (was `Vec<Color>` discarding positions). Fixture: `tests/fixtures/fills/gradients.xlsx`. |
| Gradient fills (radial/path) | ✅      | ✅     | OOXML `<gradientFill type="path">` with `left`/`right`/`top`/`bottom` inner-convergence rect (each a fraction of cell size). Renderer fills the cell with the innermost stop, then overlays a `createRadialGradient` from the inner rect's bounding circle out to the farthest cell corner. Schema: `gradientType` + `gradientLeft|Right|Top|Bottom`. Fixture: `tests/fixtures/fills/gradients.xlsx`. Hsx divergence: SpreadJS paints path gradients as a much smaller / washed-out radial blob; we match Excel desktop's stronger inner-rect-out-to-corners behavior. |

### Layout

| Feature                   | Extract | Render | Notes                                     |
| ------------------------- | ------- | ------ | ----------------------------------------- |
| Custom column widths      | ✅      | ✅     |                                           |
| Custom row heights        | ✅      | ✅     |                                           |
| Hidden rows / cols        | ✅      | ✅     | (zero-sized in grid).                     |
| Outline / group levels    | ✅      | 🟡     | OOXML `<row outlineLevel="N">` and `<col outlineLevel="N">` extracted (capped at the spec limit of 7). Wire: `Col.outlineLevel: u8` + a new `RowMetaBlob.outlineLevel` u8 blob, omitted entirely from JSON when every row is at level 0. `<sheetPr><outlinePr summaryBelow summaryRight/>` lands on `Sheet.outlinePr` (defaults true/true match Excel). Renderer paints level brackets inside the existing row + column header strips: vertical `[` on the left edge of HEADER_W per row run, horizontal `⌐...¬` on the top edge of HEADER_H per col run, one slot per level (7px step), tick caps at the run endpoints, scoped to pinned + scrolling pane segments. No expand/collapse buttons, no summary-row glyphs, no separate gutter strip (squeezing the bracket inside the existing 44/22 px strips avoided a 91-call-site refactor of `HEADER_W` / `HEADER_H`). Fixture: `tests/fixtures/outline/outline-groups.xlsx`. **Open:** proper Excel-style outline gutter strip outside the row/col header strips (with summary +/- buttons + level numerals at the corner) is the planned follow-up. Hsx divergence: SpreadJS doesn't render outline gutters in screenshot mode at all; Excel desktop does and we match it. |
| Freeze panes              | ✅      | ✅     | 4-pane split.                             |
| Split panes (non-frozen)  | ❌      | ❌     |                                           |
| Merged cells              | ✅      | ✅     |                                           |
| Right-to-left sheet       | ❌      | ❌     |                                           |
| Print areas / page breaks | n/a     | n/a    | print fidelity is out of scope for v0.    |

### Conditional formatting

| Feature                            | Extract | Render | Notes                                           |
| ---------------------------------- | ------- | ------ | ----------------------------------------------- |
| `colorScale` (2-stop / 3-stop)     | ✅      | ✅     | min/max/percent/percentile/num.                 |
| `dataBar`                          | 🟡      | 🟡     | Legacy `<dataBar>` parsed; renderer paints proportional fill (gradient by default — `createLinearGradient` from anchor to tip with stops `color@1.0 → color@0.8 at 70% → color@0.05`), splits at zero for mixed-sign ranges, suppresses text on `showValue=false`. New schema field `CfDataBar.gradient: bool` defaults true (matches Excel 2010+, SpreadJS, LibreOffice); when x14 parsing lands the extractor will read the actual flag from `<x14:dataBar gradient="..."/>`. Open: x14 extension (canonical color / negative color / axis color / `automin`-`automax` cfvos / `minLength`-`maxLength`). Fixture + workarounds: `tests/fixtures/cf/{data-bar.xlsx,TRIAGE.md}`. |
| `iconSet`                          | ✅      | ✅     | All 17 OOXML preset IDs parsed; renderer paints hand-coded canvas paths for arrows / traffic-lights / signs / flags / symbols / ratings / quarters / boxes / triangles / stars (red→yellow→green ramps for color sets, gray ramp for `*Gray`). Bucket assignment is `largest k where v >= cfvo[k]`; `reverse` swaps. `showValue=false` suppresses cell text. Open: x14 extension (custom thresholds + per-icon mixing), curved arrow shapes, row-height–aware sizing. Fixture: `tests/fixtures/cf/icon-set.xlsx`. |
| `cellIs` (>, <, between, …)        | ✅      | ✅     | All 8 operators (eq/ne/gt/ge/lt/le/between/notBetween) with literal-number / quoted-string operands; cell-ref / formula operands need recalc and are skipped (false). Excel text-vs-number ordering quirk (text > any number) is honored. Fixture: `tests/fixtures/cf/cell-is.xlsx`. |
| `expression` (formula)             | 🟡      | ❌     | needs IronCalc to evaluate formula per cell.    |
| `top10` / `bottom10`               | ✅      | ✅     | `rank` count + `percent` flag + `bottom` flag. Ties at the cutoff value also match (Excel behavior). Non-numeric cells never match. Fixture: `tests/fixtures/cf/cf-non-recalc.xlsx`. |
| `aboveAverage` / `belowAverage`    | ✅      | ✅     | Plain mean compare + `equalAverage` inclusive variant + `stdDev` (N-stdev band, population variance). Non-numeric cells skipped from the mean. Fixture: `tests/fixtures/cf/cf-non-recalc.xlsx`. |
| `containsText` / friends           | ✅      | ✅     | All four kinds — `containsText` / `notContainsText` / `beginsWith` / `endsWith` — case-insensitive against the displayed text. `notContainsText` matches empty cells (Excel parity). Fixture: `tests/fixtures/cf/cf-non-recalc.xlsx`. |
| `duplicateValues` / `uniqueValues` | ✅      | ✅     | Count-by-value over the rule's combined ranges; numbers and text-of-the-same-digits stay in distinct buckets (`1` ≠ `"1"`, mirrors Excel). Empty cells excluded. Fixture: `tests/fixtures/cf/cf-non-recalc.xlsx`. |
| `timePeriod`                       | ✅      | ✅     | All 10 named periods (yesterday / today / tomorrow / last7Days / lastWeek / thisWeek / nextWeek / lastMonth / thisMonth / nextMonth) evaluated against the wall-clock at render time. Excel weeks are Sunday–Saturday. **Not in the fixture corpus** because the matching set rotates daily and would invalidate the snapshot. |
| Stop-if-true semantics             | ✅      | ✅     | Cross-kind masking: a higher-priority rule with `stopIfTrue=true` suppresses every lower-priority rule on the same cell, regardless of kind (cellIs / colorScale / dataBar / iconSet all participate). Implemented via a single `computeCfStopLocks(sheet, layout)` upfront pass that flattens all CF blocks, sorts by priority globally, and emits `Map<cellKey, lockedAtPriority>`; each visual pass (`computeCfDxfMap`, color-scale paint, data-bar paint, `computeCfIconState`, `computeCfTextSuppress`) calls `isCfLocked(locks, k, rule.priority)` and skips. Predicate kinds (`cellIs`, `top10`, etc.) lock only the cells whose value matches; visual kinds (`colorScale`/`dataBar`/`iconSet`) lock every cell in their `sqref` (Excel's UI doesn't allow stopIfTrue here, but the OOXML schema does and we honor it). `expression` doesn't lock without recalc — better to under-mask than over-mask. Fixture: `tests/fixtures/cf/stop-if-true.xlsx` (4 columns: control + cellIs masking colorScale / dataBar / iconSet). Pixel-matches hsx. |

### Tables, validation, charts, drawings

| Feature                                   | Extract | Render | Notes                                                                                                             |
| ----------------------------------------- | ------- | ------ | ----------------------------------------------------------------------------------------------------------------- |
| `<table>` (ListObject)                    | ✅      | 🟡     | Schema: `Table { name, displayName, range, headerRowCount, totalsRowCount, columns[], style { name, show* }, hasAutoFilter }`. Renderer paints header band (theme accent picked from `TableStyleMediumN`/etc. trailing index), bold-white header text, banded data rows (12% accent tint), filter-arrow glyphs in header cells, and a tinted totals row. No filtering interactivity. Open: per-row borders inside the table, exact match for Light/Dark style intensities, custom user table styles. Fixture: `tests/fixtures/tables/table-medium.xlsx`. |
| Pivot tables                              | 🟡      | 🟡     | Cheap path: extractor surfaces `Pivot { name, range, filterArrowCells[] }` from `xl/pivotTables/pivotTableN.xml`; renderer paints filter-dropdown chevrons on the row-field + col-field axis label cells (re-using the table chrome's `drawFilterArrows`). The materialized result cells (header band, banded rows, grand-total row) come for free — Excel writes them into `<sheetData>` with explicit cell xfs. No filtering / refresh / expand-collapse interactivity (that needs an aggregation engine; months). Open: multi-row/col pivots, page fields, compact layout, `<pivotTableStyle>` variants beyond the default Medium2 chrome. Fixture: `tests/fixtures/pivot/pivot-simple.xlsx`. |
| Slicers / Timelines                       | ❌      | ❌     |                                                                                                                   |
| Data validation (list / decimal / date)   | ❌      | ❌     | dropdowns are a compositor concern; not a render-only thing.                                                      |
| AutoFilter (filtered rows)                | ❌      | ❌     | rows that are filtered out should hide.                                                                           |
| Chart: column / bar (clustered + stacked) | ✅      | ✅     | with theme colors, axis ticks, legend.                                                                            |
| Chart: line                               | ✅      | ✅     | Standard / stacked / percentStacked groupings; markers per data point. Series colors resolve via shared `common_series` (explicit srgbClr → schemeClr accent → Office defaults). Fixture: `tests/fixtures/charts/line-pie-area-scatter.xlsx`. |
| Chart: pie / doughnut                     | ✅      | ✅     | Slice geometry + doughnut hole correct; sweep starts at 12 o'clock. **Per-slice colors land via `ChartSeries.pointColors`** — extractor pulls each `<c:dPt>`'s `spPr/solidFill/srgbClr` and the renderer prefers `pointColors[i]` over the fixed 6-color palette. Falls back to the palette when the workbook didn't write `<c:dPt>` blocks (the common hsx-emitted case). Fixture: `tests/fixtures/charts/pie-explicit-points.xlsx`. Open: legend per-category instead of per-series, data labels. |
| Chart: scatter / bubble                   | ✅      | 🟡     | Scatter renders xy points with numeric x-axis; schema fields `xValues` / `xValuesRef` (resolved post-sheet-extract) + `scatterStyle` on `Chart`. ECMA-376 §21.2.2.193 styles all wired: `marker` / `lineMarker` / `line` (straight segments through x-sorted points) / `smooth` / `smoothMarker` (Catmull-Rom→Bezier with tension 0.5). Unset `scatterStyle` defaults to marker-only — matches Excel's *UI* default for new scatters even though the OOXML enum default is `line`. Divergence: `hsx` paints `lineMarker` as marker-only (ignores the explicit style); we draw the connecting line per Excel desktop. Fixture: `tests/fixtures/charts/line-pie-area-scatter.xlsx`. Open: bubble sizing. |
| Chart: area (stacked / 100%)              | ✅      | ✅     | Default stacked behavior matches Excel; `standard` (overlapping) and `percentStacked` also handled. Translucent fill + outlined top edge. |
| Chart: combo / secondary axis             | ❌      | ❌     |                                                                                                                   |
| Chart: data labels                        | ✅      | ✅     | `<c:dLbls>` extracted at chart-group + per-series level. New `DataLabels { showValue, showCategory, showSeriesName, showPercent, position, separator, numFmt }` schema; series-level overrides chart-level. Renderer paints labels for column/bar (outEnd/inEnd/inBase/ctr), line (t/b/l/r/ctr above markers), area (top edge of each segment), pie/doughnut (outEnd/ctr/inEnd, percent computed against series total), scatter (right of each xy point). White halo behind each label for legibility on filled bars. Open: leader-line drawing for pie outside labels (cosmetic; Excel draws thin gray lines from the slice edge to outside labels when they would collide with the pie); per-point `<c:dLbl idx=N>` overrides; `<c:dLbls>` ext-list `dispBlanksAs` propagation. ooxmlsdk parse quirk: SpreadJS-emitted pies write `<c:leaderLines>` AFTER `<c:extLst>` inside `<c:dLbls>`, which trips the SDK's strict-sequence parser and silently zeros the show* fields — the fixture build script post-patches the offending block via Python zip-edit. Real Excel-emitted pies put leaderLines before extLst and parse cleanly. Fixture: `tests/fixtures/charts/data-labels.xlsx` (column/bar/line/area/pie/scatter, six configurations). |
| Sparklines                                | ❌      | ❌     | stored under `extLst`.                                                                                            |
| Images (raster)                           | ✅      | ✅     | base64 inline.                                                                                                    |
| Images: cropped / rotated                 | 🟡      | 🟡     | crop/rotation transforms ignored.                                                                                 |
| Shapes                                    | ❌      | ❌     | placeholder grey box.                                                                                             |
| SmartArt                                  | ❌      | ❌     |                                                                                                                   |

### Annotations & links

| Feature                      | Extract | Render | Notes                                     |
| ---------------------------- | ------- | ------ | ----------------------------------------- |
| Comments / threaded comments | ✅      | ✅     | Schema: `Comment { r, c, author, text, runs }` extracted from the `WorksheetCommentsPart`; `<authors>` table resolved by id, rich-text body preserved as `TextRun[]`. Renderer paints a 6px red right-triangle in the cell's top-right corner; `interact.ts` adds a hover popover (pale-yellow Excel-style box, bold author + body, repositions when it would clip off-viewport). Open: threaded-comment thread (separate `WorksheetThreadedCommentsPart`). Fixture: `tests/fixtures/annotations/hyperlinks-comments.xlsx`. |
| Hyperlinks                   | ✅      | ✅     | Schema: `Hyperlink { range, target, location, tooltip, display }`; `r:id` resolves to absolute URL via the worksheet's hyperlink rels at extract time. Renderer overlays a `Dxf { fontColor: theme[10], underline: true }` so the standard text-styling pass colors + underlines the cell. `interact.ts` swaps the cursor to `pointer`, opens external targets in a new tab via `window.open(target, "_blank", "noopener")`, and dispatches a `xlcore-hyperlink-jump` CustomEvent for in-workbook locations so the host can scroll/select. Fixture: `tests/fixtures/annotations/hyperlinks-comments.xlsx`. |
| Defined names                | n/a     | n/a    | engine concern (recalc), not render.      |

### Engine (formula recalc)

| Feature                                                                               | State | Notes                                    |
| ------------------------------------------------------------------------------------- | ----- | ---------------------------------------- |
| Source-cached `<v>` values                                                            | ✅    | what we currently render. Zero recalc.   |
| IronCalc fork integration                                                             | ❌    | milestone 1 in `plan-excel-rust-lib.md`. |
| `SUMPRODUCT`, `LET`, `LAMBDA`, `FILTER`, `SORT`, `UNIQUE`, `SEQUENCE`, dynamic arrays | ❌    | function gap list lives in the plan.     |

## Quick wins (next in priority order)

The first 4 items are small, high-visibility, and would each catch a class
of real-world workbooks looking wrong.

1. ~~**Theme XML parsing.**~~ **DONE.** `xl/theme/theme1.xml` is parsed
   in `xlcore-export/theme.rs` and emitted as `WorkbookLayout.theme`
   (12 hex colors in spreadsheet-index order + major/minor font names).
   The renderer reads it via `setActiveTheme()` at the top of `render()`;
   chart series colors resolve `accent{n}` via `theme.colors[3+n]`.
   Fixture: `tests/fixtures/themes/custom-theme-accent.xlsx`. Open
   extensions (HLS tints, scrgb/hsl/prst color resolvers) tracked under
   "Bigger lifts".

2. **Conditional formatting beyond color scales.** In rough effort order:
   - ~~`cellIs`~~ **DONE.** `<dxfs>` table + per-rule operator/operands/dxfId
     wired through schema; renderer evaluates 8 operators against literal
     operands and overlays the dxf (font color/bold/italic/underline/strike,
     solid fill foreground). Honors rule priority + `stopIfTrue`. Fixture:
     `tests/fixtures/cf/cell-is.xlsx`. Open: cell-ref / formula operands
     (need IronCalc); `numFmt` overlay from dxf is extracted but the renderer
     doesn't re-derive the formatted text yet (cosmetic).
   - ~~`dataBar`~~ **DONE.** Per-cell horizontal bar = `clamp((v-min)/(max-min), 0, 1) * (maxLenPct - minLenPct) + minLenPct`
     of cell width. Mixed-sign ranges split at zero (negative half
     paints red); `showValue=false` suppresses cell text. Hsx writes
     incomplete `<dataBar>` XML (no `<color>` child, defaults wrong)
     so the fixture build-script post-patches with a zip rewrite.
     ~~Gradient fill~~ **DONE** — `CfDataBar.gradient: bool` defaults
     true and the renderer uses `createLinearGradient` (anchor→tip:
     `color@1.0 → color@0.8 at 70% → color@0.05`) instead of a flat
     fill. Pixel-matches hsx on `tests/fixtures/cf/data-bar.xlsx`
     across the auto / num-num / mixed-sign / barOnly / explicit-gradient
     rows. Open sub-item: x14 extension parsing (would let users
     turn gradient off). See `tests/fixtures/cf/TRIAGE.md`.
   - ~~`iconSet`~~ **DONE.** Schema: `CfIconSet { iconSet, cfvos[],
     showValue, reverse }` extracted from legacy `<x:iconSet>`.
     Renderer: per-cell glyph drawn as canvas paths (arrows / traffic
     lights / signs / symbols / ratings / quarters / etc.); reserves
     18px on the cell's left and shifts left-aligned text right.
     `largest k where v >= cfvo[k]` bucket selection; `reverse=true`
     swaps the order. `showValue=false` hides text. Fixture:
     `tests/fixtures/cf/icon-set.xlsx`; visual matches hsx in shape
     placement and colors (paths are hand-drawn so curve detail
     differs).
   - ~~`top10` / `bottom10`~~ **DONE.** `rank` (count) or `rank %`
     when `percent=true`, `bottom=true` flips to bottom-N. Ties at the
     cutoff value also match. Non-numeric cells never match.
   - ~~`aboveAverage` / `belowAverage`~~ **DONE.** Plain mean compare,
     `equalAverage` inclusive variant, and `stdDev` (N-stdev band over
     the population variance, matching Excel's `STDEVP`-based formula).
   - ~~`duplicateValues` / `uniqueValues`~~ **DONE.** Bucket by
     normalized value (text vs number kept distinct).
   - ~~`containsText` / `notContainsText` / `beginsWith` / `endsWith`~~
     **DONE.** Case-insensitive against the displayed text;
     `notContainsText` matches empty cells (Excel parity).
   - ~~`timePeriod`~~ **DONE.** All 10 named periods against the
     wall-clock at render time. Excel weeks are Sun–Sat. Not in the
     fixture corpus because the matching set rotates daily.
   - `expression` — punt until IronCalc lands; needs to evaluate the
     formula per cell.

3. ~~**Tables (`<table>` ListObjects).**~~ **DONE.** Schema:
   `Table { name, displayName, range, headerRowCount, totalsRowCount,
   columns[], style { name, show* }, hasAutoFilter }` extracted from
   the worksheet's `<tablePart>` rels. Renderer: table chrome is folded
   into the same dxf-overlay map that conditional formatting uses
   (header gets accent fill + white bold text; banded data rows get a
   12% tint of the accent; totals row gets bold + tint), with filter-
   arrow glyphs painted in their own pass. Style accent index comes
   from the trailing integer in the style name (`TableStyleMedium2`
   → accent1, `Medium7` → accent6). CF rules win over table chrome on
   conflicting cells. Fixture: `tests/fixtures/tables/table-medium.xlsx`.
   Open: per-row borders inside the table (Excel paints a thin rule
   under the header and above the totals), Light/Dark style intensity
   variants, user-defined `<customTableStyles>` from `styles.xml`.

4. ~~**Number-format compiler.**~~ **DONE.** `render-ts/src/numfmt.ts`
   ships a real format-section evaluator: tokenize → split sections →
   pick by sign/condition → dispatch to number/date/fraction/scientific
   renderer. Color (`[Red]`, `[Color12]`) propagates to the cell's text
   spans. Verified against four fixtures in `tests/fixtures/numfmt/`
   (date/time, currency-locale, multi-section conditions, fractions +
   scientific). All 45 sample rows match hsx to the character. Open
   sub-items: `_x`/`*x` width-aware padding (needs cell-width plumbing),
   per-format memoization, locale separator, parser-on-Rust-side
   refactor — see `tests/fixtures/numfmt/TRIAGE.md`.

5. ~~**Charts: line / pie / scatter / area.**~~ **DONE.** Four new
   drawers in `render-ts/src/chart.ts`:
   - `drawLineChart` — path strokes per series + circle markers; honors
     `grouping=stacked`/`percentStacked` (uses `buildStackedRows` helper).
   - `drawAreaChart` — filled polygon per series with translucent fill
     and outlined top edge; default stacked, supports `standard` and
     `percentStacked`.
   - `drawPieChart` — slice-per-category with `arc()`; doughnut variant
     punches a center hole. (Per-slice colors cycle a 6-color palette
     since our schema only carries series-level color today.)
   - `drawScatterChart` — point markers on a numeric x-axis using a new
     `ChartSeries.xValues` / `xValuesRef` schema field, with the same
     post-sheet-extract resolver path as `valuesRef`.
   Extraction was generalized: `common_series` now handles bar/line/area/
   pie via shared field shapes, and a parallel `common_series_scatter`
   reads `c:yVal` / `c:xVal`. Fixture: `tests/fixtures/charts/line-pie-area-scatter.xlsx`.
   Open sub-items: pie legend per-category, bubble sizing — all small.
   ~~Scatter connecting lines (`scatterStyle=lineMarker`)~~ **DONE** —
   `Chart.scatterStyle` extracted from `<c:scatterStyle val=...>`;
   renderer in `drawScatterChart` strokes lines through points in
   x-sorted order for `line` / `lineMarker` and Catmull-Rom-via-Bezier
   curves for `smooth` / `smoothMarker`. Markers suppressed for plain
   `line` / `smooth`. Hsx divergence noted in PARITY row.
   ~~Data labels~~ **DONE** (own row above).

6. ~~**Comments + hyperlinks.**~~ **DONE.** Schema: `Hyperlink { range,
   target, location, tooltip, display }` + `Comment { r, c, author,
   text, runs }` on `Sheet`. Extractor resolves hyperlink `r:id` rels to
   absolute URLs and pulls comment authors from the comments part's
   `<authors>` table; rich-text bodies preserve runs. Renderer overlays
   a synthetic dxf (`fontColor: theme[10] /* hlink */, underline: true`)
   for hyperlink cells — same plumbing as table chrome — and paints a
   6px red right-triangle in the top-right corner of every commented
   cell. `interact.ts` adds the actual interactivity: cursor swap to
   `pointer` over hyperlink cells, single-click opens external targets
   in a new tab and emits a `xlcore-hyperlink-jump` CustomEvent for
   in-workbook locations, hover over a commented cell shows a pale-
   yellow Excel-style popover with bold author + body. Bonus: text
   underline + strikethrough now actually paint on the canvas
   (previously extracted and silently dropped); see the
   `paintTextDecorations` helper. Fixture:
   `tests/fixtures/annotations/hyperlinks-comments.xlsx`. Open:
   double / accounting underline variants; threaded-comments part.

7. ~~**Pivot tables (cheap path).**~~ **DONE.** Schema:
   `Pivot { name, range, filterArrowCells[] }` extracted from
   `xl/pivotTables/pivotTableN.xml`. The materialized result cells
   already live in `<sheetData>` with explicit cell xfs (header
   band, banded rows, bold grand-total row), so the only thing the
   renderer needs to add is the **filter-dropdown chevron** on the
   row-field + col-field axis label cells — we re-use the table
   chrome's `drawFilterArrows` and feed it the extractor-precomputed
   `filterArrowCells` list (computed from `<location firstHeaderRow
   firstDataRow firstDataCol/>` + which axes have fields). Bonus
   shipped at the same time: `WorkbookLayout.activeSheetIndex`
   honors `<bookViews><workbookView activeTab="N"/></bookViews>` so
   pivot demos open on the right tab. Fixture:
   `tests/fixtures/pivot/pivot-simple.xlsx`. **Open** (not in cheap
   path): page-field rows above the pivot, multi-row/col compound
   field layouts, expand/collapse glyphs, true filtering
   interactivity — all need an aggregation engine (Bar 2 in the
   `pivot has to work` decomposition; months of work).

Bigger lifts (own milestones):

- ~~**Diagonal borders.**~~ **DONE.** `Border.diagonalUp` /
  `diagonalDown` / `diagonal` (style + color) added to the schema;
  extractor reads the attrs + `<diagonal>` child; renderer draws
  clipped slashes in `drawDiagonalBorders`. Fixture:
  `tests/fixtures/borders/diagonal.xlsx` covers down-only / up-only /
  X / X-thick / X-dashed-red. Pixel-matches hsx.
- ~~**Per-slice colors for pie / doughnut.**~~ **DONE.**
  `ChartSeries.pointColors: Vec<String>` extracted from `<c:dPt>` per
  slice; renderer prefers it over the default 6-color palette in
  `drawPieChart`. Fix also caught a latent bug in
  `series_color_via_debug` (Debug-string scan was looking for the XML
  qname `srgbClr` instead of the Rust struct name `RgbColorModelHex`,
  so explicit series-level fills had been silently falling through to
  the theme accent for as long as the function existed). Fixture:
  `tests/fixtures/charts/pie-explicit-points.xlsx`.
- ~~**Theme color tints (proper HLS).**~~ **DONE.** `applyTint` in
  `render-ts/src/render.ts` now does RGB → HSL → scale-luminance → RGB
  per ECMA-376 §18.8.19. Lightening Accent1 (#4472C4) by +0.8 / +0.6
  / +0.4 and darkening by -0.25 / -0.5 all match Excel's color picker
  to within ±2/255 per channel (rounding wobble; Excel uses 240-step
  HLSMAX, we work in [0,1]). Unit tests live in
  `render-ts/src/render.test.ts`. Visually verified on the
  `tables/table-medium.xlsx` fixture (12% accent banded rows).
- ~~**Theme color: non-srgb resolvers.**~~ **DONE.**
  `<a:scrgbClr>`, `<a:hslClr>`, and `<a:prstClr>` now all resolve in
  `crates/xlcore-export/src/theme.rs` instead of falling through to the
  Office default:
  - `scrgbClr` — RGB percentages stored in 1000ths (0..100000) → 0..255
    bytes via `scrgb_byte`.
  - `hslClr` — OOXML HSL (hue in 60000ths of a degree, sat/lum in
    100000ths of a percent) via standard sRGB `hsl_to_rgb` (CSS Color 3).
  - `prstClr` — lookup against a 190-entry table generated from the
    schema enum: CSS3/X11 named colors + the OOXML-specific
    `dk*`/`lt*`/`med*` abbreviations + 2010-era duplicates (e.g.
    `MediumAquamarine2010` aliases `MedAquamarine`). Color modifier
    children (`<a:tint>`, `<a:shade>`, `<a:lumMod>`, `<a:satMod>`,
    `<a:alpha>`) are intentionally not applied at the theme level —
    cell-level tints already run through `applyTint` in the renderer
    against the resolved hex. Unit tests cover `scrgb_byte` extremes,
    HSL primaries (red/green/blue/black/white/gray), and hex format.
- **Formula recalc.** Forking IronCalc + filling its function gaps is
  milestone 1 in `plan-excel-rust-lib.md`.
- ~~**node-canvas backend.**~~ **DONE.** `@xlcore/render-ts` now exports
  `renderToCanvas()` / `renderToPng()` from `render-ts/src/node.ts`, backed by
  `@napi-rs/canvas` and the exact same `render()` pass used by the browser
  preview. Pattern-fill offscreen canvases route through a shared factory so
  hatch fills keep working outside `document`. Open follow-up: wire this into
  fixture pixel-diff CI.
- **Filtered-row hiding (autoFilter).** Needs the engine OR a "hidden row"
  fast-path keyed off `<row hidden="1">` markers some writers emit.
- **Sparklines.** Stored under `extLst`; chart-class effort.
- **Selection / interactivity bug-for-bug parity.** Out of scope for the
  render path; `interact.ts` covers the basics.

## Schema sync

The Rust extractor and the TS renderer are kept in lock-step by
[`ts-rs`](https://github.com/Aleph-Alpha/ts-rs). Every `WorkbookLayout`
type in `crates/xlcore-export/src/schema.rs` derives `TS` and emits a
generated TS file under `render-ts/src/schema/<TypeName>.ts`.
`render-ts/src/types.ts` is a tiny barrel that re-exports them so the rest
of the renderer doesn't notice.

Regenerate after any schema change:

```bash
cargo test --release -p xlcore-export export_bindings
# rebuilds render-ts/src/schema/*.ts; if new types were added, also update
# the barrel `render-ts/src/schema/index.ts` and `render-ts/src/types.ts`.
```

This would have caught the `wrap_text` / `wrapText` mismatch we shipped in
v0 that silently disabled all text wrapping. It does _not_ catch logic bugs
where the renderer reads the right field but interprets it wrong; those
need a visual diff (below).

CI guard (planned, see [Open work](#open-work)): run the export, then
`git diff --exit-code render-ts/src/schema/`.

Conventions enforced by attributes on the Rust types:

- `#[serde(rename_all = "camelCase")]` at struct level → JSON keys are
  camelCase, matching what the renderer expects.
- `#[ts(optional)]` on every `Option<T>` field → emits `field?: T`
  (matches the runtime shape, since `skip_serializing_if = "Option::is_none"`
  drops the key entirely when the value is `None`).
- `#[ts(type = "number")]` on `i64` EMU offsets → emits `number` instead of
  `bigint` (EMU values fit in JS Number safely).

Add new types to **all of**:

1. `crates/xlcore-export/src/schema.rs` with the three attributes above.
2. `render-ts/src/schema/index.ts` (barrel).
3. `render-ts/src/types.ts` (re-export list).

## Fixture corpus (in progress)

Fixtures live in `tests/fixtures/` (source-controlled). See
[`tests/fixtures/README.md`](tests/fixtures/README.md) for the live
table + how to add new ones. Today:

- `kitchensink/kitchensink.xlsx` — the canonical mixed workbook.
- `themes/custom-theme-accent.xlsx` — theme-color resolution per
  spreadsheet `theme="N"` slot, against a non-default palette.

The goal of the corpus is that **a failed visual diff names the
suspect**. Target sketch (each row a future fixture; ones already
landed are checked):

```
tests/fixtures/
  kitchensink/                # ✅ landed
    kitchensink.xlsx
    build.sh
  text/
    rich-text-runs.xlsx       # bold/italic/color spans + \n
    wrap-text.xlsx
    overflow-into-empty.xlsx
    merged-with-borders.xlsx  # would have caught the perimeter-border bug
    indent-rotation.xlsx
  cf/
    color-scale-2-stop.xlsx
    color-scale-3-stop.xlsx
    data-bar.xlsx
    icon-set-arrows.xlsx
    cell-is-greater.xlsx
    expression-formula.xlsx
    top10.xlsx
    duplicate-values.xlsx
  numfmt/
    currency-locale.xlsx
    date-time-formats.xlsx
    custom-section-conditions.xlsx   # `[Red]` / `[>100]` etc.
    fraction-and-scientific.xlsx
  borders/
    every-style.xlsx          # thin/medium/thick/dashed/double/etc.
    diagonal.xlsx
    around-merged-range.xlsx
  fills/
    pattern-types.xlsx
    gradient-linear.xlsx
    theme-color-tints.xlsx
  charts/
    line.xlsx
    pie.xlsx
    scatter.xlsx
    area-stacked.xlsx
    combo-secondary-axis.xlsx
    data-labels.xlsx
  drawings/
    image-anchored.xlsx
    image-cropped.xlsx
    shape.xlsx
  pivot/
    pivot-simple.xlsx
    pivot-multi-row.xlsx
    pivot-with-slicer.xlsx
  tables/
    table-medium-style.xlsx
    table-with-totals.xlsx
  validation/
    list-dropdown.xlsx
    decimal-range.xlsx
  layout/
    freeze-rows-cols.xlsx
    hidden-rows-cols.xlsx
    grouped-outlined.xlsx
    rtl-sheet.xlsx
  text-overflow/
    long-string-into-empty.xlsx
    long-string-into-occupied.xlsx
    centered-merge-overflow.xlsx
  themes/                     # ✅ custom-theme-accent landed
    custom-theme-accent.xlsx
    dark-theme.xlsx
```

### How fixtures are built

One bash script per fixture, using `hsx` to write the file. Reproducible
and reviewable in git; no hand-edited binary blobs. Example:

```bash
# tests/fixtures/text/build-rich-text-runs.sh
set -e
out="$(dirname "$0")/rich-text-runs.xlsx"
hsx create "$out"
hsx eval "$out" '
  range("A1").value({
    richText: [
      { text: "Bold", style: { fontStyle: { bold: true }}},
      { text: " then italic", style: { fontStyle: { italic: true }}},
      { text: "\nnew line", style: {}},
    ]
  });
  sheet.setColumnWidth(0, 200);
  sheet.getCell(0, 0).wordWrap(true);
'
```

A top-level `tests/fixtures/build-all.sh` rebuilds every fixture; CI
verifies the script and the committed `.xlsx` agree.

### Reference artifacts per fixture

For each `X.xlsx`, commit two snapshots alongside the script:

- `X.layout.json` — ground-truth `WorkbookLayout` JSON. Regenerate by
  running `xlcore extract` and accepting via `cargo insta accept`.
- `X.hsx.png` — `hsx screenshot X.xlsx -o X.hsx.png` at a fixed viewport.

CI loop (planned):

```
for fixture in tests/fixtures/**/*.xlsx; do
  cargo run -- extract "$fixture" -o /tmp/out.json
  diff <(jq -S . "$fixture.layout.json") <(jq -S . /tmp/out.json)  # data
  cargo run -- preview "$fixture" -o /tmp/preview.html
  # render via browser-harness, pixel-diff against $fixture.hsx.png
done
```

A **kitchen-sink** fixture stays in addition to the unit ones — catches
feature-interaction bugs (CF + merged + wrapped).

## Manual visual-diff workflow (current)

Until the CI loop above lands, use this for spot-checks:

```bash
# 1. build
cd render-ts && bun run build && cd ..
cargo build --release

# 2. our render
./target/release/xlcore preview path/to/file.xlsx -o /tmp/preview.html
uv run browser-harness <<'PY'
goto("file:///tmp/preview.html")
wait_for_load()
import time; time.sleep(2)
screenshot("/tmp/ours.png")
PY

# 3. ground truth
hsx screenshot path/to/file.xlsx -o /tmp/hsx.png

# 4. compare
__PI_IMAGE__ /tmp/ours.png /tmp/hsx.png   # in clawd; or whatever side-by-side viewer
```

Browser-harness occasionally throws `no close frame received or sent` — fix
with `cd ~/Developer/browser-harness && uv run python -c "from admin import
restart_daemon; restart_daemon()"` then retry.

Range screenshots from `hsx`: `hsx screenshot file.xlsx "Sheet!A1:M30" -o
out.png`. Useful for narrowing in on a specific feature without the
surrounding workbook noise.

## Open work

- [x] Move `kitchensink.xlsx` into `tests/fixtures/` (source-controlled).
- [x] First per-feature fixture: `themes/custom-theme-accent.xlsx`
      (built via Python zip-patch; see `tests/fixtures/README.md` for
      why we sidestep `hsx` mid-stream there).
- [x] Number-format triage fixtures: `numfmt/{date-time-formats,
      currency-locale,custom-section-conditions,fraction-and-scientific}.xlsx`.
      Findings in `tests/fixtures/numfmt/TRIAGE.md`. Date/time + fractions +
      scientific + multi-section all confirmed broken; see triage doc for
      the format-section-evaluator design that replaces them.
- [x] CF `cellIs` rule: `tests/fixtures/cf/cell-is.xlsx` covers all 8
      operators against numeric + string cells; pixel-matches `hsx`.
- [x] CF `dataBar` rule: `tests/fixtures/cf/data-bar.xlsx` covers auto
      min/max, num min/max, mixed-sign axis split, `showValue=false`,
      and gradient-vs-solid (gradient still renders solid; tracked in
      `tests/fixtures/cf/TRIAGE.md`).
- [x] Tables (`<table>` ListObjects): `tests/fixtures/tables/table-medium.xlsx`
      covers `TableStyleMedium2` with header + 5 data rows + totals
      row + autoFilter; renderer matches hsx in header band color,
      bold white header text, filter-arrow glyphs, banded row stripes,
      and totals-row tint. Pixel diff is close (hsx draws a thicker
      header-bottom and totals-top border rule we skip).
- [x] Charts (line / pie / area / scatter):
      `tests/fixtures/charts/line-pie-area-scatter.xlsx` covers a line
      chart, a stacked area chart, a single-series pie, and an xy
      scatter, all driven from one tabular range. Renderer matches hsx
      in line trajectories, area stack ordering, slice proportions, and
      scatter point placement; small known gaps tracked in PARITY.md
      (per-slice colors, scatter line variant).
- [x] CF `iconSet` rule: `tests/fixtures/cf/icon-set.xlsx` covers
      3/4/5-stop sets (lights, arrows, symbols, ratings, quarters)
      plus `reverse` and `showValue=false` (icon-only). Glyphs are
      canvas paths so visual fidelity differs in curve detail; bucket
      math + colors + reserved text indent match hsx. Workarounds in
      `tests/fixtures/cf/TRIAGE.md`.
- [x] Pivot tables (cheap path): `tests/fixtures/pivot/pivot-simple.xlsx`
      — 12 source rows on Sheet1, a 4-region × 2-product pivot on
      Sheet "Pivot" with Sum-of-Amount values + grand totals. The
      materialized cells (header band, banded rows, bold totals)
      come straight from `<sheetData>` with their explicit xfs; the
      renderer adds the row-field and col-field filter chevrons
      (B3 "Region", C2 "Product"). Active tab opens on "Pivot" via
      the new `WorkbookLayout.activeSheetIndex` field. Pixel-matches
      hsx for the static snapshot.
- [x] Hyperlinks + comments: `tests/fixtures/annotations/hyperlinks-comments.xlsx`
      covers https / mailto / file:// external links, an in-workbook
      `#Sheet1!D7` location link, and three commented cells with
      distinct rich-text bodies. Renderer pixel-matches `hsx` for both
      blue+underlined link cells and red triangle markers.
- [x] Non-recalc CF rules (top10, aboveAverage, dup/unique, text,
      timePeriod): `tests/fixtures/cf/cf-non-recalc.xlsx` covers 14
      rules across 4 sections (top-N count + percent + bottom variants;
      above/below mean + equalAverage + stdDev; duplicate/unique;
      containsText / notContainsText / beginsWith / endsWith). Pixel-
      matches hsx. `timePeriod` ships in the renderer but is excluded
      from the fixture because matches rotate daily. Build script
      post-patches the worksheet XML around four hsx CF-emission bugs
      (empty `sqref` on top10, missing `dxfId`, mistyped `containsText`
      for all text rules, bogus `text="null"` attribute).
- [x] Indent (`textIndent`) rendering: `tests/fixtures/text/indent.xlsx`
      covers indent=0..5 on left- and right-aligned cells; renderer
      pixel-matches hsx in the staircase step rate.
- [x] Pattern fills (16 hatches + solid): `tests/fixtures/fills/patterns.xlsx`
      lays out gray125 / gray0625 / lightGray / mediumGray / darkGray /
      light+dark Horizontal / Vertical / Down / Up / Grid / Trellis +
      solid in a 6×3 grid with dark-blue fg on white bg. Built by
      `_patch_patterns.py` because hsx silently drops everything but
      `solid` on xlsx export. Renderer paints each as an 8x8 tile via
      `ctx.createPattern`; visual character matches hsx for all 18.
- [x] Diagonal borders: `tests/fixtures/borders/diagonal.xlsx` covers
      `<border>` `diagonalUp` / `diagonalDown` toggles + the shared
      `<diagonal>` child (style + color). Five cells: down-only,
      up-only, X (both), X-thick, X-dashed-red. Renderer clips
      diagonals to the cell rect and pixel-matches hsx. SpreadJS
      drops these on xlsx export so the fixture is built via a
      Python zip-patch.
- [x] Chart data labels (`<c:dLbls>`): `tests/fixtures/charts/data-labels.xlsx`
      lays out six charts side-by-side — column/bar/line/area/pie/scatter
      — each with a different `DataLabels` configuration covering
      `showValue`, `showCategory`, `showPercent`, plus `position` =
      outEnd / ctr / t / r. Renderer pixel-matches hsx in label content
      and approximate placement; minor differences in pie label radius
      (we place labels at r+12, hsx draws leader lines further out)
      and bar inEnd/inBase positioning when the label string is wider
      than the bar (we don't shrink-to-fit). Build script post-patches
      a SpreadJS-emitted ooxmlsdk parse bug — see PARITY.md row.
- [x] Text rotation rendering: `tests/fixtures/text/rotation.xlsx`
      covers OOXML `textRotation` 30 / 45 / 90 (CCW), 120 / 135 / 180 (CW),
      and 255 (stacked) in a row of column-header cells. Renderer paints
      each variant at the right tilt with all glyphs visible inside the
      author-sized row; small vertical-anchor mismatch on slanted angles
      tracked in PARITY.md.
- [x] `applyFont` / `applyFill` / `applyBorder` / `applyNumberFormat` /
      `applyAlignment` inheritance from `cellStyleXfs`:
      `tests/fixtures/styles/named-inheritance.xlsx` covers all five
      `apply*="0"` paths. Without the fix, every cell rendered as
      plain Calibri 11 (default xf ids); now Title / Heading 1 /
      Highlighted / Centered all pick up their named-style
      formatting. Hsx divergence noted in the fixture README.
- [x] Outline / group levels: `tests/fixtures/outline/outline-groups.xlsx`
      lays out two row groups (rows 3-4, 7-8 at `outlineLevel=1`) and
      one column group (cols B-D at `outlineLevel=1`). Schema additions:
      `Col.outlineLevel: u8`, `RowMetaBlob.outlineLevel` u8 blob (skipped
      from JSON when all-zero), `Sheet.outlinePr: Option<{summaryBelow,
      summaryRight}>`. Renderer paints the brackets inside the existing
      header strips (no layout shift, no `HEADER_W` / `HEADER_H`
      refactor); scrolling + freeze panes both honored via separate
      pinned/scrolling run passes per level. SpreadJS drops
      `outlineLevel` on xlsx export so the fixture is built via Python
      zip-patch (also splits the existing `<col min="2" max="6">` block
      into B-D / E-F so only the grouped columns get the attr). **Open:**
      proper Excel-style outline gutter strip outside the header strips
      (with +/- buttons + level numerals at the corner) is the planned
      follow-up.
- [x] CF stopIfTrue cross-kind masking: `tests/fixtures/cf/stop-if-true.xlsx`
      — four side-by-side columns each with values 1..10 and a
      `cellIs(>7)` yellow-fill dxf rule layered against (a) no stop,
      (b) a colorScale, (c) a dataBar, (d) an iconSet; the stopping
      cellIs rule has higher priority and `stopIfTrue=true`. Without
      the fix, lower-priority colorScale / dataBar / iconSet rules
      paint over the stopped cells; with the fix, rows 8–10 in cols
      B–D show only the yellow dxf, matching Excel + hsx. SpreadJS
      drops `stopIfTrue` on its public xlsx-emit path so the fixture
      is built via Python zip-patch (`_patch_stop_if_true.py`).
- [x] Every-border-style fixture: `tests/fixtures/borders/every-style.xlsx`
      lays out all 14 OOXML `ST_BorderStyle` values in a 2×7 grid,
      each on all four sides of its cell. Caught two latent bugs:
      (a) extractor returned `"dashDot"` for `slantDashDot` because
      the substring-match cascade tested the shorter `dashdot`
      first; (b) renderer painted `mediumDashDot` / `mediumDashDotDot` /
      `slantDashDot` as solid medium lines (no dash branch). Built via
      Python zip-patch (`_patch_every_style.py`) to write byte-exact
      OOXML; hsx's public API doesn't expose `slantDashDot`. Documented
      hsx divergence: SpreadJS draws all `*DashDot*` variants as solid
      lines, ours match Excel desktop. See PARITY row.
- [x] Underline variants (double / singleAccounting / doubleAccounting):
      `tests/fixtures/text/underline.xlsx` puts all 4 ST_UnderlineValues +
      a "no underline" control in a row of 5 cells. Schema gains
      `underlineStyle: Option<String>` on `Font` / `TextRun` / `Dxf`;
      extractor reads `<u val="..."/>` via a shared `underline_variant`
      helper. Renderer paints `double` / `doubleAccounting` as two
      parallel strokes (`gap = max(2, fontSizePx * 0.1)`); accounting
      variants currently render like their non-accounting siblings
      (the across-cell-width semantics need cell-rect plumbing into
      `paintTextDecorations`). Hsx draws all 4 variants as identical
      single thin lines so this fixture is built via Python zip-patch
      (hsx's public JS API only exposes a boolean `underline()`
      toggle anyway).
- [x] Gradient fills (linear multi-stop + path/radial):
      `tests/fixtures/fills/gradients.xlsx` lays out 6 cells covering
      linear `degree=0/45/90/270`, a 3-stop linear, and a path gradient
      with a centered inner-convergence rect. Schema gains `GradientStop
      { position, color }` (replaces `Vec<Color>` that silently dropped
      stop positions) plus `gradientType` / `gradientDegree` /
      `gradientLeft|Right|Top|Bottom`. Renderer in `cellPaint.ts` paints
      multi-stop linear via rotated-axis projection of the cell rect onto
      `(cosθ, sinθ)` (so position 0 hits the leading corner along the
      gradient axis and position 1 hits the trailing corner), and path
      gradients via `createRadialGradient` from the inner rect's bounding
      circle out to the farthest cell corner with the innermost stop
      pre-filled across the full cell. SpreadJS doesn't expose gradient
      fills on its public style API so the fixture is built via Python
      zip-patch (`_patch_gradients.py`).
- [ ] Land the next batch of per-feature fixtures (the four marked above
      as "would have caught X bug").
- [ ] CI guard: `cargo test … export_bindings && git diff --exit-code
    render-ts/src/schema/`.
- [ ] `cargo-insta` snapshot test on `WorkbookLayout` JSON for every
      fixture.
- [ ] Pixel-diff snapshot test — render via the new node-canvas adapter,
      imagehash against the stored `*.hsx.png`, fail CI on regression.
- [ ] `bun test` on pure-helper TS (`niceTicks`, `formatNumber`, A1
      helpers, `layoutSpans`).
