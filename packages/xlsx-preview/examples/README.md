# Examples

Live demos: <https://xlcore.pages.dev>.

## Plain HTML

Open these directly in a browser (or serve the folder statically). They
prefer a local `../dist/` build, falling back to the published CDN package
outside local/dev hosts.

- [`xlsx-app.html`](./xlsx-app.html) — full previewer (file picker, sheet
  tabs, zoom, selection).
- [`xlsx-tiles.html`](./xlsx-tiles.html) — thumbnail tile grid.
- [`xlsx-multi.html`](./xlsx-multi.html) — multiple workbooks in
  side-by-side panes.

## No-build CDN demo

- [`vanilla-js/demo.html`](./vanilla-js/demo.html) — the smallest possible
  integration. Open the file in a browser; code and wasm are served from
  jsDelivr. Uses main-thread extraction so it also works from `file://`. This
  pinned demo only shows features already published to npm.

## Framework starters

Each is a self-contained app that depends on the local package via
`file:../..`, so the examples exercise this worktree rather than the last
published npm version. Run them in place with `pnpm install && pnpm dev`.

- [`vanilla-js/`](./vanilla-js/) — Vite + plain JavaScript.
- [`react-vite/`](./react-vite/) — Vite + React + TypeScript.
- [`nextjs/`](./nextjs/) — Next.js App Router.
