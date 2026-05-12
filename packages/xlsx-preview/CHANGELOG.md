# Changelog

All notable changes to `@hewliyang/xlsx-preview` are documented here.
This project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.0.2] - 2026-05-12

### Fixed

- **Browser/React entry now works out of the box in Vite and webpack 5.**
  The browser loader was previously pre-bundled by esbuild, which defeated
  the consuming bundler's static asset analysis — the wasm binary and the
  worker module never made it into the user's build, and a Blob-worker
  fallback hid the real error. `browserLoader.js` and `xlsxWorker.js` are
  now shipped un-bundled, so `new Worker(new URL("./xlsxWorker.js", import.meta.url), { type: "module" })`
  and `new URL("./xlcore_wasm_bg.wasm", import.meta.url)` are visible to
  bundlers and get emitted as static assets automatically.
- The worker now statically imports the wasm-bindgen shim and accepts the
  resolved wasm binary URL via `postMessage`, bypassing the shim's broken
  `new URL(..., import.meta.url)` default for the wasm binary.
- The Blob-worker fallback path was removed — it could never work with
  multi-MB wasm (null-origin workers can't dynamically import
  same-origin modules) and silently masked real errors.
- README: corrected the Node `renderXlsxToPng` example, which previously
  showed a non-existent `(input, output, opts)` signature; the real
  signature is `(input, opts) => Promise<Buffer>`.

### Added

- `@hewliyang/xlsx-preview/cdn` subpath exporting `jsDelivrUrls(version)`
  for unbundled / plain-`<script type="module">` use.
- README recipes for Vite explicit-URL setup and CDN consumption.

### Changed

- **Breaking (unreleased-grade):** the browser loader option `wasmUrl`
  (which pointed at the JS shim and was effectively undocumented) is
  replaced by `wasmBinaryUrl` (the `.wasm` file directly). `workerUrl`
  is unchanged. Nobody was successfully using the old option because the
  default path didn't work; this rename makes the explicit-URL escape
  hatch match what bundlers like Vite produce from `?url` imports.
- `engines.node >= 20` declared explicitly. The library has always
  required ES2022 + ESM; this just makes it loud.

## [0.0.1] - 2026-05-12

- Initial release: canvas renderer + Node CLI + React/browser entry points.
- Rust extractor (`xlcore-export`) → `WorkbookLayout` JSON shared via `ts-rs`.
- Self-contained wasm extractor bundled into `dist/` for the browser entry.
- See [`docs/PARITY.md`](../../docs/PARITY.md) for the feature scoreboard.
