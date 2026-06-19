# Chart axis fidelity backlog

Charts auto-scale axes wrong vs Excel/hsx. No explicit `<c:min>/<c:max>` in this
workbook's `<c:valAx>` (only `<c:orientation val="minMax"/>`), so Excel
auto-computes bounds. Reference fixture: `~/Downloads/studywhat (5).xlsx`
(Trends line charts, Summary/Dashboard bar charts).

Verify build: `pnpm --filter @hewliyang/xlsx-preview test` (runs typecheck path via
build + vitest). Render compare:
`node packages/xlsx-preview/dist/cli.js "<file>" --sheet <Name> -o /tmp/x.png`.

## Todo

## Shipped

- Auto-rotate category labels (-45°, escalate to -90°) when horizontal labels overflow inner width; shared `resolveCatAxisRotation` used by `drawAxisFrame` (band reservation) and `drawCategoryAxis` (render all, none dropped); explicit XML `rot` stays authoritative.

- Recalibrated `chartFontScale` reference 360->200 and max 2->2.2 so typical embedded charts (~300px min dim) get ~1.5x bump matching Excel weight.

- Chart fonts auto-scale with plot-area size via shared `chartFontScale`/`applyChartFontScale` in `chartUtils.ts` (exported `let` sizes consumed by all renderers); explicit `<a:rPr sz=...>` overrides stay authoritative.
- Data-driven zero-clamp (Excel 5/6 rule) in `resolveAxisRange`; dropped per-call-site `zeroClamp` boolean.
- `formatAxisValue` ignores quoted `"..."` literals (e.g. `0.0"%"`) when detecting the `%`/`$`/comma operators; emits them as literal suffix.
