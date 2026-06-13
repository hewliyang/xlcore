# Chart parity

Tracks the **draw path** (xlsx-preview renderer) against `hsx` (SpreadJS) and
Excel desktop. The **write path** (`xlcore-api` authoring) is now ahead of the
renderer — see `docs/triage-api-expressiveness.md` for what round-trips but
isn't drawn yet. This doc is the renderer's backlog.

Corpus: small public fixtures in `tests/fixtures/charts/`. Private cases are
minimal repros with synthetic data. `hsx` is not always ground truth for
charts — side with Excel desktop/spec where they disagree.

## Scoreboard (xlsx-preview vs hsx)

| Area | xlsx-preview | hsx | Winner |
| --- | --- | --- | --- |
| Combo / dual axes, secondary-axis floor | ✅ | ❌ zero-clamps | xlsx-preview |
| Stacked / percentStacked col, theme modifiers | ✅ | 🟡 | xlsx-preview |
| Waterfall via no-fill stack | ✅ | ✅ | tie |
| Missing chart title | ✅ omits | ❌ paints placeholder | xlsx-preview |
| Anchor clipping (`xdr:to`) | ✅ | ❌ overflows | xlsx-preview |
| `stockChart` HLC/OHLC | ✅ | ❌ empty plot | xlsx-preview |
| Legend position / marker shapes | ✅ | 🟡 clips overflow | xlsx-preview |
| Bar widths (`gapWidth`/`overlap`) | ✅ | 🟡 often wider | xlsx-preview |
| `dispUnits` tick scaling + caption | ✅ | ❌ dropped | xlsx-preview |
| Axis titles, gridline toggle, line/marker styling | ✅ | ✅ | tie |
| Negative-range axes | 🟡 | 🟡 | tie |
| `<c:multiLvlStrRef>` hierarchical cat axis | ✅ | ✅ | tie |
| chartEx waterfall | ✅ | ❌ empty bbox | xlsx-preview |
| chartEx funnel/treemap/sunburst | ✅ | ✅ | tie |
| chartEx histogram/pareto/boxWhisker | ✅ | ❌ degenerate | xlsx-preview |
| chartEx regionMap | ✅ choropleth | ❌ column fallback | xlsx-preview |
| Series trendlines (`c:trendline`) | ✅ | ✅ | tie |
| Series error bars (`c:errBars`) | ✅ | ✅ | tie |
| Axis tick-label rotation (`txPr bodyPr rot`) | ✅ | ✅ | tie |
| Data table (`c:dTable`) | ❌ | ✅ grid below plot | xlsx-preview |
| Per-point gradient / pattern fills (`c:dPt` `a:gradFill`/`a:pattFill`) | ✅ | 🟡 gradient→solid, pattern→gray | xlsx-preview |
| Plot-area / legend `spPr` fills + borders + `txPr` font | ✅ | ✅ | tie |
| ofPie satellite split (`pieOfPie`/`barOfPie`) | ✅ | ❌ plain pie | xlsx-preview |
| Manual layout (`c:layout/c:manualLayout`) plot area/legend/title | ✅ | ✅ | tie |

## Chart-type coverage

| Namespace | Type | Status | Notes |
| --- | --- | --- | --- |
| `c:` | `barChart` | ✅ | clustered/stacked/percentStacked; `gapWidth`+`overlap` |
| `c:` | `lineChart` | ✅ | standard/stacked/percentStacked; marker suppression |
| `c:` | `pieChart` / `doughnutChart` | ✅ | per-slice colors; doughnut hole |
| `c:` | `areaChart` | ✅ | standard/stacked/percentStacked |
| `c:` | `scatterChart` | ✅ | marker/line/lineMarker/smooth |
| `c:` | `bubbleChart` | ✅ | `bubbleScale`, `sizeRepresents` |
| `c:` | `radarChart` | ✅ | `radarStyle` standard/marker/filled |
| `c:` | `stockChart` | ✅ | HLC/OHLC; hi-low marks, up/down bars |
| `c:` | `bar3D`/`column3D` | ✅ | pseudo-3D (oblique) boxes + floor/back/side walls; `view3D` rotX/rotY + `depthPercent`/`gapDepth` size the depth |
| `c:` | `line3D`/`area3D`/`pie3D` | 🟡 | drawn flat via 2D painters; perspective/depth dropped |
| `c:` | `ofPieChart` | ✅ | pieOfPie/barOfPie satellite split; `splitType`/`splitPos`/`secondPieSize`/`serLines` |
| `c:` | `surfaceChart`/`surface3DChart` | ✅ | pseudo-3D mesh over (category × series) grid; oblique projection + floor/back/side walls; filled height-band color ramp or `c:wireframe` grid lines |
| `cx:` | `waterfall` | ✅ | `drawChartEx` waterfall painter |
| `cx:` | `funnel`/`treemap`/`sunburst` | ✅ | `chartEx.ts` painters |
| `cx:` | `histogram`/`pareto`/`boxWhisker` | ✅ | `chartExStats.ts` painters |
| `cx:` | `regionMap` | ✅ | `chartExRegionMap.ts`; Natural Earth 110m |

## Renderer backlog

### 3D rendering (write path ships these, renderer ignores)

1. **bar3D/column3D** now drawn as pseudo-3D (oblique-projection) boxes
   (front + lighter top + darker side faces, painter-sorted) with a floor +
   back wall + side wall, consuming `c:view3D` (rotX/rotY, `depthPercent`) +
   `gapDepth` to size the depth and `c:floor`/`c:sideWall`/`c:backWall` spPr
   fills (`chart3d.ts`). Approximation only — no true perspective.
2. **True 3D geometry** for `line3D`/`area3D`/`pie3D` — still flattened to the
   2D painter; `view3D`/walls dropped for those kinds.
3. **bar3D `c:shape`** (box/cone/cylinder/pyramid) — only box rendered.
4. **`surfaceChart` / `surface3DChart`** — done: `drawSurfaceChart`
   (`chart3d.ts`) builds a z-grid (rows = series, cols = categories,
   z = value), reuses the bar3D oblique projection + floor/back/side walls,
   then painter-sorts grid quads back-to-front. Filled mode colors each quad
   by a height band (blue→red HSL ramp across N bands); `c:wireframe` strokes
   the grid lines instead. Value (z) axis with ticks + gridlines on the back
   wall.

### Open rendering nits

- Rotated-along-axis `<c:dispUnitsLbl>` placement.
- Negative-range category labels at value-axis crossing (`tickLblPos="nextTo"`).
- Stacked bars crossing zero: polygon-aware overlap handling.
- Area chart gaps for missing points.
- Negative-range cat-axis band drift (Bug #13 remainder).
- Live recalc for formula-only chart source cells; blocked on `xlcore-engine`.

## Open / partial bugs

| # | Issue | Side | Status | Notes |
| --- | --- | --- | --- | --- |
| 4 | Literal `Chart Title` placeholder leaks | hsx | open | xlsx-preview omits absent titles |
| 5 | Chart bbox overflows anchor | hsx | open | xlsx-preview honors `xdr:to` |
| 9 | Secondary axis auto floor zero-clamped | hsx | open | xlsx-preview is data-driven |
| 13 | Negative-range y-axes rough | both | partial | zero baseline fixed; cat-axis band drift remains |

Resolved bugs (#1–3, 6–8, 10–12, 14–27) are in git history; the renderer
modules above (`chart.ts`, `chartCombo.ts`, `chartStock.ts`, `chartAdvanced.ts`,
`chartEx.ts`, `chartExStats.ts`, `chartExRegionMap.ts`) carry the current logic.

## Fixture corpus

| Path | Contents |
| --- | --- |
| `tests/fixtures/charts/chart-*.xlsx` | named minimal regression fixtures |
| `tests/fixtures/charts/chart-*.ours.png` | checked-in xlsx-preview renders |
| `tests/fixtures/charts/chart-*.hsx.png` | checked-in hsx renders |
| `tests/fixtures/charts/chart-*.layout.json` | extracted layout snapshots |
| `tests/fixtures/charts/build-chart-regressions.sh` | rebuilds minimal fixtures |
| `tests/fixtures/charts/build-chartex.sh` | rebuilds SpreadJS-authored chartEx fixtures |

chartEx fixtures need an Office-grade authoring path — the XML uses opaque
`_xlchart.vN.X` definedName aliases (resolved via `workbook.xml` hidden
`<definedName>` entries) and pulls colors from a chartStyle/colorStyle pair.
Authoring paths:

- **Excel desktop** — waterfall, histogram, pareto, boxWhisker, regionMap.
- **SpreadJS via `hsx eval`** (`build-chartex.sh`) — funnel, treemap, sunburst
  (the layoutIds where SpreadJS round-trips cleanly). `paretoLine`/`boxWhisker`/
  histogram (`clusteredColumn`)/`regionMap` have known SpreadJS export gaps
  (missing `<cx:axis>`, no auto-binning, render-as-cluster) — see the rationale
  block in `build-chartex.sh`.

## Reproduction

```bash
F=tests/fixtures/charts/chart-combo-secondary-axis.xlsx
pnpm --filter @hewliyang/xlsx-preview build:ts
node packages/xlsx-preview/dist/cli.js "$PWD/$F" -o "$PWD/${F%.xlsx}.ours.png" --sheet Sheet1 --range F2:N19 --scale 2
hsx --timeout 60 screenshot "$F" 'Sheet1!F2:N19' -o "${F%.xlsx}.hsx.png"
```
