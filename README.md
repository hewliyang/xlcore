# xlsx-preview

Render `.xlsx` workbooks to a `<canvas>` or PNG — in the browser, in Node, or
from the CLI. Published as [`@hewliyang/xlsx-preview`](https://www.npmjs.com/package/@hewliyang/xlsx-preview).

```bash
npm install @hewliyang/xlsx-preview
```

## browser

```ts
import { createWorkbookPreviewerFromFile } from "@hewliyang/xlsx-preview/browser";

const file = /* File from an <input type="file"> */;
await createWorkbookPreviewerFromFile(container, file);
```

The browser entry runs the Rust extractor as wasm inside a Web Worker, so the
xlsx never leaves the page.

## react

```tsx
import { ExcelPreviewer } from "@hewliyang/xlsx-preview/react";

<ExcelPreviewer file={file} />;
```

Vite and webpack 5 emit the worker and wasm asset automatically. If your
setup does not, pass explicit asset URLs:

```tsx
import workerUrl from "@hewliyang/xlsx-preview/dist/xlsxWorker.js?url";
import wasmBinaryUrl from "@hewliyang/xlsx-preview/dist/xlcore_wasm_bg.wasm?url";

<ExcelPreviewer file={file} workerUrl={workerUrl} wasmBinaryUrl={wasmBinaryUrl} />;
```

For plain ESM pages, use CDN-hosted assets:

```tsx
import { jsDelivrUrls } from "@hewliyang/xlsx-preview/cdn";

<ExcelPreviewer file={file} {...jsDelivrUrls("0.0.2")} />;
```

## node

```ts
import { renderXlsxToPng } from "@hewliyang/xlsx-preview";
import { writeFile } from "node:fs/promises";

const png = await renderXlsxToPng("model.xlsx", { scale: 2 });
await writeFile("out.png", png);
```

## cli

```bash
xlsx-preview model.xlsx --info
xlsx-preview model.xlsx -o cover.png --sheet Cover --range B3:E12 --scale 2
xlsx-preview model.xlsx -o previews/{index}-{sheet}.png --all
```

## examples

Runnable HTML demos live under
[`packages/xlsx-preview/examples/`](packages/xlsx-preview/examples/):

- [`xlsx-app.html`](packages/xlsx-preview/examples/xlsx-app.html) — full
  previewer app (file picker, sheet tabs, zoom).
- [`xlsx-tiles.html`](packages/xlsx-preview/examples/xlsx-tiles.html) —
  multi-workbook tiled previews

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
