# xlsx-preview

Render `.xlsx`, `.csv`, and Parquet workbooks to a `<canvas>` or PNG — in the
browser, in Node, or from the CLI. Published as
[`@hewliyang/xlsx-preview`](https://www.npmjs.com/package/@hewliyang/xlsx-preview).

```bash
npm install @hewliyang/xlsx-preview
```

## browser

```ts
import { createWorkbookPreviewerFromFile } from "@hewliyang/xlsx-preview/browser";

await createWorkbookPreviewerFromFile(container, file);
```

The browser entry runs the Rust extractor as wasm inside a Web Worker, so the
file never leaves the page. See the
[`vanilla-js` examples](packages/xlsx-preview/examples/vanilla-js/) for both a
no-build CDN demo and a Vite starter.

## react

```tsx
import { ExcelPreviewer } from "@hewliyang/xlsx-preview/react";

<ExcelPreviewer file={file} />;
```

See the framework starters for full minimal projects:
[`react-vite`](packages/xlsx-preview/examples/react-vite/) and
[`nextjs`](packages/xlsx-preview/examples/nextjs/).

## node

```ts
import {
  loadWorkbookFromCsv,
  loadWorkbookFromParquet,
  renderXlsxToPng,
} from "@hewliyang/xlsx-preview/node";
import { writeFile } from "node:fs/promises";

const png = await renderXlsxToPng("model.xlsx", { scale: 2 });
await writeFile("out.png", png);

const csvLayout = await loadWorkbookFromCsv("sales.csv");
const parquetLayout = await loadWorkbookFromParquet("events.parquet");
```

## cli

```bash
xlsx-preview model.xlsx --info
xlsx-preview model.xlsx -o cover.png --sheet Cover --range B3:E12 --scale 2
xlsx-preview model.xlsx -o previews/{index}-{sheet}.png --all
xlsx-preview data.csv -o data.png --delimiter ","
xlsx-preview events.parquet -o events.png --max-rows 5000
```

## examples

Live demos: <https://xlcore.pages.dev>.

For runnable examples and minimal starters, see
[`packages/xlsx-preview/examples/`](packages/xlsx-preview/examples/).

## development

This repo is a Rust workspace (the extractor + wasm + a `xlcore` CLI) plus the
TypeScript renderer package.

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
  `xlcore-wasm` inside a Web Worker and hands the JSON to the canvas renderer
  in the main thread. No server.
- **Node / CLI:** `renderToPng()` / `renderXlsxToPng()` use `skia-canvas`
  against the same render pass; the `xlsx-preview` bin shells out to it.
- **Static HTML preview:** `xlcore preview <file.xlsx>` (Rust CLI) extracts
  server-side and inlines `{ layout JSON (gzip+base64), renderer bundle }`
  into one self-contained `.html`.

The JSON contract is generated from one source via
[`ts-rs`](https://github.com/Aleph-Alpha/ts-rs) — Rust types in
`crates/xlcore-export/src/schema.rs` produce the TS in
`packages/xlsx-preview/src/schema/`. Regenerate after schema changes with
`cargo test --release -p xlcore-export export_bindings`.

Contributor docs:

- [`docs/plan-excel-rust-lib.md`](docs/plan-excel-rust-lib.md) — architecture
  and roadmap.
- [`docs/PARITY.md`](docs/PARITY.md) — feature-by-feature scoreboard against
  Excel / SpreadJS.
- [`docs/TESTING.md`](docs/TESTING.md) — fixture + visual-diff workflow.
