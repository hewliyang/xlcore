# excel-spike (xlcore)

Rust crate(s) + TS canvas renderer for an agent-/HITL-friendly Excel pipeline.
See [`plan-excel-rust-lib.md`](plan-excel-rust-lib.md) for the architecture and
multi-month roadmap.

Current scope: **`xlsx → WorkbookLayout JSON → browser-canvas preview`**, with
full fidelity on cells/styles/borders/merges/CF/charts (bar/column). Recalc is
not wired yet — formula `<v>` caches are emitted as-is. The IronCalc fork lands
under `crates/xlcore-engine/`.

## layout

```
crates/
  xlcore-io/        ooxmlsdk facade (open/save, A1 helpers)
  xlcore-export/    extract WorkbookLayout (cells, styles, layout, charts) → JSON
  xlcore-cli/       `xlcore extract` and `xlcore preview` binaries
render-ts/          canvas renderer; runs in browser + (eventually) node-canvas
plan-excel-rust-lib.md
```

Future crates (per the plan):

```
crates/
  xlcore-engine/    ironcalc fork + missing functions (SUMPRODUCT, LET, ...)
  xlcore-bridge/    harvest cells → ironcalc, write recomputed <v> back
  xlcore-wasm/      wasm-bindgen entry for browser
```

## try it

```bash
# 1. build the renderer bundle
cd render-ts && bun install && bun run build && cd ..

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
  manual is more reliable here), the visual checklist, and open work on
  snapshot/pixel-diff CI.
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
WorkbookLayout JSON  ──►  render-ts canvas renderer  ──►  <canvas> / PNG
```

The JSON contract is generated from one source via
[`ts-rs`](https://github.com/Aleph-Alpha/ts-rs):
- Rust source of truth: [`crates/xlcore-export/src/schema.rs`](crates/xlcore-export/src/schema.rs)
- Generated TS (per type): `render-ts/src/schema/*.ts`
- Barrel re-export the renderer imports: [`render-ts/src/types.ts`](render-ts/src/types.ts)

Regenerate after any schema change:

```bash
cargo test --release -p xlcore-export export_bindings
```

See the [Schema sync](PARITY.md#schema-sync) section of PARITY.md for
the attributes (`#[serde(rename_all = "camelCase")]`, `#[ts(optional)]`,
`#[ts(type = "number")]`) that keep generated TS in lock-step with what
the extractor writes.

## v0 fidelity

Implemented:

- Shared strings, inline strings, **rich-text runs** (per-run
  bold/italic/color/font/size + `\n` line breaks), custom column widths,
  custom row heights, hidden rows/cols.
- Fonts (name/size/bold/italic/color), pattern-solid fills, gradient
  fills (linear), per-side borders + style (thin/medium/thick/dashed/
  dotted/double/etc.), borders around merged ranges.
- Alignment (h/v), `wrapText` (word-wrap respects per-run font metrics),
  indent (extracted), text overflow into adjacent empty cells.
- Merged cells, freeze panes (subtle indicator), grid-line toggle.
- Number formats: built-in IDs + custom code (currency, percent,
  grouped, decimals; partial date/time).
- **Theme XML**: parsed from `xl/theme/theme1.xml`; cell + chart-series
  accents resolve against the workbook palette with the spreadsheet's
  lt1/dk1, lt2/dk2 index swap handled. Falls back to Office 2007+
  defaults when missing or for `prstClr`/`hslClr`/`scrgbClr` slots.
- Indexed-color palette (small built-in subset).
- **Conditional formatting**: `colorScale` (2/3-stop,
  min/max/percent/percentile/num CFVOs).
- **Charts**: column + bar, clustered + stacked. Title (RichText or
  StrRef), axis ticks with workbook number format, **workbook-theme**
  accent series colors, bottom/top/left/right legend, range-formula
  resolution when `numCache` is empty.
- Raster images (base64 inline).
- **Vector-crisp zoom**: re-renders on `devicePixelRatio` change
  (browser Cmd+/-) and via `±` app-level zoom buttons; never
  bitmap-upscales.
- Schema kept in sync via `ts-rs`-generated `render-ts/src/schema/*.ts`.

Not yet (engine preserves XML on round-trip; renderer skips). See
[`PARITY.md`](PARITY.md) for the full per-feature scoreboard:

- Line / pie / area / scatter / doughnut charts (render as labelled
  placeholder boxes; data is still extracted).
- Drawings other than charts + images (shapes, SmartArt).
- Pivot tables, slicers, timelines.
- Conditional formats other than color scale (`dataBar`, `iconSet`,
  `cellIs`, `expression`, `top10`, etc.).
- Comments, hyperlinks.
- Tables (`<table>` ListObjects) — banded rows + filter arrows.
- Theme color tints via proper HLS (we use a linear approximation).
- Rotated text, vertical text, strikethrough, underline rendering.
- Recalc — `#SPILL!` and cells whose source `<v>` is empty render
  blank. Unblocks once the IronCalc fork lands.

## next steps

In rough priority order. PARITY.md's ["Quick wins"](PARITY.md#quick-wins-next-in-priority-order)
section has the full ranked list with effort estimates.

1. **CF beyond color scales** — `cellIs` + `dataBar` + `iconSet`. No
   recalc needed; `expression` waits on the engine.
2. **Tables** (`<table>` ListObjects) — banded rows, filter-arrow glyph,
   optional totals row. ~80 LOC each side.
3. **Number-format compiler** — real format-section evaluator (`[Red]`,
   `[>0]`, AM/PM, fractions, scientific).
4. **Charts: line / pie / scatter / area** — each is one new drawer in
   `render-ts/src/chart.ts`.
5. **Comments + hyperlinks** — cheap and visible (red-triangle marker,
   blue+underline). Both have OOXML parts not yet in `WorkbookLayout`.
6. **`xlcore-engine`** — fork IronCalc, port `SUMPRODUCT` first.
   Unblocks live recalc, `#SPILL!`, and `expression` CF rules.
7. **`xlcore-wasm`** — wasm-bindgen entry for end-to-end in-browser
   extraction (currently `xlcore preview` re-runs the Rust extractor
   server-side).
8. ~~**node-canvas adapter** so the same TS renderer produces server-side
   PNGs~~ **DONE** — next step is pixel-diff CI against `hsx screenshot`.
9. **Virtualized rendering** for large workbooks (current code paints
   the whole sheet).
10. **Active-cell + selection rendering** for the HITL preview.
