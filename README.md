# xlsx-preview

Rust extractor workspace + `@hewliyang/xlsx-preview` canvas renderer for an agent-/HITL-friendly Excel pipeline.
See [`plan-excel-rust-lib.md`](plan-excel-rust-lib.md) for the architecture and
multi-month roadmap.

Current scope: **`xlsx → WorkbookLayout JSON → canvas preview`**, end-to-end in
the browser via wasm or in Node via `skia-canvas`. Recalc is not wired yet
— formula `<v>` caches are emitted as-is. The IronCalc fork is the next big
milestone (planned under `crates/xlcore-engine/` + `crates/xlcore-bridge/`).

## layout

```
crates/
  xlcore-io/        ooxmlsdk facade (open/save, A1 helpers)
  xlcore-export/    extract WorkbookLayout (cells, styles, layout, charts) → JSON
  xlcore-wasm/      wasm-bindgen entry; in-browser extraction
  xlcore-cli/       `xlcore extract` and `xlcore preview` binaries
packages/xlsx-preview/          npm package source for `@hewliyang/xlsx-preview`;
                                canvas renderer for browser/React/Node
plan-excel-rust-lib.md
```

Future crates (per the plan):

```
crates/
  xlcore-engine/    ironcalc fork + missing functions (SUMPRODUCT, LET, ...)
  xlcore-bridge/    harvest cells → ironcalc, write recomputed <v> back
```

## try it

```bash
# 1. build the renderer bundle
pnpm install
pnpm build

# 2. build the rust workspace
cargo build --release

# 3. extract a layout JSON
./target/release/xlcore extract tests/fixtures/kitchensink/kitchensink.xlsx -o /tmp/kitchensink.json

# 4. emit a self-contained HTML preview (renderer + data inlined)
./target/release/xlcore preview tests/fixtures/kitchensink/kitchensink.xlsx -o /tmp/preview.html
open /tmp/preview.html
```

## test & validate

- `cargo test --workspace` for Rust unit tests + `ts-rs` schema export.
- Visual fidelity: render our preview, then `hsx screenshot` and the OAI
  Walnut server on the same fixture. Eyeball the three.
- Source-controlled fixture corpus under [`tests/fixtures/`](tests/fixtures/);
  see [`tests/fixtures/README.md`](tests/fixtures/README.md) for the table
  and how to add new ones.
- See [`TESTING.md`](TESTING.md) for the exact step-by-step (no script;
  manual is more reliable here) + the visual checklist.
- Feature-by-feature scoreboard against Excel/SpreadJS:
  [`PARITY.md`](PARITY.md).

## data flow

```
xlsx
  │  ooxmlsdk (xlcore-io)
  ▼
SpreadsheetDocument (full OOXML tree, charts/CF/etc preserved verbatim)
  │  walk sheets/rows/cells, resolve fonts/fills/borders/numFmts;
  │  extract drawings + charts; resolve chart range refs
  ▼  (xlcore-export)
WorkbookLayout JSON  ──►  @hewliyang/xlsx-preview canvas renderer  ──►  <canvas> / PNG
```

Three runtime paths share the same `render()` core:

- **In-browser end-to-end:** `createWorkbookPreviewerFromFile()` runs
  `xlcore-wasm` inside a Web Worker (see `packages/xlsx-preview/src/xlsxWorker.ts`),
  hands the JSON to the canvas renderer in the main thread. No server.
- **Node / CLI:** `renderToPng()` / `renderXlsxToPng()` from
  `@hewliyang/xlsx-preview` use `skia-canvas` against the same render
  pass; the `xlsx-preview` bin shells out to it.
- **Static HTML preview:** `xlcore preview <file.xlsx>` (Rust CLI) extracts
  server-side and inlines `{ layout JSON (gzip+base64), renderer bundle }`
  into one self-contained `.html` — useful when you want a portable artifact.


The JSON contract is generated from one source via
[`ts-rs`](https://github.com/Aleph-Alpha/ts-rs):
- Rust source of truth: [`crates/xlcore-export/src/schema.rs`](crates/xlcore-export/src/schema.rs)
- Generated TS (per type): `packages/xlsx-preview/src/schema/*.ts`
- Barrel re-export the renderer imports: [`packages/xlsx-preview/src/types.ts`](packages/xlsx-preview/src/types.ts)

Regenerate after any schema change:

```bash
cargo test --release -p xlcore-export export_bindings
```

See the [Schema sync](PARITY.md#schema-sync) section of PARITY.md for
the attributes (`#[serde(rename_all = "camelCase")]`, `#[ts(optional)]`,
`#[ts(type = "number")]`) that keep generated TS in lock-step with what
the extractor writes.

## v0 fidelity

[`PARITY.md`](PARITY.md) is the per-feature scoreboard and the canonical
source of truth. Quick summary:

Shipped end-to-end (extract → schema → render):

- Cells: shared / inline strings, rich-text runs, `\n` + `wrapText`, indent,
  text overflow into empties, custom widths/heights, hidden rows/cols,
  merged cells, freeze panes, gridline toggle, outline groups + gutter.
- Fonts: name + `<scheme>`/`<family>` resolution, size, bold, italic,
  color, super/subscript, strikethrough, all 4 underline variants
  (`single` / `double` / `singleAccounting` / `doubleAccounting`),
  text rotation (1–180° + stacked).
- Fills: pattern (all 18 hatches + solid), linear + path gradients with
  multi-stop positions, theme tints via proper OOXML HLS.
- Borders: all 14 styles incl. `slantDashDot`, around merges, diagonal
  up/down/X with style + color.
- Number formats: full ECMA-376 §18.8.30 built-ins + a real format-section
  evaluator (multi-section, `[Red]`/`[Color12]`, `[>0]` gates, `[$€-407]`
  currency tags, fractions via Stern–Brocot, engineering-shift scientific,
  `*x` width-aware fill padding).
- Theme: full `xl/theme/theme1.xml` parsing — all 5 color-choice variants
  (`srgbClr` / `sysClr` / `scrgbClr` / `hslClr` / 190-entry `prstClr`),
  lt1/dk1/lt2/dk2 index swap, full ECMA indexed palette.
- Conditional formatting: `colorScale` (2/3-stop), `cellIs` (all 8 ops),
  `dataBar` (gradient + mixed-sign), `iconSet` (3/4/5-stop, reverse,
  showIconOnly), `top10` / `aboveAverage` / `duplicateValues` / text /
  `timePeriod`, with `stopIfTrue` cross-kind masking. Only `expression`
  is blocked on the engine.
- Charts: column / bar (clustered + stacked), line, area (standard /
  stacked / percentStacked), pie + doughnut (with per-slice `<c:dPt>`
  colors), xy scatter (incl. `lineMarker` / `smoothMarker`), data
  labels, axis ticks with workbook number format, theme accent series
  colors, legend, range-formula resolution when `numCache` is empty.
- Drawings: raster images (base64 inline), x14 sparklines (line +
  column + win/loss, group-scoped axes).
- Tables: `TableStyleMedium*` chrome (header band, banded rows, totals)
  + filter-arrow glyphs.
- Pivot tables: cheap path (materialized cells from `<sheetData>` with
  pivot filter-arrow chevrons; active-tab honored).
- Annotations: hyperlinks (https/mailto/file/in-workbook, cursor +
  click + new-tab) and threaded-comment markers with hover popover.
- Vector-crisp zoom (re-render on `devicePixelRatio` + `±` zoom).
- Viewport-clipped row/col walks in `grid.ts` so big workbooks only
  paint what's on screen.
- Schema kept in sync via `ts-rs`-generated `packages/xlsx-preview/src/schema/*.ts`.

Known gaps:

- **Formula recalc.** `#SPILL!` and any cell whose source `<v>` is empty
  renders blank. Unblocks once the IronCalc fork (`xlcore-engine` +
  `xlcore-bridge`) lands.
- Drawings other than charts / images / sparklines (shapes, SmartArt).
- Slicers, timelines; pivot interactivity (expand/collapse, filter).
- `expression` CF rules — need the engine.
- Selection / active-cell rendering.
- Combo charts, secondary-axis, bubble.

## next steps

The near-term quick-wins from earlier roadmap revisions (CF rules,
tables, number-format compiler, line/pie/area/scatter, comments +
hyperlinks, pivot cheap-path, wasm, skia-canvas Node adapter) are all shipped
— see PARITY.md for per-feature status. Remaining work:

1. **`xlcore-engine` + `xlcore-bridge`** — fork IronCalc, port the missing
   functions (`SUMPRODUCT`, `LET`, `LAMBDA`, `FILTER`, `SORT`, `UNIQUE`,
   `SEQUENCE`, `XMATCH`, dynamic-array stack functions). Unblocks live
   recalc, `#SPILL!`, and `expression` CF rules.
2. **Selection / active-cell rendering** for the HITL preview.
3. **x14 extension parsing** — would let users override dataBar
   gradient/negativeColor and iconSet thresholds (currently we fall
   back on observed defaults; see `tests/fixtures/cf/TRIAGE.md`).
4. **Combo charts + secondary axis**, **bubble sizing**, **filtered-row
   hiding** via `autoFilter`.
