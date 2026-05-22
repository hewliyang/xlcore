# xlsx-preview · Vanilla JS

Two flavors, pick one:

## 1. No-build, single HTML file

[`demo.html`](./demo.html) loads the currently published package from a CDN —
just open it in a browser (or serve the folder statically) and pick an
`.xlsx` file. No
`npm install`, no bundler.

Because double-clicked `file://` pages cannot start module workers in Chrome,
this demo runs the wasm extractor on the main thread and then renders with
`createWorkbookPreviewer`. For production apps, prefer the Vite starter below
or the React / Next.js starters so extraction runs in a Worker.

CSV and Parquet support is available in this worktree through the build-based
starter below; the pinned CDN demo should only advertise features already
published to npm.

## 2. Build-based starter (Vite)

Minimal Vite + plain JavaScript using
`@hewliyang/xlsx-preview/browser`. No framework, no TypeScript.
This starter depends on the local package via `file:../..`, so run it from
this directory when testing worktree changes.

```bash
pnpm install
pnpm dev
```

Then open the printed URL and pick an `.xlsx`, `.csv`, or `.parquet` file.

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
| `demo.html` | No-build CDN version, works from `file://` |
| `index.html` | Vite entry markup |
| `src/main.js` | One call to `createWorkbookPreviewerFromFile` |
| `vite.config.js` | `optimizeDeps.exclude` so the worker URL resolves in dev |
