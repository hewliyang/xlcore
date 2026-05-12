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
  browser <canvas>         skia-canvas (same TS)
  HITL preview             agent visual verify, PDF
```

The two design bets that justified the architecture were validated by
early spikes (now folded into this workspace; the canonical kitchen-sink
fixture lives at `tests/fixtures/kitchensink/kitchensink.xlsx`):

- ooxmlsdk + a thin fuse layer round-trips the kitchen-sink workbook
  with 100% feature fidelity (charts/CF/tables/comments/theme/extLst
  all preserved); IronCalc's dependency graph fires correctly on input
  mutation.
- The same engine output renders cleanly via browser `<canvas>` and
  badly via `tiny-skia`. **Decision: drop pure-Rust rendering.**

## key decisions

1. **Compose, don't rewrite.** `ooxmlsdk` (`/tmp/ooxmlsdk/`, ~118k LOC of generated SpreadsheetML schemas, MIT/Apache) for I/O fidelity; `ironcalc` (`~/.cargo/registry/src/index.crates.io-*/ironcalc-0.7.1/`) for calc. Ship v0 in months, not years.
2. **Fill the IronCalc function gap.** Confirmed missing in 0.7.1: `SUMPRODUCT`, `LET`, `LAMBDA`, `FILTER`, `SORT`, `SORTBY`, `UNIQUE`, `SEQUENCE`, `XMATCH`, `HSTACK/VSTACK/TOCOL/TOROW/TAKE/DROP/CHOOSE{ROWS,COLS}`, `BYROW/BYCOL/MAP/REDUCE/SCAN/MAKEARRAY`, `AGGREGATE`. Spec source: LibreOffice `sc/source/core/tool/interpr1.cxx … interpr8.cxx`. Fallback already implemented in spike: when engine errors, preserve source-cached `<v>`.
3. **Engine emits workbook data, not pixels.** Cells with resolved styles, merge maps, column widths, CF evaluated per-cell, chart data, drawings. NOT rasterized primitives.
4. **Renderer is TypeScript on canvas.** Same code in browser and Node (via `skia-canvas`). Native browser text/font/AA. Reference: `/Users/m1a1/Developer/oai-artifact-tools/examples/browser-workbook-preview/src/simple-workbook-canvas.mjs` (2800 LOC, the realistic scope) and `walnut-spreadsheet-proto.d.ts` (schema-design thinking already done).
5. **Defer:** OT/CRDT collab, multi-threaded recalc, vectorized formula groups, full chart suite. All possible but v1+.

## non-goals for v0

- ~~Charts beyond placeholder boxes.~~ **Shipped.** Column / bar
  (clustered + stacked), line, area (standard/stacked/percentStacked),
  pie + doughnut (with per-slice `<c:dPt>` colors), xy scatter
  (`lineMarker` / `smoothMarker`), data labels.
- ~~Pivot tables.~~ **Cheap path shipped** — materialized cells +
  filter-arrow chevrons. True pivot interactivity (filter / refresh /
  expand-collapse) still out of scope until the aggregation engine
  lands. Slicers, timelines remain out of scope.
- LAMBDA/LET (artifact-tool itself stubs these).
- Native-Rust rendering. Use browser canvas.

## crate layout

```
crates/
  xlcore-io/       # ✅ ooxmlsdk facade
  xlcore-export/   # ✅ WorkbookLayout serializer (ts-rs-generated TS bindings)
  xlcore-wasm/     # ✅ wasm-bindgen entry for browser (used by xlsxWorker.ts)
  xlcore-cli/      # ✅ `xlcore extract` / `xlcore preview`
  xlcore-engine/   # 🔴 not yet — ironcalc fork + missing functions
  xlcore-bridge/   # 🔴 not yet — harvest/replay/write-back; agent batch-mutation API
packages/xlsx-preview/  # ✅ npm package, runs in browser + Node
```

## validation

Ground truth = `hsx screenshot` (SpreadJS, the OAI artifact-tool's
Office-grade renderer). Same-class reference = OAI's Walnut canvas demo.
The canonical fixture lives at
`tests/fixtures/kitchensink/kitchensink.xlsx`; per-feature fixtures sit
alongside it under `tests/fixtures/<feature>/`. See
[`TESTING.md`](TESTING.md) for the workflow,
[`tests/fixtures/README.md`](../tests/fixtures/README.md) for the fixture
table + how to add new ones, and [`PARITY.md`](PARITY.md) for the
feature-by-feature scoreboard.

## references on this machine

- `/tmp/ooxmlsdk/` — full ooxmlsdk source, 159 schema files
- `~/.cargo/registry/src/index.crates.io-*/ironcalc{,_base}-0.7.1/` — IronCalc internals; `src/functions/mod.rs` for the function enum
- `/Users/m1a1/Developer/oai-artifact-tools/examples/browser-workbook-preview/` — OAI's canvas renderer reference (`src/simple-workbook-canvas.mjs`, `FULL_FIDELITY_PLAN.md`)
- `/Users/m1a1/Developer/oai-artifact-tools/openai-primary-runtime/plugins/spreadsheets/skills/spreadsheets/SKILL.md` — agent-facing API surface that artifact-tool exposes; useful as a target shape for our agent API
- LibreOffice Calc (clone separately): `sc/source/core/tool/interpr*.cxx` — function semantics spec; `sc/source/filter/oox/` — xlsx import quirks

## status

The extraction → schema → canvas pipeline is **end-to-end in the browser**
via wasm (`xlcore-wasm` + `packages/xlsx-preview/src/xlsxWorker.ts`) and
in Node via `skia-canvas` (`renderToPng()` in `packages/xlsx-preview/
src/node.ts`). See `PARITY.md` for the per-feature scoreboard — most of
the original v0 milestones plus a good chunk of the "v1+" wishlist
(line/pie/area/scatter, theme XML, sparklines, pivot cheap-path, x14
comments, all of CF except `expression`) have landed.

Remaining headline work:

1. **`xlcore-engine` + `xlcore-bridge`.** Fork IronCalc, fill the
   function gap (§key decisions #2), wire harvest/replay through to
   the layout. Unblocks live recalc, `#SPILL!`, `expression` CF rules,
   and the agent batch-mutation API.
2. **Selection / active-cell** rendering for HITL.
3. The long tail: combo charts, secondary axes, slicers, validation
   UI, formula-driven CF, `autoFilter` filtered-row hiding. See
   `PARITY.md` for the ranked list.

Visual fidelity is checked manually against `hsx screenshot` (ground
truth) per `TESTING.md` — a pixel-diff CI was scoped and rejected as
unrealistic (Excel/SpreadJS render small subpixel deltas across font
stacks + DPI that swamp imagehash tolerances; the manual checklist
catches regressions cheaper).
