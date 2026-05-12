# Changelog

All notable changes to `@hewliyang/xlsx-preview` are documented here.
This project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.0.3] - 2026-05-12

### Fixed

- `@hewliyang/xlsx-preview/browser` and the example HTML files now resolve
  against the actual emitted file. In 0.0.2 the loader was emitted as
  `dist/browserLoader.js` (matching the source name and the existing
  `.d.ts`), but `package.json` `exports["./browser"]` and the demo HTML
  pages still pointed at the legacy `dist/browser-loader.js` path.

## [0.0.2] - 2026-05-12

### Fixed

- Browser and React entry points now work in Vite and webpack 5 without
  manual asset configuration. The worker and wasm binary are shipped as
  discoverable ESM assets instead of being hidden inside a pre-bundled file.
- The browser worker initializes wasm from the resolved binary URL provided
  by the loader.
- Corrected the Node `renderXlsxToPng` README example. The function returns
  a `Buffer`; callers write it to disk themselves.

### Added

- `@hewliyang/xlsx-preview/cdn`, with `jsDelivrUrls(version)` for plain
  ESM pages and other non-bundled environments.

### Changed

- Renamed the browser loader option `wasmUrl` to `wasmBinaryUrl`; it now
  points directly at `xlcore_wasm_bg.wasm`. `workerUrl` is unchanged.
- Declared `engines.node >= 20`.

## [0.0.1] - 2026-05-12

- Initial release: canvas renderer + Node CLI + React/browser entry points.
- Rust extractor (`xlcore-export`) → `WorkbookLayout` JSON shared via `ts-rs`.
- Self-contained wasm extractor bundled into `dist/` for the browser entry.
- See [`docs/PARITY.md`](../../docs/PARITY.md) for the feature scoreboard.
