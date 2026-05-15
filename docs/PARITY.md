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
| Time period | ✅ | ✅ | 10 named periods; no snapshot fixture |
| Stop-if-true | ✅ | ✅ | cross-kind masking |

## Tables, pivots, charts, drawings

| Feature | Extract | Render | Notes |
| --- | --- | --- | --- |
| Tables / ListObjects | ✅ | 🟡 | header, banding, filter glyphs, totals; no filtering |
| Pivot tables | 🟡 | 🟡 | static materialized cells + filter chevrons; no refresh/interactivity |
| Slicers / timelines | ❌ | ❌ | |
| Data validation | ❌ | ❌ | dropdowns are compositor/interactivity work |
| AutoFilter hidden rows | ❌ | ❌ | filtered rows should hide |
| Chart: column/bar | ✅ | ✅ | clustered/stacked/percent, ticks, legend, theme colors |
| Chart: line | ✅ | ✅ | stacked/percent, outline colors, marker suppression |
| Chart: pie/doughnut | ✅ | ✅ | slices, hole, per-slice colors; open leader lines |
| Chart: scatter | ✅ | ✅ | marker/line/lineMarker/smooth/smoothMarker |
| Chart: bubble | ✅ | ✅ | size scaling, area/width modes |
| Chart: area | ✅ | ✅ | standard/stacked/percent |
| Chart: combo/secondary axis | ✅ | ✅ | multi-group plotAreas, dual y-axes, axis titles, data labels, `dispUnits` |
| Chart: data labels | ✅ | ✅ | chart/series/per-point overrides |
| ChartEx (`cx:`) | ❌ | ❌ | waterfall/funnel/treemap/sunburst/histogram/boxWhisker/regionMap; see `docs/parity-charts.md` |
| Sparklines | ✅ | ✅ | line/column/win-loss; open prefix robustness + marker shape |
| Raster images | ✅ | ✅ | base64 inline |
| Cropped/rotated images | 🟡 | 🟡 | transforms ignored |
| Shapes | ❌ | ❌ | placeholder grey box |
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
| Formula recalc | ❌ | IronCalc fork planned; see `plan-excel-rust-lib.md` |
| Modern/dynamic functions | ❌ | `SUMPRODUCT`, `LET`, `LAMBDA`, `FILTER`, `SORT`, `UNIQUE`, `SEQUENCE`, arrays |

## Major open work

| Area | Status |
| --- | --- |
| Formula engine / recalc | ❌ |
| Filtered-row hiding | ❌ |
| Split panes | ❌ |
| RTL sheets | ❌ |
| Data validation UI | ❌ |
| Slicers / timelines | ❌ |
| Shapes / SmartArt | ❌ |
| chartEx support | ❌ |
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
| `tests/fixtures/**/*.layout.json` | planned/insta data snapshots |

Representative fixtures:

| Area | Fixture |
| --- | --- |
| Text | `text/indent.xlsx`, `text/rotation.xlsx`, `text/fontfamily.xlsx`, `text/vertalign.xlsx`, `text/underline.xlsx` |
| Number formats | `numfmt/date-time-formats.xlsx`, `numfmt/custom-section-conditions.xlsx`, `numfmt/fraction-and-scientific.xlsx` |
| Borders/fills | `borders/every-style.xlsx`, `borders/diagonal.xlsx`, `fills/patterns.xlsx`, `fills/gradients.xlsx` |
| Conditional formatting | `cf/cell-is.xlsx`, `cf/data-bar.xlsx`, `cf/icon-set.xlsx`, `cf/cf-non-recalc.xlsx`, `cf/stop-if-true.xlsx` |
| Tables/pivots | `tables/table-medium.xlsx`, `pivot/pivot-simple.xlsx` |
| Charts | `charts/line-pie-area-scatter.xlsx`, `charts/data-labels.xlsx`, `charts/bubble.xlsx`, `charts/chart-*.xlsx` |
| Layout | `outline/outline-groups.xlsx`, freeze-pane fixtures |
| Annotations | `annotations/hyperlinks-comments.xlsx` |
| Sheets | `sheets/hidden-and-tabcolor.xlsx` |
| Sparklines | `sparklines/sparklines.xlsx` |
| Styles/themes | `styles/named-inheritance.xlsx`, `themes/custom-theme-accent.xlsx` |

## Schema sync

| Task | Command / file |
| --- | --- |
| Regenerate TS bindings | `cargo test --release -p xlcore-export export_bindings` |
| Rust schema | `crates/xlcore-export/src/schema.rs` |
| TS generated schema | `packages/xlsx-preview/src/schema/*.ts` |
| TS barrel | `packages/xlsx-preview/src/schema/index.ts` |
| Public TS re-export | `packages/xlsx-preview/src/types.ts` |
| Planned CI guard | run export, then `git diff --exit-code packages/xlsx-preview/src/schema/` |

Schema conventions:

- `#[serde(rename_all = "camelCase")]`
- `#[ts(optional)]` on `Option<T>`
- `#[ts(type = "number")]` on JS-safe `i64` EMU fields

## Manual visual-diff workflow

```bash
pnpm build
cargo build --release

./target/release/xlcore preview path/to/file.xlsx -o /tmp/preview.html
uv run browser-harness <<'PY'
goto("file:///tmp/preview.html")
wait_for_load()
import time; time.sleep(2)
screenshot("/tmp/ours.png")
PY

hsx screenshot path/to/file.xlsx -o /tmp/hsx.png
__PI_IMAGE__ /tmp/ours.png /tmp/hsx.png
```

Range screenshot:

```bash
hsx screenshot file.xlsx "Sheet!A1:M30" -o out.png
```

If browser-harness breaks:

```bash
cd ~/Developer/browser-harness
uv run python -c "from admin import restart_daemon; restart_daemon()"
```
