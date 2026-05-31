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
  xlcore-tabular/  # ✅ csv + parquet (parquet behind `parquet` cargo feature; enabled in wasm)
  ironcalc-base/   # 🟡 vendored IronCalc fork; SUMPRODUCT added
  xlcore-engine/   # 🟡 thin engine facade + core recalc tests + LET shim PoC
  xlcore-bridge/   # 🟡 first OOXML harvest/recalc API; write-back still pending
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
feature-by-feature scoreboard. Formula/recalc parity has its own hillclimb in
[`parity-engine.md`](parity-engine.md).

## references on this machine

- `./ecma-376/` — **indexed ECMA-376 Parts 1–4**. Always consult via the
  `./ecma-376/ecma` CLI (`search`, `toc`, `show`) before citing any
  section number, element name, or attribute semantics. **Do not
  hallucinate ECMA section IDs / titles** — if you need spec backing,
  run the CLI and quote the real section. Example:
  `./ecma-376/ecma search "shared strings"` →
  `./ecma-376/ecma show p1-18-4-9-sst-shared-string-table`.
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

1. **`xlcore-engine` + `xlcore-bridge`.** IronCalc is now vendored as
   `crates/ironcalc-base`, with `SUMPRODUCT` added in the fork.
   `xlcore-engine` has the first thin facade with same-sheet and cross-sheet
   recalc tests plus a scalar `LET` compatibility-shim PoC. `xlcore-bridge`
   now harvests scalar cells/formulas from OOXML and returns recalculated formula
   values. Next: replay evaluated values into `WorkbookLayout` and write
   updated cached `<v>` values back into OOXML, then fill the rest of the
   function gap (§key decisions #2). Unblocks live recalc, `#SPILL!`,
   `expression` CF rules, and the agent batch-mutation API.
2. **Selection / active-cell** rendering for HITL.
3. The long tail: combo charts, secondary axes, slicers, validation
   UI, formula-driven CF, `autoFilter` filtered-row hiding. See
   `PARITY.md` for the ranked list.

Visual fidelity is checked manually against `hsx screenshot` (ground
truth) per `TESTING.md` — a pixel-diff CI was scoped and rejected as
unrealistic (Excel/SpreadJS render small subpixel deltas across font
stacks + DPI that swamp imagehash tolerances; the manual checklist
catches regressions cheaper).
