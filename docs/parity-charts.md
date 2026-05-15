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
| chartEx (`cx:` waterfall/funnel/treemap/…) | ❌ empty bbox | ✅ | hsx |

## Fixture corpus

| Path | Contents |
| --- | --- |
| `tests/fixtures/charts/chart-*.xlsx` | named minimal chart regression fixtures |
| `tests/fixtures/charts/chart-*.ours.png` | checked-in xlsx-preview renders |
| `tests/fixtures/charts/chart-*.hsx.png` | checked-in hsx renders |
| `tests/fixtures/charts/chart-*.layout.json` | extracted layout snapshots for debugging |
| `tests/fixtures/charts/build-chart-regressions.sh` | rebuilds the minimal chart regression fixtures |

Example fixtures:

| Sheet | Chart | Range | Notes |
| --- | --- | --- | --- |
| `Sheet1` | Waterfall via no-fill stack | `F2:M18` | `chart-waterfall-nofill-stacked.xlsx`: percent-stacked column using an invisible spacer series |
| `Sheet1` | Stacked color modifiers | `F2:M18` | `chart-stacked-color-modifiers.xlsx`: two stacked column series with distinct fills |
| `Sheet1` | Combo with secondary axis | `F2:N19` | `chart-combo-secondary-axis.xlsx`: clustered column + line on secondary y-axis |
| `Sheet1` | Dual-axis lines | `F2:N19` | `chart-dual-axis-lines.xlsx`: two line series on primary/secondary y-axes |

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
| `c:` | `radarChart` | ❌ | not wired |
| `c:` | `stockChart` | ❌ | not wired |
| `c:` | `surfaceChart` / `surface3DChart` | ❌ | not wired |
| `c:` | `ofPieChart` | ❌ | not wired |
| `c:` | `bar3DChart` / `line3DChart` / `area3DChart` / `pie3DChart` | ❌ | should initially dispatch to 2D painters |
| `cx:` | `waterfall` | ❌ | chartEx unsupported |
| `cx:` | `funnel` | ❌ | chartEx unsupported |
| `cx:` | `treemap` | ❌ | chartEx unsupported |
| `cx:` | `sunburst` | ❌ | chartEx unsupported |
| `cx:` | `histogram` / `pareto` | ❌ | chartEx unsupported |
| `cx:` | `boxWhisker` | ❌ | chartEx unsupported |
| `cx:` | `regionMap` | ❌ | chartEx unsupported |

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

1. 3D legacy chart variants: dispatch to 2D painters.
2. `radarChart`.
3. `ofPieChart` as plain pie first.
4. `stockChart`.
5. `cx:` waterfall.
6. `cx:` funnel / treemap / sunburst / histogram / boxWhisker.
7. `surfaceChart` / `regionMap`.

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
