# Chart axis fidelity backlog

Charts auto-scale axes wrong vs Excel/hsx. No explicit `<c:min>/<c:max>` in this
workbook's `<c:valAx>` (only `<c:orientation val="minMax"/>`), so Excel
auto-computes bounds. Reference fixture: `~/Downloads/studywhat (5).xlsx`
(Trends line charts, Summary/Dashboard bar charts).

Verify build: `pnpm --filter @hewliyang/xlsx-preview test` (runs typecheck path via
build + vitest). Render compare:
`node packages/xlsx-preview/dist/cli.js "<file>" --sheet <Name> -o /tmp/x.png`.

## Todo

### 2. Chart font auto-scaling to plot-area size
- Font sizes are fixed px constants (`AXIS_FONT_SIZE=10`, title 14, legend/axis-title
  11) duplicated across `chart.ts`, `chartAdvanced.ts`, `chart3d.ts`, `chartEx.ts`,
  `chartCombo.ts`, `chartStock.ts`, `chartExStats.ts`. They don't scale with chart
  size, so large charts get tiny text (Excel auto-scales font with plot area).
- Scale fonts proportionally to chart dimensions (Excel uses ~min(width,height)
  based scaling, base ~10pt at a reference size). Keep explicit `<a:rPr sz=...>`
  overrides authoritative. Centralize the constant if practical.

### 3. Auto-rotate category labels when they collide
- `drawCategoryAxis` (`src/chartUtils.ts:353-368`) only rotates when explicit `rot`
  is in XML; otherwise draws horizontal and *drops* colliding labels
  (`if (left < lastRight + minGapPx) continue;`). Excel auto-rotates (45°/90°) to
  fit all labels. After item 2 grows fonts, the 17 year labels will collide.
- When no explicit rot and horizontal labels overflow, auto-rotate (try 45°, then
  90°) instead of dropping. Reuse `drawRotatedLabel` / `rotatedLabelBandHeight`.

## Shipped

- Data-driven zero-clamp (Excel 5/6 rule) in `resolveAxisRange`; dropped per-call-site `zeroClamp` boolean.
