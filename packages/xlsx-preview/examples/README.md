# Examples

Live demos: <https://xlcore.pages.dev>.

## Plain HTML

Open these directly in a browser (or serve the folder statically). They
load `@hewliyang/xlsx-preview` from jsDelivr.

- [`xlsx-app.html`](./xlsx-app.html) — full previewer (file picker, sheet
  tabs, zoom, selection).
- [`xlsx-tiles.html`](./xlsx-tiles.html) — thumbnail tile grid.
- [`xlsx-multi.html`](./xlsx-multi.html) — multiple workbooks in
  side-by-side panes.

## No-build CDN demo

- [`vanilla-js/demo.html`](./vanilla-js/demo.html) — the smallest possible
  integration. Open the file in a browser; everything (code, worker,
  wasm) is served from jsDelivr.

## Framework starters

Each is a self-contained app that installs `@hewliyang/xlsx-preview` from
npm. Copy the directory anywhere and `pnpm install && pnpm dev`.

- [`vanilla-js/`](./vanilla-js/) — Vite + plain JavaScript.
- [`react-vite/`](./react-vite/) — Vite + React + TypeScript.
- [`nextjs/`](./nextjs/) — Next.js App Router.
