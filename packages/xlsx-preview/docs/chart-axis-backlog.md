# Chart axis fidelity backlog

Charts auto-scale axes wrong vs Excel/hsx. No explicit `<c:min>/<c:max>` in this
workbook's `<c:valAx>` (only `<c:orientation val="minMax"/>`), so Excel
auto-computes bounds. Reference fixture: `~/Downloads/studywhat (5).xlsx`
(Trends line charts, Summary/Dashboard bar charts).

Verify build: `pnpm --filter @hewliyang/xlsx-preview test` (runs typecheck path via
build + vitest). Render compare:
`node packages/xlsx-preview/dist/cli.js "<file>" --sheet <Name> -o /tmp/x.png`.

## Todo

### 3. Auto-rotate category labels when they collide
- `drawCategoryAxis` (`src/chartUtils.ts:353-368`) only rotates when explicit `rot`
  is in XML; otherwise draws horizontal and *drops* colliding labels
  (`if (left < lastRight + minGapPx) continue;`). Excel auto-rotates (45°/90°) to
  fit all labels. After item 2 grows fonts, the 17 year labels will collide.
- When no explicit rot and horizontal labels overflow, auto-rotate (try 45°, then
  90°) instead of dropping. Reuse `drawRotatedLabel` / `rotatedLabelBandHeight`.

## Shipped

- Recalibrated `chartFontScale` reference 360->200 and max 2->2.2 so typical embedded charts (~300px min dim) get ~1.5x bump matching Excel weight.

- Chart fonts auto-scale with plot-area size via shared `chartFontScale`/`applyChartFontScale` in `chartUtils.ts` (exported `let` sizes consumed by all renderers); explicit `<a:rPr sz=...>` overrides stay authoritative.
- Data-driven zero-clamp (Excel 5/6 rule) in `resolveAxisRange`; dropped per-call-site `zeroClamp` boolean.
- `formatAxisValue` ignores quoted `"..."` literals (e.g. `0.0"%"`) when detecting the `%`/`$`/comma operators; emits them as literal suffix.
