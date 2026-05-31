# Parity

Status of `xlcore` vs Excel / `hsx` (SpreadJS screenshot).

Legend: ✅ done · 🟡 partial · ❌ missing · n/a out of scope.

## Rules

| Layer | Question |
| --- | --- |
| Data | Does `xlcore-export` surface the OOXML feature in `WorkbookLayout` JSON? |
| Schema | Do Rust schema and TS renderer types match? |
| Visual | Does `xlsx-preview` render like `hsx` / Excel? |

- `hsx` is the usual visual baseline.
- When `hsx` disagrees with Excel desktop/spec, record it and pick the Excel/spec side.

## Cell content & formatting

| Feature | Extract | Render | Notes |
| --- | --- | --- | --- |
| Shared strings | ✅ | ✅ | |
| Inline strings | ✅ | ✅ | |
| Rich-text runs | ✅ | ✅ | font/bold/italic/color/size per run |
| Hard line breaks | ✅ | ✅ | `\n` |
| Soft wrap | ✅ | ✅ | per-run metrics |
| Text overflow into empty cells | n/a | ✅ | left/right/center; merged cells |
| Indent | ✅ | ✅ | `textIndent`; fixture `text/indent.xlsx` |
| Text rotation | ✅ | ✅ | 1–180 and 255 stacked; fixture `text/rotation.xlsx`; open vertical anchoring for slanted angles |
| Strikethrough | ✅ | ✅ | |
| Underline variants | ✅ | ✅ | single/double/accounting; fixture `text/underline.xlsx` |
| Font scheme | ✅ | ✅ | resolves theme major/minor; hsx ignores, Excel side chosen |
| Font family fallback | ✅ | ✅ | Roman/Swiss/Modern/Script/Decorative generics |
| Superscript / subscript | ✅ | ✅ | `vertAlign`; fixture `text/vertalign.xlsx` |
| Boolean unset attrs | ✅ | ✅ | e.g. `<i val="0"/>` |
| Theme XML colors | ✅ | ✅ | theme palette + font names |
| OOXML color choices | ✅ | ✅ | `srgbClr`, `sysClr.lastClr`, `scrgbClr`, `hslClr`, `prstClr` |
| Indexed colors | ✅ | ✅ | default palette; open workbook palette override |
| Tint | ✅ | ✅ | OOXML HLS-space tint |
| Built-in number formats | ✅ | ✅ | IDs 0–49 |
| Custom number formats | ✅ | ✅ | sections, colors, conditions, currency, trailing-comma scaling |
| Fractions | ✅ | ✅ | variable/fixed denominator |
| Scientific formats | ✅ | ✅ | classic + engineering shift |
| Named-style inheritance | ✅ | ✅ | `cellStyleXfs`, `apply*="0"`; hsx ignores, Excel side chosen |

## Borders & fills

| Feature | Extract | Render | Notes |
| --- | --- | --- | --- |
| Per-side borders | ✅ | ✅ | all 14 OOXML styles; hsx dash-dot divergence |
| Diagonal borders | ✅ | ✅ | up/down/both; merged ranges |
| Borders around merged ranges | ✅ | ✅ | perimeter cells |
| Solid fills | ✅ | ✅ | |
| Pattern fills | ✅ | ✅ | 18 OOXML patterns; 8×8 tiles |
| Linear gradients | ✅ | ✅ | multi-stop, arbitrary degree |
| Path/radial gradients | ✅ | ✅ | inner convergence rect; hsx divergence, Excel side chosen |

## Layout

| Feature | Extract | Render | Notes |
| --- | --- | --- | --- |
| Custom column widths | ✅ | ✅ | |
| Custom row heights | ✅ | ✅ | |
| Hidden rows / cols | ✅ | ✅ | zero-sized; collapsed-group ticks |
| Hidden / very-hidden sheets | ✅ | ✅ | `state` honored; `veryHidden` always off-chrome, `hidden` reveal via previewer `showHidden` option / `?showHidden=1`; fixture `sheets/hidden-and-tabcolor.xlsx` |
| Sheet tab color | ✅ | ✅ | `<sheetPr><tabColor/>` rgb/theme/indexed+tint; full inactive-tab fill + active-tab text color; fixture `sheets/hidden-and-tabcolor.xlsx` |
| Outline/group levels | ✅ | ✅ | gutters, brackets, +/- buttons, level buttons |
| Freeze panes | ✅ | ✅ | 4-pane split |
| Split panes | ❌ | ❌ | non-frozen panes |
| Merged cells | ✅ | ✅ | |
| Right-to-left sheet | ❌ | ❌ | |
| Print areas / page breaks | n/a | n/a | print fidelity out of scope |

## Conditional formatting

| Feature | Extract | Render | Notes |
| --- | --- | --- | --- |
| Color scale | ✅ | ✅ | 2-/3-stop; min/max/percent/percentile/num |
| Data bar | 🟡 | 🟡 | legacy parsed; open x14 extension details |
| Icon set | ✅ | ✅ | 17 preset IDs; open custom x14 icons/thresholds |
| `cellIs` | ✅ | ✅ | 8 operators; literals only |
| Formula expression | 🟡 | ❌ | needs formula engine |
| Top/bottom N | ✅ | ✅ | count/percent/ties |
| Above/below average | ✅ | ✅ | mean, equalAverage, stdDev |
| Text contains/begins/ends | ✅ | ✅ | case-insensitive displayed text |
| Duplicate/unique values | ✅ | ✅ | empty excluded; number/text distinct |
| Time period | ✅ | ✅ | 10 named periods; fixture `cf/time-period.xlsx` (rebuilt against current date) |
| Stop-if-true | ✅ | ✅ | cross-kind masking |

## Tables, pivots, charts, drawings

| Feature | Extract | Render | Notes |
| --- | --- | --- | --- |
| Tables / ListObjects | ✅ | ✅ | header, banding, filter glyphs, totals; no filter interactivity/re-evaluation. Custom `<tableStyles>` definitions resolved — header / firstRowStripe / totalRow / wholeTable dxfs apply via the same `cfDxfs` pipeline as conditional formatting; built-in `TableStyleMedium*` names still fall back to the accent-from-trailing-digit heuristic. Per-table direct overrides (`headerRowDxfId`, `dataDxfId`) not yet honored. |
| Pivot tables | 🟡 | 🟡 | static materialized cells + filter chevrons; no refresh/interactivity |
| Slicers / timelines | ❌ | ❌ | |
| Data validation | ❌ | ❌ | dropdowns are compositor/interactivity work |
| AutoFilter hidden rows | ✅ | ✅ | worksheet-level `<autoFilter ref>` surfaced as `autoFilterRange`; saved filter results collapse via serialized row `hidden` flags; fixture `tables/autofilter-hidden-rows.xlsx` |
| Chart: column/bar | ✅ | ✅ | clustered/stacked/percent, ticks, legend, theme colors |
| Chart: line | ✅ | ✅ | stacked/percent, outline colors, marker suppression |
| Chart: pie/doughnut | ✅ | ✅ | slices, hole, per-slice colors; open leader lines |
| Chart: scatter | ✅ | ✅ | marker/line/lineMarker/smooth/smoothMarker |
| Chart: bubble | ✅ | ✅ | size scaling, area/width modes |
| Chart: area | ✅ | ✅ | standard/stacked/percent |
| Chart: combo/secondary axis | ✅ | ✅ | multi-group plotAreas, dual y-axes, axis titles, data labels, `dispUnits` |
| Chart: data labels | ✅ | ✅ | chart/series/per-point overrides |
| ChartEx (`cx:`) | ✅ | ✅ | waterfall / funnel / treemap / sunburst / histogram / pareto / boxWhisker / regionMap all shipped. regionMap uses an embedded Natural Earth 110m countries dataset + choropleth painter; hsx falls back to clustered-column for that layout. See `docs/parity-charts.md`. |
| Sparklines | ✅ | ✅ | line/column/win-loss; open prefix robustness + marker shape |
| Raster images | ✅ | ✅ | base64 inline |
| Cropped/rotated images | 🟡 | 🟡 | transforms ignored |
| Shapes | 🟡 | 🟡 | v0 shipped. `<xdr:sp>` autoshapes and `<xdr:grpSp>` group shapes (with `xfrm`/`chOff`/`chExt` nested-frame mapping) extract via `crates/xlcore-export/src/shapes.rs`; renderer painter lives in `packages/xlsx-preview/src/shape.ts`. Coverage: `prstGeom` rect / roundRect / ellipse / triangle / diamond / leftArrow / rightArrow / upArrow / downArrow (unknown presets fall back to plain rectangle); `<a:solidFill>` with `srgbClr`/`schemeClr`/`prstClr`/`sysClr`; `<a:ln>` outline with width; theme color modifiers via existing `apply_color_modifiers`; text paragraphs with `<a:rPr sz="" b="" i=""/>`, solidFill run color, latin font (incl. `+mn-lt`/`+mj-lt` theme refs), `<a:pPr algn=""/>`, body-anchor (`t`/`ctr`/`b`), word-wrap on `<a:bodyPr wrap="square"/>` (Excel default; `wrap="none"` lets text overflow); `<a:xfrm rot=""/>` rotation; nested `<xdr:pic>` inside `<xdr:grpSp>` surfaces as a shape-node image leaf with `<a:srcRect>` crop (shares the top-level image-decode cache via `imageCache.ts`). Verified against the Microsoft Map Chart template (`chart-regionmap-chartex.xlsx`). Linear + path `<a:gradFill>` and direct `<a:effectLst><a:outerShdw>` (with `dist`/`dir`/`blurRad` → canvas `shadow*`, alpha modifier honored) also shipped — locked in by `shapes/gradient-fills.xlsx` and `shapes/outer-shadow.xlsx`. `<a:blipFill>` on `<xdr:sp>/<xdr:spPr>` (shape-as-image-fill, distinct from `<xdr:pic>`) painted by clipping the preset path and stretching the image into the bbox; `asvg:svgBlip` SVG sidecar wins over the raster fallback when present — locked in by `shapes/blip-fills.xlsx`. Detailed spec audit + prioritized backlog: `docs/parity-shapes.md`. |
| SmartArt | ❌ | ❌ | |

## Annotations & links

| Feature | Extract | Render | Notes |
| --- | --- | --- | --- |
| Comments | ✅ | ✅ | red marker + hover popover; threaded-comment thread open |
| Hyperlinks | ✅ | ✅ | external open + workbook-jump event |
| Defined names | n/a | n/a | engine/recalc concern |

## Engine

| Feature | State | Notes |
| --- | --- | --- |
| Cached `<v>` values | ✅ | current render source |
| Formula recalc | 🟡 | `xlcore-engine` crate started with a thin IronCalc facade, vendored IronCalc fork, core recalc tests, real `SUMPRODUCT`, scalar `LET` shim PoC, and first `xlcore-bridge` OOXML harvest/recalc API. OOXML/layout writeback still missing. See `docs/parity-engine.md`. |
| Modern/dynamic functions | 🟡 | `SUMPRODUCT` implemented in fork; scalar `LET` shim only. Missing real `LET`, `LAMBDA`, `FILTER`, `SORT`, `UNIQUE`, `SEQUENCE`, arrays |

## Major open work

| Area | Status |
| --- | --- |
| Formula engine / recalc | ❌ |
| Split panes | ❌ |
| RTL sheets | ❌ |
| Data validation UI | ❌ |
| Slicers / timelines | ❌ |
| Shapes / SmartArt | 🟡 (shapes v0 shipped — word-wrap + nested pictures inside groups now in; SmartArt still ❌) |
| Image crop/rotation | 🟡 |
| CF x14 extensions | 🟡 |
| Area chart gaps for missing points | 🟡 |
| Live recalc for chart/sparkline formula-only source cells | ❌ |

## Fixture corpus

| Location | Purpose |
| --- | --- |
| `tests/fixtures/README.md` | source of truth for fixture list |
| `tests/fixtures/**/build-*.sh` | reproducible fixture builders |
| `tests/fixtures/**/*.xlsx` | source workbooks |
| `tests/fixtures/**/*.hsx.png` | visual reference where committed |

Representative fixtures:

| Area | Fixture |
| --- | --- |
| Text | `text/indent.xlsx`, `text/rotation.xlsx`, `text/fontfamily.xlsx`, `text/vertalign.xlsx`, `text/underline.xlsx` |
| Number formats | `numfmt/date-time-formats.xlsx`, `numfmt/custom-section-conditions.xlsx`, `numfmt/fraction-and-scientific.xlsx` |
| Borders/fills | `borders/every-style.xlsx`, `borders/diagonal.xlsx`, `fills/patterns.xlsx`, `fills/gradients.xlsx` |
| Conditional formatting | `cf/cell-is.xlsx`, `cf/data-bar.xlsx`, `cf/icon-set.xlsx`, `cf/cf-non-recalc.xlsx`, `cf/stop-if-true.xlsx`, `cf/time-period.xlsx` |
| Tables/pivots | `tables/table-medium.xlsx`, `tables/autofilter-hidden-rows.xlsx`, `pivot/pivot-simple.xlsx` |
| Charts | `charts/line-pie-area-scatter.xlsx`, `charts/data-labels.xlsx`, `charts/bubble.xlsx`, `charts/chart-*.xlsx` |
| Layout | `outline/outline-groups.xlsx`, freeze-pane fixtures |
| Annotations | `annotations/hyperlinks-comments.xlsx` |
| Sheets | `sheets/hidden-and-tabcolor.xlsx` |
| Sparklines | `sparklines/sparklines.xlsx` |
| Styles/themes | `styles/named-inheritance.xlsx`, `themes/custom-theme-accent.xlsx` |

## Schema sync

| Task | Command / file |
| --- | --- |
| Regenerate TS bindings | `scripts/regen-schema.sh` (runs export tests + biome format) |
| Rust schema | `crates/xlcore-export/src/schema.rs` |
| TS generated schema | `packages/xlsx-preview/src/schema/*.ts` |
| TS barrel | `packages/xlsx-preview/src/schema/index.ts` |
| Public TS re-export | `packages/xlsx-preview/src/types.ts` |
| CI guard | `scripts/regen-schema.sh && git diff --exit-code packages/xlsx-preview/src/schema/` (see `.github/workflows/ci.yml`) |

Schema conventions:

- `#[serde(rename_all = "camelCase")]`
- `#[ts(optional)]` on `Option<T>`
- `#[ts(type = "number")]` on JS-safe `i64` EMU fields

## Manual visual-diff workflow

No browser needed — render straight to PNG via `skia-canvas`:

```bash
cargo build --release                            # rebuild xlcore + wasm if extractor changed
pnpm --filter @hewliyang/xlsx-preview run build:release   # TS + wasm
# or, if only TS / renderer changed:
pnpm --filter @hewliyang/xlsx-preview build

F=path/to/file.xlsx
node packages/xlsx-preview/dist/cli.js "$F" -o /tmp/ours.png --scale 2
hsx screenshot "$F" -o /tmp/hsx.png
__PI_IMAGE__ /tmp/ours.png /tmp/hsx.png
```

Range / single-sheet screenshot:

```bash
node packages/xlsx-preview/dist/cli.js "$F" -o /tmp/ours.png \
    --sheet Cover --range B3:E12 --scale 2
hsx screenshot "$F" "Cover!B3:E12" -o /tmp/hsx.png
```

Every sheet at once:

```bash
node packages/xlsx-preview/dist/cli.js "$F" -o /tmp/previews/{index}-{sheet}.png --all
```

Don't use `pnpm exec xlsx-preview` / bare `xlsx-preview` — they resolve to
the global install and silently bypass your local build. See
`docs/TESTING.md#footguns`.
