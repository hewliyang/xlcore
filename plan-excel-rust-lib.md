# plan: rust xlsx lib for agent + human-loop spreadsheets

Goal: a Rust crate that (1) round-trips xlsx with high fidelity, (2) recalcs formulas, (3) exposes an agent-friendly API, (4) feeds a browser-canvas renderer for human-in-the-loop preview.

## architecture

```
xlsx bytes ─► xlcore (Rust, wasm-targetable)
                ├── xlcore-io      = ooxmlsdk wrapper (OOXML fidelity)
                ├── xlcore-engine  = ironcalc fork (calc + dep graph)
                └── xlcore-bridge  = ~250 LOC, harvest cells → ironcalc, write
                                     recomputed <v> back into ooxmlsdk tree
                emits structured WorkbookLayout JSON
                    │
        ┌───────────┴───────────┐
        ▼                       ▼
  browser <canvas>         node-canvas (same TS)
  HITL preview             agent visual verify, PDF
```

Validated by spikes in `/tmp/ssbench/` (now folded into the main
workspace; the canonical kitchen-sink fixture lives at
`tests/fixtures/kitchensink/kitchensink.xlsx`):

- `src/spike.rs` + `src/spike_mutate.rs` — fuse pattern works, 100% feature fidelity round-trip on the kitchen-sink workbook (charts/CF/tables/comments/theme/extLst all preserved), dependency graph fires correctly on input mutation.
- `src/render_spike.rs` + `out/render_canvas.html` — same engine output renders correctly via browser `<canvas>`, badly via tiny-skia. **Drop pure-Rust rendering.**

## key decisions

1. **Compose, don't rewrite.** `ooxmlsdk` (`/tmp/ooxmlsdk/`, ~118k LOC of generated SpreadsheetML schemas, MIT/Apache) for I/O fidelity; `ironcalc` (`~/.cargo/registry/src/index.crates.io-*/ironcalc-0.7.1/`) for calc. Ship v0 in months, not years.
2. **Fill the IronCalc function gap.** Confirmed missing in 0.7.1: `SUMPRODUCT`, `LET`, `LAMBDA`, `FILTER`, `SORT`, `SORTBY`, `UNIQUE`, `SEQUENCE`, `XMATCH`, `HSTACK/VSTACK/TOCOL/TOROW/TAKE/DROP/CHOOSE{ROWS,COLS}`, `BYROW/BYCOL/MAP/REDUCE/SCAN/MAKEARRAY`, `AGGREGATE`. Spec source: LibreOffice `sc/source/core/tool/interpr1.cxx … interpr8.cxx`. Fallback already implemented in spike: when engine errors, preserve source-cached `<v>`.
3. **Engine emits workbook data, not pixels.** Cells with resolved styles, merge maps, column widths, CF evaluated per-cell, chart data, drawings. NOT rasterized primitives.
4. **Renderer is TypeScript on canvas.** Same code in browser and Node (via `node-canvas`). Native browser text/font/AA. Reference: `/Users/m1a1/Developer/oai-artifact-tools/examples/browser-workbook-preview/src/simple-workbook-canvas.mjs` (2800 LOC, the realistic scope) and `walnut-spreadsheet-proto.d.ts` (schema-design thinking already done).
5. **Defer:** OT/CRDT collab, multi-threaded recalc, vectorized formula groups, full chart suite. All possible but v1+.

## non-goals for v0

- ~~Charts beyond placeholder boxes with title.~~ **Shipped:** clustered/stacked column + bar charts with title, axis ticks (workbook number-format aware), theme accent series colors, legend, range-formula resolution when numCache is empty. **Line / pie+doughnut / area (standard, stacked, percentStacked) / xy scatter** also shipped — see `tests/fixtures/charts/line-pie-area-scatter.xlsx` and the per-row notes in PARITY.md.
- Pivot tables, slicers, timelines (engine preserves the XML, renderer skips).
- LAMBDA/LET (artifact-tool itself stubs these — see `/tmp/ssbench/artifact_tool.pretty.mjs:274957`).
- Native-Rust rendering. Use browser canvas.

## crate layout

```
xlcore/
  xlcore-io/       # ooxmlsdk facade, narrowed schema generator (~70% size cut)
  xlcore-engine/   # ironcalc fork + missing functions
  xlcore-bridge/   # harvest/replay/write-back; agent batch-mutation API
  xlcore-export/   # WorkbookLayout serializer (JSON + Postcard)
  xlcore-wasm/     # wasm-bindgen entry for browser
xlcore-render-ts/  # separate npm pkg, runs in browser + Node
```

## validation

Ground truth = `hsx screenshot` (SpreadJS, the OAI artifact-tool's
Office-grade renderer). Same-class reference = OAI's Walnut canvas demo.
The canonical fixture lives at
`tests/fixtures/kitchensink/kitchensink.xlsx`; per-feature fixtures sit
alongside it under `tests/fixtures/<feature>/`. See
[`TESTING.md`](TESTING.md) for the workflow,
[`tests/fixtures/README.md`](tests/fixtures/README.md) for the fixture
table + how to add new ones, and [`PARITY.md`](PARITY.md) for the
feature-by-feature scoreboard.

## references on this machine

- `/tmp/ssbench/` — all spike code + outputs (kitchen sink, mutated, render comparisons)
- `/tmp/ooxmlsdk/` — full ooxmlsdk source, 159 schema files
- `~/.cargo/registry/src/index.crates.io-*/ironcalc{,_base}-0.7.1/` — IronCalc internals; `src/functions/mod.rs` for the function enum
- `/Users/m1a1/Developer/oai-artifact-tools/examples/browser-workbook-preview/` — OAI's canvas renderer reference (`README.md`, `src/simple-workbook-canvas.mjs`, `FULL_FIDELITY_PLAN.md`)
- `/Users/m1a1/Developer/oai-artifact-tools/openai-primary-runtime/plugins/spreadsheets/skills/spreadsheets/SKILL.md` — agent-facing API surface that artifact-tool exposes; useful as a target shape for our agent API
- `/tmp/ssbench/artifact_tool.pretty.mjs` — prettier-formatted OAI bundle (305k lines), grep for function impls vs stubs
- LibreOffice Calc (clone separately): `sc/source/core/tool/interpr*.cxx` — function semantics spec; `sc/source/filter/oox/` — xlsx import quirks

## first milestones

1. ~~Fork IronCalc, port `SUMPRODUCT`~~ **TODO** — still the headline blocker for live recalc.
2. ~~Define `WorkbookLayout` schema based on `walnut-spreadsheet-proto.d.ts`.~~ **DONE** — `crates/xlcore-export/src/schema.rs` + mirrored `render-ts/src/types.ts`.
3. ~~Port a slim `simple-workbook-canvas.mjs` (cells/styles/borders/merges/basic CF, no charts).~~ **DONE** — `render-ts/src/render.ts` (~700 LOC). Includes CF color scales, text overflow into empty neighbors, vector-crisp zoom (re-renders on DPR + app-zoom changes), subtle freeze indicators, basic number formats.
4. Wire `xlcore` → wasm → JSON → browser canvas end-to-end. **PARTIAL** — JSON pipeline + standalone HTML preview shipped (`xlcore preview` inlines renderer + data). Wasm entry not done; preview currently re-runs the Rust extractor server-side.
5. ~~Add `node-canvas` backend running same TS for server PNG.~~ **TODO** — `render-ts` is target-agnostic but the node-canvas adapter isn't wired yet.

Bonus shipped (was "v1+"):

- ~~Charts beyond placeholders.~~ **DONE** for column + bar (clustered + stacked) plus line / area (standard/stacked/percentStacked) / pie+doughnut / xy scatter, all with axis number formats, theme colors, and shared legend. See `crates/xlcore-export/src/charts.rs` and `render-ts/src/chart.ts`.
- ~~Theme XML parsing.~~ **DONE.** `xl/theme/theme1.xml` parsed by
  `crates/xlcore-export/src/theme.rs`; emitted as
  `WorkbookLayout.theme` (12-entry palette in spreadsheet-index order +
  major/minor font names). Cell colors and chart-series accents now
  resolve against the workbook's actual theme instead of hardcoded
  Office 2007+ defaults. Fixture:
  `tests/fixtures/themes/custom-theme-accent.xlsx`.
- ~~Source-controlled fixture corpus.~~ **STARTED** under
  `tests/fixtures/`, with reproducible build scripts. See
  `tests/fixtures/README.md`.

After milestone 1 (recalc) + 4 (wasm) + 5 (node-canvas): usable v0 for agent edits + HITL preview. The long tail (line/pie/scatter charts, pivot tables, selection UI, virtualized scrolling, CF beyond color scales) comes next — see `PARITY.md` for the ranked list.
