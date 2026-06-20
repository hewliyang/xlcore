# Chart text styling backlog

Follow-ups to the chart title/axis-title font work (commit 4c9895c). Each item
must round-trip (author -> save -> reopen) AND render in xlsx-preview, mirroring
the `title_font` implementation exactly.

## Workflow (read before starting)

- Template commit: `git show 4c9895c` (title/axis-title font, end to end). Also
  look at how the legend models `fill`/`border` (`ChartLegend`, `ChartPlotArea`)
  and `ChartLine` for borders.
- Layers to touch per item (same as the template):
  1. `crates/xlcore-types/src/charts.rs` — DTO fields on `ChartPatch`,
     `ChartUpdate`, `ChartInfo`, and/or `ChartAxisPatch`.
  2. `crates/xlcore-api/src/charts/write.rs` — emit OOXML.
  3. `crates/xlcore-api/src/charts/mod.rs` — update path + `ChartInfo`/`ChartPatch` synth.
  4. `crates/xlcore-api/src/charts/read.rs` — read back.
  5. `crates/xlcore-export/src/schema/charts.rs` — render-model `Chart` fields.
  6. `crates/xlcore-export/src/{chart_colors,charts_legacy,charts_ex}.rs` — importer.
  7. `packages/xlsx-preview/src/{chartUtils,chart}.ts` — renderer.
- Regenerate TS bindings: `cargo test -p xlcore-export -p xlcore-types --features typescript export_bindings`.
  Then revert churn: `git checkout -- packages/xlsx-preview/src/schema packages/xlsx-preview/src/api-schema`
  is WRONG (loses real diffs); instead run
  `npx @biomejs/biome format --write src/schema src/api-schema` then
  `git checkout` only the files whose diff is pure formatting (biome canonicalizes
  unchanged files back to identical). Confirm `git status` shows ONLY the files
  you intended to change.
- ts-rs collapses formatting on ALL ~70 schema files; the biome pass above is how
  the template kept the diff to just the 6 intended files.
- Add a Rust round-trip test mirroring `chart_title_and_axis_title_fonts_roundtrip`
  in `crates/xlcore-api/src/tests/charts.rs`.
- Add `ChartPatch` test literals need the new field too: tests construct
  `ChartPatch` fully (no `..Default`), so add the field there (see how the
  template added `title_font: None`). `ChartUpdate`/`ChartInfo`/`ChartAxisPatch`
  use `..Default::default()` in tests, so they don't need edits.
- Update the Title/axis manifest pair in `scripts/schema_coverage.toml` so
  `schema_diff.py --check` stays green (the spPr/txPr element becomes covered via
  an alias instead of excluded).
- Never write comments/docstrings (repo rule). Changelog entries terse.

## Checks (run all, must pass)

```bash
cargo test -p xlcore-api charts 2>&1 | grep "test result"
cargo test -p xlcore-export 2>&1 | grep -c "test result: ok"
cd packages/xlsx-preview && pnpm build:wasm && pnpm build:ts \
  && npx tsc -p tsconfig.build.json --noEmit \
  && npx vitest run chart && pnpm run check:schema && pnpm run check:api
```

E2E: author a chart with the new property, `wb.save()` to xlsx, render with
`node dist/cli.js <xlsx> --output /tmp/out.png --scale 2`, and eyeball it.

## Backlog

### 1. Chart & axis title box fill + border (`c:title/c:spPr`)

The title pair currently marks `spPr` excluded. Model the title box background
fill and border, mirroring the legend's `fill`/`border`.

- DTO: add `title_fill: Option<String>` (hex or "none") and
  `title_border: Option<ChartLine>` to `ChartPatch`, `ChartUpdate`, `ChartInfo`
  (chart title) and `ChartAxisPatch` (axis title). Reuse `ChartLine` (see legend
  `border`). `ChartTextStyle`/`ChartLine` patterns already exist.
- Write: emit `c:title/c:spPr` with `a:solidFill`/`a:noFill` + `a:ln` (see how
  legend spPr / plot-area border is built in write.rs).
- Render model + renderer: surface as `Chart.titleFill`/`titleBorder` etc and
  draw a filled/stroked box behind the title text (see `drawStyleBox` usage for
  plot area / legend in `chart.ts`).
- Manifest: in the `Title` pair, replace `excluded = ["spPr"]` with
  `aliases = { ..., spPr = "title_fill" }` (keep tx/layout/txPr aliases). Verify
  `python3 scripts/schema_diff.py Title ChartUpdate` shows spPr covered, and
  `--audit`/`--check` stay green.

### 2. Axis tick-label font (`c:catAx/c:txPr`, `c:valAx/c:txPr`)

Axes currently model only label rotation from their own `c:txPr`. Add tick-label
font control (size/bold/italic/color/typeface).

- DTO: add `label_font: Option<ChartTextStyle>` to `ChartAxisPatch`.
- Write: emit the axis `c:txPr` `a:defRPr` with font props, alongside the
  existing rotation handling (don't clobber rotation; merge both into one txPr).
- Read: parse axis `c:txPr` font.
- Render model + renderer: surface per-axis label font; the renderer currently
  uses hardcoded `AXIS_FONT_SIZE`/`AXIS_LABEL_COLOR` for tick labels — honor the
  custom font (reuse the `resolveTitleFont` helper or add a sibling).
- Manifest: the axis pairs alias `txPr = "label_rotation"`; that already marks
  txPr covered, so no change strictly needed, but confirm `--check` green.

## Shipped

(none yet)
