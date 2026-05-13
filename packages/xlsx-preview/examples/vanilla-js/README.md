# xlsx-preview · Vanilla JS

Two flavors, pick one:

## 1. No-build, single HTML file

[`demo.html`](./demo.html) loads everything from a CDN — just open it in
a browser (or serve the folder statically) and pick an `.xlsx` file. No
`npm install`, no bundler. This is the smallest possible integration
(~30 lines of HTML + JS).

The CDN flow works thanks to the same-origin Blob worker shim added in
0.0.4 — earlier versions throw on cross-origin module workers.

## 2. Build-based starter (Vite)

Minimal Vite + plain JavaScript using
`@hewliyang/xlsx-preview/browser`. No framework, no TypeScript.

```bash
pnpm install
pnpm dev
```

Then open the printed URL and pick an `.xlsx` file.

## What this shows

- `createWorkbookPreviewerFromFile(container, file)` is the entire
  integration. It returns a `WorkbookPreviewer` you can `.destroy()` when
  swapping files.
- Vite picks up the worker (`xlsxWorker.js`) and wasm
  (`xlcore_wasm_bg.wasm`) automatically.
- `pnpm build` emits the worker as a hashed asset under `dist/assets/`.

## Files

| File | Why |
| --- | --- |
| `demo.html` | No-build CDN version |
| `index.html` | Vite entry markup |
| `src/main.js` | One call to `createWorkbookPreviewerFromFile` |
| `vite.config.js` | `optimizeDeps.exclude` so the worker URL resolves in dev |
