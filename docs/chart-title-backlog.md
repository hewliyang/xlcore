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

_(empty)_

## Shipped

### 2. Axis tick-label font (`c:catAx/c:txPr`, `c:valAx/c:txPr`)

Added `label_font: Option<ChartTextStyle>` to `ChartAxisPatch`, merged into the
axis `c:txPr` `a:defRPr` alongside `label_rotation` (one shared txPr), reads
back, render model surfaces `catAxisLabelFont`/`valAxisLabelFont`, and the
renderer honors them for tick labels via `resolveAxisLabelFont` (sibling of
`resolveTitleFont`). Axis pairs already alias `txPr = label_rotation`; `--check`
stays green.

### 1. Chart & axis title box fill + border (`c:title/c:spPr`)

Modeled `title_fill`/`title_border` on `ChartPatch`/`ChartUpdate`/`ChartInfo`
(chart title) and `ChartAxisPatch` (axis title), reusing `ChartLine`. Emits
`c:title/c:spPr` solidFill/noFill + `a:ln`, reads back, render model surfaces
`titleFill`/`titleBorder` + axis variants, and the preview draws a filled/stroked
box behind each title. Title pair now aliases `spPr = title_fill`.
