# Changelog

All notable changes to `@hewliyang/xlsx-preview` are documented here.
This project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Expanded chart support for combo and dual-axis charts, including secondary
  value axes, per-series axis groups/chart types, secondary formats/scaling,
  axis titles, display units/labels, and secondary gridline metadata.
- Added bubble chart schema/rendering support with bubble sizes, bubble scale,
  and size-representation handling.
- Added per-data-point chart data-label overrides (`PointDataLabel`) with
  literal text, delete/suppress, position, number-format, and show-field
  inheritance/overrides.
- Added chart fixtures/builders for bubble charts, per-point data labels,
  no-fill stacked waterfall bars, stacked color modifiers, combo secondary
  axes, and dual-axis lines.
- Added chart utility tests for bar slot metrics, display-unit axis formatting,
  and zero-baseline helpers.

### Changed

- Improved chart axis range resolution to honor explicit scaling bounds,
  avoid unintended zero-clamping, apply display-unit divisors to tick labels,
  and draw a heavier zero baseline when axes straddle zero.
- Improved bar/column geometry to follow OOXML `gapWidth` and `overlap` for
  clustered, stacked, and percent-stacked charts.
- Improved legends to reflect series style with filled swatches, line strokes,
  markers, or line+marker combinations.
- Improved line, scatter, combo, pie/doughnut, and bar rendering for marker
  suppression, blank-point gaps, sorted scatter line paths, per-point colors,
  no-fill point overrides, and per-point labels.

### Fixed

- Fixed value-axis gridline rendering so gridlines only paint when authored
  and do not double-paint the zero line.
- Fixed chart data labels across bar, line, area, pie/doughnut, scatter, and
  combo renderers to respect per-point delete/text/format/position overrides.
- Fixed generated TypeScript schema exports for the new chart and data-label
  fields.

## [0.0.4] - 2026-05-13

### Fixed

- Cross-origin worker URLs now work. When `workerUrl` resolves to a
  different origin (e.g. a jsDelivr or unpkg CDN), the loader wraps the
  script in a same-origin Blob shim before constructing the module worker,
  so the documented `jsDelivrUrls()` / `unpkgUrls()` flow renders instead
  of throwing `Failed to construct 'Worker': ... cannot be accessed from
  origin`.
- Workbooks from producers that use alternate threaded-comment namespace
  prefixes, including Google Sheets, now load without
  `unexpected tag while parsing PersonList` errors.
- Data-bar conditional formats that use Excel's `<x14:color>` fill-color
  element now load without `unexpected tag while parsing DataBar` errors.
- Charts anchored with `<xdr:oneCellAnchor>`, including Excel's
  "move but don't size with cells" drawings, are now rendered.
- Chart data resolution now ignores text cells, so shared-string indexes are
  no longer treated as numeric series values.
- Chart series backed by padded array-formula ranges are trimmed at the last numeric value instead of rendering an empty zero-value tail.
- Pie and doughnut legends now render one entry per category, using the same per-slice colors as the chart (`c:dPt` overrides, otherwise theme accents).
- Dense line and area chart category labels are thinned to avoid overlap.
- Numeric category-axis labels, including date serials, now use the chart cache or source cell number format.
- Text format `@` applied to a numeric cell (e.g. a formula result with `numFmtId=49`) now renders the value via general formatting instead of an empty string.
- Rotated text no longer clips to its cell rect, so huge fonts in narrow cells (e.g. a 220pt vertical "2026" in a 21px-wide merged column) render instead of vanishing. Stacked text (`textRotation=255`) is still clipped — its glyphs always fit by construction.
- Rotated text with `halign=center`/`left`/`right` now positions the rotated glyph bounding box rather than its baseline, fixing horizontal placement at 90° + large font sizes where ascender/descender asymmetry shifted the glyph noticeably off the column center.

### Changed

- Legends now honor the chart's `legendPos` value, including vertical
  left and right legends.

### Added

- `DrawingAnchor.extEmuCx` and `extEmuCy` expose explicit
  `oneCellAnchor` extents to the renderer.
- `Chart.categoriesFormat` exposes the number format used for category-axis
  labels.

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
