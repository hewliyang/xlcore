# Changelog

All notable changes to `@hewliyang/xlsx-preview` are documented here.
This project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- DrawingML `<a:fld>` (text field) runs are now extracted alongside
  `<a:r>`. Previously the `ParagraphChoice` match in
  `shapes.rs::text_body_to_paragraphs` only handled `AR` / `ABr` and
  fell through `_ => {}` for everything else — silently dropping every
  `<a:fld type="TxLink">` text run on the floor. Field runs are how
  Excel caches the displayed value of shape text bound to a cell via
  the `textlink="$T$51"` attribute, and dashboard / SOTP-style diagrams
  use them constantly: on `e-007_input-4.xlsx :: SOTP Summary`, every
  numeric cell (`$252.0B`, `+13%`, `$163.8B`, `16.0x '27E EBITDA`, …),
  every period header (`'26E` / `'27E`), and the entire bottom red
  banner ("Alphabet SOTP Enterprise Value = $4.17T …") were emitted
  via `<a:fld>`. New `AFld` arm builds a `TextRun` from `field.text` +
  `field.run_properties` and runs it through the same lstStyle → lvl →
  pPr/defRPr → rPr cascade we already use for regular runs.
- DrawingML `<a:rPr u="...">` / `strike="..."` are enums, not bools.
  `apply_run_properties` and `apply_default_run_properties` previously
  set `tr.underline = rp.underline.is_some()`, but `u="none"` parses to
  `Some(TextUnderlineValues::None)` — so every run that explicitly
  turned underline OFF still rendered underlined (same trap for
  `strike="noStrike"`). Now gated through two new helpers
  (`underline_is_visible` / `strike_is_visible`) that pattern-match on
  the enum variant. This was invisible until the `<a:fld>` fix landed
  and exposed it on the SOTP sheet, where every field rPr sets
  `u="none" strike="noStrike"`.
- Wrapped shape text past line 1 was being dropped on centered short
  boxes. The clip rule in `shape.ts::drawShapeText` was
  `cursorY + lineHeight > innerY + innerH + 0.5 → break`, which
  discarded line 2 of any `anchor="ctr"` box whose two wrapped lines
  were ~1px taller than the body — killing the `SUBSCRIPTIONS,
  PLATFORMS, & DEVICES`, `PLAY, DEVICES, GOOGLE ONE, ETC.`,
  `YOUTUBE ADS`, and `YOUTUBE SUBS` labels on the SOTP sheet. Replaced
  with the spec-default `vertOverflow="overflow"` behavior: a wrapped
  line paints as long as its TOP starts inside the body rect, so the
  last line may spill slightly past the bottom (matching Excel's
  default) while genuinely-too-tall paragraphs still stop clipping at
  roughly the right place. Explicit `vertOverflow="clip"` modeling
  remains TODO.

### Added

- `<xdr:cxnSp>` connector endpoints now resolve `<a:stCxn id=N idx=I/>`
  and `<a:endCxn>` against the actual target shapes.
  `extract_shape_tree` does a pre-pass collecting `cNvPr/@id → world
  bbox` for every `xdr:sp` (including nested in groups), and
  `visit_connector` looks up each end via a new `connection_site()`
  helper that maps the four cardinal OOXML indices
  (`0/1/2/3 = top/right/bottom/left center`) onto the target bbox.
  When BOTH ends resolve, the connector's world bbox is rebuilt to
  span the two resolved points; `flipH`/`flipV` are derived from
  start-vs-end position; and the xfrm's `rot` is zeroed (the new bbox
  is already axis-aligned). Also surfaces a new `elbowAxis` field on
  `ShapeNode` (`"vertical"` when both sites are top/bottom,
  `"horizontal"` when both are left/right) so the `bentConnector3`
  painter overrides its w/h-ratio heuristic and picks the correct path
  orientation — without this, two arrows that share an `endCxn`
  (e.g. YOUTUBE ADS + YOUTUBE SUBS → YOUTUBE top on the SOTP sheet)
  would still meet at the same point but their last segments would
  approach horizontally in opposite directions and form a `▶◀` bowtie
  of arrowheads instead of merging into a single visual head.
  Preset-aware connection sites (chevron tip, star points, flowchart
  shape sides at non-cardinal positions) and the shape's own
  `<a:cxnLst>` are still TODO; today we fall back to bbox center for
  any `idx` outside `0..=3`.
- Brace and bracket presets: `leftBrace`, `rightBrace`, `leftBracket`,
  `rightBracket`. Stroked open paths with each outer corner traced as
  a 90° quadratic-bezier quarter-arc and (for braces only) a matching
  central cusp pointing toward the tip. Reads `adj1` (corner curl
  size, as 1/100000 of h) and `adj2` (tip Y, as 1/100000 of h),
  clamped per ECMA: curl in `[0, h/2]`, tip in `[curl, h-curl]`.
  `lineCap` / `lineJoin` switch to `round` while painting any
  brace-like preset so the arc joins don't show as right-angled tips.
  The horizontal `‿` brace that sits between the segment grid and the
  SOTP banner on `e-007_input-4.xlsx :: SOTP Summary` (a `leftBrace`
  rotated 270°) was the motivating case — it used to fall through the
  preset switch and render as a plain rectangle. `bracePair`,
  `bracketPair`, and the curly-brace diagonal variants are still
  unmodeled.
- `adj2` is now extracted on every `xdr:sp` (previously connectors
  only, and only `adj1`). The new `preset_adj_n` helper consolidates
  the attribute walk so `adj1` / `adj2` / future named adjusts can
  share one implementation. `roundRect`, arrow heads / tails,
  callouts, stars, and arcs still ignore their `avLst` and use
  hardcoded defaults.

### Changed

- Split `crates/xlcore-export/src/shapes.rs` into `shapes` / `shapes_style`
  / `shapes_text` and `packages/xlsx-preview/src/shape.ts` into
  `shape.ts` + `shapePaths.ts` to keep every file under the
  900-LoC `check:loc` ceiling. No behavior change — pure code motion
  (style-ref color resolution + run/paragraph property cascade moved
  out of `shapes.rs`; preset path construction + brace/line predicates
  moved out of `shape.ts`). `check-loc.ts` now skips `world110m.ts`
  (13k-line generated TopoJSON atlas).

- DrawingML `<a:lstStyle>` + paragraph-`<a:pPr><a:defRPr>` run/paragraph
  inheritance. `shapes.rs::text_body_to_paragraphs` now cascades run +
  paragraph properties in OOXML-spec precedence order, lowest → highest:
  (1) `<a:lstStyle><a:defPPr><a:defRPr>`, (2)
  `<a:lstStyle><a:lvl{N+1}pPr><a:defRPr>` matching the paragraph's
  `pPr@lvl` (default 0 → `lvl1pPr`; clamped to 0…8), (3)
  `<a:p><a:pPr><a:defRPr>` (previously ignored — this was the real
  fidelity gap on themed templates, since SpreadJS-emitted shape XML
  always writes pPr/defRPr and only sometimes echoes it on the run
  rPr), (4) `<a:r><a:rPr>`. Same cascade applied to paragraph alignment
  via the new `pick_align` helper. The existing `apply_run_properties`
  was refactored into a field-wise `apply_run_fields` core + an
  `apply_default_run_properties` sibling so the cascade can chain
  `RunProperties` (`<a:rPr>`) and `DefaultRunProperties`
  (`<a:defRPr>`) — the SDK generates parallel types with identical
  fields but differently-named choice enums. Scope (v0): font size,
  bold, italic, underline-present, strike-present, solidFill color,
  latin font — same fields the rPr path already supports. Still TODO:
  `marL`/`indent`, line spacing, `spcBef`/`spcAft`, kerning, baseline,
  `<a:rPr u="none">` to clear inherited underline, and the bullet list.
  Locked in by `tests/fixtures/shapes/list-style-inheritance.xlsx`
  (4 panels: control / lstStyle-defPPr / lstStyle-lvl1pPr /
  cascade-precedence). Pixel-identical to all existing committed shape
  baselines (basic-autoshapes / textbox-wrap-align / style-refs-themed /
  connectors / groups-and-pictures all `bbox=None totaldiff=0` at
  scale 1) since the cascade only changes behavior when a run is
  missing fields. Divergence vs HSX: HSX doesn't honor
  `<a:lstStyle><a:defPPr>` at all and only partially honors
  `<a:lvl1pPr>` (size doesn't inherit), so on synthetic lstStyle-only
  fixtures we render more spec-correctly than HSX. On real
  Excel-authored workbooks (which always set rPr explicitly) we never
  render LESS than HSX.
- Dedicated z-order + grouping + picture regression baseline fixture
  `tests/fixtures/shapes/groups-and-pictures.xlsx` (+ `.hsx.png` /
  `.ours.png` + `build-groups-and-pictures.sh`). Five labelled panels
  exercising features already shipped in the extractor + renderer:
  (A) three overlapping shapes (rect / oval / diamond) at the same
  anchor emitted in that XML order — the diamond paints on top,
  locking in "z-order = XML traversal order"; (B) standalone pictures
  via both `sheet.pictures.add` (the loose `<xdr:pic>` path with
  `<a:xfrm>` zero-extent + anchor-derived geometry) and
  `sheet.shapes.addPictureShape` (the `<xdr:pic>` path with explicit
  `<a:xfrm>`); (C) a 3-shape group exercising `<xdr:grpSp>` flattening
  + `chOff/chExt`; (D) a label rect + nested picture grouped together
  — the "nested `<xdr:pic>` inside `<xdr:grpSp>`" extractor path in
  `shapes::visit_group`; (E) a nested group (outer `{ rect, inner
  { triangle, oval } }`) exercising the recursive `visit_shapes` walk
  + compounded `chOff/chExt`. Hsx and ours match pixel-for-pixel on
  every panel — this is a pure regression baseline, not a known gap.
  Drawings emitted: 4 `<xdr:grpSp>`, 4 `<xdr:pic>`, 10 `<xdr:sp>`.
- DrawingML `<xdr:cxnSp>` connectors (and bare `prstGeom=line`/`lineInv`
  shapes) now extract and render end-to-end. New schema fields on
  `ShapeNode`: `isConnector`, `flipH`, `flipV`, `lineDash`, `headEnd` /
  `tailEnd` (new `LineEnd` struct — `kind`/`w`/`len`), `adj1`. Extractor
  (`shapes::visit_connector`) walks `cxnSp` at root and inside groups,
  reading `<a:xfrm flipH/flipV>`, `<a:prstDash val="…"/>`,
  `<a:headEnd>/<a:tailEnd>` (type / `w` / `len` enums), and
  `<a:avLst>/<a:gd name="adj1" fmla="val …"/>`. `charts.rs` wires
  `XdrCxnSp` through `twoCellAnchor` and `oneCellAnchor`. Renderer
  (`shape.ts::drawConnector`) paints straight connectors,
  `bentConnector3` Z-routes (with horizontal vs vertical bend chosen
  by the dominant axis), and `line`/`lineInv` diagonals — honoring
  flips, dash patterns scaled to stroke width
  (`dash`/`dot`/`dashDot`/`lgDash`/`sysDash`/…), and five OOXML
  arrowhead kinds (triangle / stealth / diamond / oval / open `arrow`)
  whose size scales from the `w`/`len` enum + stroke width.
  `anchorToRect` (`grid.ts`) relaxed so axis-degenerate connector
  anchors (horizontal lines with `h==0`, vertical with `w==0`) survive
  layout instead of being filtered as zero-area drawings. Locked in by
  `tests/fixtures/shapes/connectors.xlsx` (all five connectors now
  visible against the HSX reference). Bonus side-effect: the
  `prstGeom=line` diagonal in `tests/fixtures/shapes/basic-autoshapes.xlsx`
  is no longer painted as a blue rect.
- DrawingML shape text now honors `<a:bodyPr lIns/tIns/rIns/bIns/>`
  insets. New schema field `ShapeNode.textInsetsEmu` (length-4 EMU
  vector); extractor reads `BodyProperties.{left,top,right,bottom}_inset`
  and back-fills missing slots with the ECMA-376 §21.1.2.1.1 defaults
  (91440 / 45720 / 91440 / 45720 EMU). Renderer (`shape.ts::drawShapeText`)
  applies them at `PX_PER_EMU`, replacing the old 4%-of-shape magic
  margin. Visible fix: text inside narrow autoshapes (triangle / chevron /
  hexagon / flowchartDecision) no longer fragments into single-character
  vertical strips.
- Initial DrawingML shape fixture corpus under `tests/fixtures/shapes/`,
  each shipping the `.xlsx` plus committed `.hsx.png` (ground truth) and
  `.ours.png` (current baseline):
  - `basic-autoshapes.xlsx` — 19 `prstGeom` presets (rect, roundRect,
    oval, triangle, diamond, 5 block arrows, chevron, pentagon, hexagon,
    star5, two flowchart symbols, line, lineInverse) plus 30°/90° rotated
    copies. Exercises preset dispatch, text label paths, z-order, and
    `xfrm@rot`.
  - `textbox-wrap-align.xlsx` — 4×4 grid covering h-alignment
    (`l`/`ctr`/`r`/`just`), vertical anchor (`t`/`ctr`/`b`), wrap
    (`square`/`none`), and four inset variants (default / tight / loose /
    asymmetric). Built via `hsx eval` + a Python zip-patch
    (`_patch_textbox.py`) that splices in attrs SpreadJS won't emit
    (`algn="just"`, `wrap="none"`, explicit `lIns`/`tIns`/`rIns`/`bIns`).
  - `connectors.xlsx` — four labelled node boxes with five real
    `<xdr:cxnSp>` connectors between them (straight, straight-diagonal
    with arrowhead, elbow with both-end arrowheads, dashed thick red
    straight, vertical elbow). Today's renderer paints zero of them —
    this is the wedge fixture for the upcoming connector milestone
    tracked in `docs/parity-shapes.md`. Bonus: the node boxes carry
    `<xdr:style>` blocks with `lnRef`/`fillRef`/`fontRef`/`effectRef`,
    so this fixture also exercises the upcoming style-refs resolver.

- Browser previewer now follows in-workbook hyperlinks like Excel: clicking
  worksheet navigation buttons switches sheets, selects/scrolls to the target
  cell, and resolves bare workbook/sheet defined-name targets such as `Top`,
  `HowTo`, and `Exer1`. External hyperlinks still open in a new tab.
- Worksheet-level AutoFilter chrome: non-table `<autoFilter ref="...">`
  ranges now surface as `Sheet.autoFilterRange` and paint header dropdown
  chevrons. Saved filtered results collapse via Excel's serialized row
  `hidden` flags; fixture: `tests/fixtures/tables/autofilter-hidden-rows.xlsx`.

### Fixed

- DrawingML preset names now come from the SDK's `as_xml_str()` instead of
  Rust enum variant debug names, so `roundRect`, `lineInv`, `homePlate`,
  `hexagon`, `star5`, `leftRightArrow`, `flowChartDecision`, etc. actually
  reach the renderer instead of being seen as `RoundRectangle` /
  `LineInverse` / … and falling through to a plain rect.
  (`crates/xlcore-export/src/shapes.rs::preset_geom_name`.)
- `shape.ts::pathForPreset` gained paths for the presets that previously
  fell back to rect: `roundRect` (now actually hit), `chevron`,
  `homePlate` / `pentagon`, `hexagon` / `octagon`, `star4` / `star5` /
  `star6` / `star8`, `leftRightArrow`, and `flowChartDecision`
  (aliased to diamond). New helpers `polygonPath`, `starPath`,
  `leftRightArrowPath`.
- Shape text now uses a per-preset text rect (`presetTextRect`) for
  non-rect shapes (triangle, diamond / flowChartDecision, ellipse,
  chevron, pentagon, hexagon, star, the four block arrows) so labels
  sit inside the painted region instead of overhanging it.
- `wrapParagraph` now falls back to char-by-char breaking when a single
  non-space token exceeds the line width, matching Office's wrap inside
  narrow shapes (chevron, hexagon, triangle, decision).
- Locked in by regenerated `tests/fixtures/shapes/*.ours.png` baselines
  (`basic-autoshapes`, `textbox-wrap-align`, `connectors`,
  `style-refs-themed`).
- Corrected explicit OOXML column-width conversion so authored widths do not
  get an extra 5px of padding. This removes accumulated horizontal drift in
  button-heavy / instruction-style worksheets.
- Fixed centered/right-aligned text overflow: the clip region may still expand
  into empty neighboring cells, but alignment remains anchored to the source
  cell/box rather than the expanded overflow band.
- Centered single-line button labels with literal leading/trailing spaces now
  paint using the visible label text, matching Excel-style navigation buttons
  more closely.

## [0.0.7] - 2026-05-17

### Added

- chartEx (`cx:`) `regionMap` ("Filled Map") painter
  (`chartExRegionMap.ts::drawRegionMapChartEx`). Bring-your-own world
  geometry: Natural Earth 110m admin_0 countries, slimmed +
  2-decimal-rounded into `packages/xlsx-preview/src/world110m.ts`
  (~170KB; regeneration snippet in the painter file header). The
  Bing-encoded `<cx:binary>` geoCache blobs Excel ships are
  deliberately ignored. Three pieces:
  - **Rust extractor** (`crates/xlcore-export/src/charts.rs`):
    (1) `parse_series_data` accepts `<cx:numDim type="colorVal">`
    alongside `val` / `size`; (2) `extract_chart_ex` picks the
    first non-`hidden="1"` series for `regionMap` layouts (Excel
    ships up to 4 alternate-preset series, only the last is
    visible); (3) new `extract_region_map_colors` parses
    `<cx:valueColors>` 2- or 3-stop palettes, resolving
    `<a:srgbClr>` literals + `<a:schemeClr>` theme refs (with
    modifier-chain support reused from `apply_color_modifiers`)
    into `cx_region_map_{min,mid,max}_color`.
  - **Schema**: `Chart` gains three optional fields
    (`cx_region_map_{min,mid,max}_color`). TS bindings
    regenerated via `scripts/regen-schema.sh`.
  - **TS renderer**: equirectangular projection with 1:1 lon/lat
    aspect; lat clamped to `[-58, 84]` so the world fills the
    rect; country-name lookup over NAME / NAME_LONG / ISO_A2 /
    ISO_A3 plus a small alias table (USA, UK, UAE, DRC, Czechia,
    Burma → Myanmar, Côte d'Ivoire, ...); palette honors authored
    3-stop diverging (e.g. blue→red→green) or 2-stop linear from
    the schema, falling back to a near-white → accent1 sequential
    ramp when no `<cx:valueColors>` was authored; gradient legend
    bar on the right with min/max labels; unmatched countries
    paint a neutral gray base layer. hsx falls back to a
    clustered column chart for this layout, so xlsx-preview now
    wins it outright. Fixture:
    `tests/fixtures/charts/chart-regionmap-chartex.xlsx` (covers
    both 2-color sequential and 3-color diverging palettes via
    its two sheets).
- DrawingML shape parity: word-wrap inside shape text bodies +
  nested pictures inside group shapes. Previously the shape painter
  emitted single-line runs that overflowed the box on anything
  longer than a step number, and `<xdr:pic>` children of
  `<xdr:grpSp>` were silently dropped — the Microsoft Map Chart
  template's NOTE paragraph ran off-right and the Maps-ribbon /
  `+`-button / arrow / columns-collapsed thumbnails inside its
  grouped callouts were missing entirely. Three pieces:
  - **Rust schema**: `ShapeNode` gains `text_wrap` (from
    `<a:bodyPr wrap="square|none"/>`), `image_data_uri`, and
    `image_src_rect` (4-int `<a:srcRect l t r b/>` crop in 1/1000
    percent of the source image).
  - **Rust extractor** (`shapes.rs`): new `visit_picture` arm in
    the group walker dereferences the picture's `r:embed` through
    a pre-built `rid → data:` URI map (constructed once per
    drawing in `charts.rs::extract`) and emits a leaf `ShapeNode`
    with the data URI plus the optional crop array. Top-level
    pictures still route through `AnchorTarget::Image`. Also
    surfaces `<a:bodyPr wrap="...">` via a new `body_wrap_token`
    helper.
  - **TS renderer**: new `imageCache.ts` extracted from
    `drawings.ts` so `shape.ts` can share the decoded-image cache;
    `drawShapeNode` dispatches image-bearing nodes to a new
    `drawShapeImage` (honors `srcRect` via the 9-arg
    `drawImage(s, sx, sy, sw, sh, dx, dy, dw, dh)` form);
    `drawShapeText` rewritten with proper paragraph word-wrap —
    tokenizes runs into `\S+\s*|\s+` atoms, measures with the
    active font, breaks atoms that would overflow inner width,
    preserves hard `\n` breaks, vertically anchors the wrapped
    block via `textAnchor`, trims trailing whitespace for
    center/right alignment. Wrap policy from `node.textWrap`:
    `square` (Excel default, absent attr) wraps; `none` lets text
    run on. `preloadDrawingImages` walks shape nodes too so the
    Node `renderToPng` path sees embedded thumbnails on first
    paint.
  - Verified on the existing
    `tests/fixtures/charts/chart-regionmap-chartex.xlsx` fixture
    (no new fixture needed — the Microsoft Map Chart template
    already exercises every code path). `docs/PARITY.md` Shapes
    row updated; remaining deferred items are gradient/blip/
    pattern shape fills, `<xdr:cxnSp>` connectors, and `avLst`
    adjust-value overrides on preset arrows (none triggered by
    current fixtures).

- Resolved custom `<tableStyles>` definitions. Previously the
  renderer only understood Excel's built-in style names
  (`TableStyleMedium2`, etc.) and inferred the accent color from the
  trailing digit; workbooks authored with a custom-named style — e.g.
  Microsoft's `Excel_TipsTableStyle` from the public Map Chart
  template — fell back to accent1 (blue) regardless of what the style
  actually pointed at, so a green-themed header rendered blue. Three
  pieces:
  - **Rust schema**: new `WorkbookLayout.tableStyles:
    Vec<CustomTableStyle>`. Each entry carries the style `name` plus
    `dxfId` references for the bands we paint (`wholeTable`,
    `headerRow`, `totalRow`, `firstRowStripe`, `secondRowStripe`,
    `firstColumn`, `lastColumn`). Bands we don't render yet (column
    stripes, subtotal rows, page-field cells) drop on the floor; add
    fields as the renderer grows.
  - **Rust extractor**: new `extract_table_styles` in `styles.rs`
    walks `<x:tableStyles>/<tableStyle>/<tableStyleElement>` and
    populates the named slots. Wired into `lib.rs` alongside
    `extract_dxfs`.
  - **TS renderer**: `computeTableState` now takes the workbook layout
    and resolves custom styles by name. Resolution order is (1) custom
    `<tableStyles>` lookup → dxf overlay via the existing `cfDxfs`
    pipeline, (2) built-in name heuristic fallback. Each band falls
    back independently — a custom style that only defines `headerRow`
    still gets synthesized row stripes. New helpers: `mergeDxf`
    stacks `wholeTable` underneath the band-specific overlay per
    ECMA-376 §18.8.40.
  - Fixture: `tests/fixtures/charts/chart-regionmap-chartex.xlsx`
    (slimmed copy of Microsoft's public "Map Chart samples.xlsx";
    two ~19MB `<cx:binary>` Bing geoCache blobs stripped — our
    renderer doesn't consume them). The fixture is primarily there
    to unblock the chartEx regionMap painter (still TODO) but the
    table-header bug surfaced as collateral while staging it.
  - Not yet honored: per-table direct overrides on `<table>` itself
    (`headerRowDxfId=""`, `dataDxfId=""`). These stack on top of the
    table style and would be a small follow-up.

- Surfaced a `regionMap` chartEx fixture. Microsoft's public "Map
  Chart samples.xlsx" template, slimmed from 40MB → 77KB by
  stripping the two `<cx:binary>` Bing geoCache blobs (our pipeline
  doesn't decode Bing's proprietary polygon encoding; the geometry
  will come from an embedded world-countries dataset when we ship
  the painter). The extractor already routes the layout through
  `drawChartEx` with `cx_layout="regionMap"`, but the painter still
  falls through to `drawPlaceholderPlot`. Notably hsx also falls back
  here — it renders the regionMap as a clustered column chart, not
  as a real map — so a future choropleth painter would beat hsx on
  this layout, not just match it. Fixture lives at
  `tests/fixtures/charts/chart-regionmap-chartex.xlsx`; painter design
  notes in `docs/parity-charts.md` priority #8.

- Added chartEx (`cx:` namespace) histogram / pareto / boxWhisker
  painters. All three are clear wins over hsx, which mis-renders each
  of them (histogram as raw bars, pareto as duplicated clustered
  columns, boxWhisker as a clustered column chart). Three pieces:
  - **Pre-parse normalization**: `xmlns_normalize` now rewrites
    `<cx:axisId val="N"/>` (the attribute form Excel desktop emits in
    chartEx parts to bind series to primary/secondary axes) into the
    `<cx:axisId>N</cx:axisId>` text-child form ooxmlsdk's chartEx
    schema expects. Without this, the pareto fixture crashes the
    chartEx parse entirely (`invalid field 'cx_axis_id' while parsing
    Series: ""`).
  - **Rust extractor**: `extract_chart_ex` now walks every
    `<cx:series>` (not just the first) and detects three multi-series
    / layoutPr-flagged compositions: any `paretoLine` companion
    promotes the chart to `cxLayout="pareto"`; an all-`boxWhisker`
    series list becomes `cxLayout="boxWhisker"`; a single
    `clusteredColumn` series whose `<cx:layoutPr>` carries
    `<cx:binning>` becomes `cxLayout="histogram"`.
  - **TS renderer**: new `chartExStats.ts` module (carved out of
    `chartEx.ts` to stay under the per-file LoC budget).
    `drawChartEx` dispatches on the new `cxLayout` values:
    - **Histogram**: Sturges bin count (`ceil(log2 n) + 1`),
      width rounded up to a nice `1/2/5 × 10^k` number so labels
      read as 10/20/50 rather than 9.7-and-change. Bars touch
      (`gapWidth=0`); right-closed `(low, high]` bin labels with the
      leftmost bin shown as `[low, high]` to flag its left-closed
      corner.
    - **Pareto**: primary `clusteredColumn` bars on the left value
      axis (accent1) plus a cumulative-% line on a synthesized right
      0–100% axis (accent2). The line series carries no own data in
      OOXML — cumulative % is computed from the primary series's
      values at render time, with the first point anchored at the
      origin so the line visually starts from the axis baseline.
    - **boxWhisker**: per-series quartiles computed with
      `QUARTILE.EXC` semantics (the chartEx default
      `quartileMethod="exclusive"`), 1.5×IQR whisker fences, outlier
      dots, median rule, and an × mean marker (default-on for
      chartEx). Each series renders as one vertical box centered in
      its slot with the series name as the category label.
  - Fixtures: `chart-{histogram,pareto,boxwhisker}-chartex.xlsx`
    (Excel-desktop-authored; SpreadJS round-trip is unreliable for
    these three layouts — see `build-chartex.sh`).

- Added chartEx (`cx:` namespace) funnel / treemap / sunburst painters
  alongside the existing waterfall arm. Three pieces:
  - **Rust extractor**: `extract_chart_ex` now accepts
    `<cx:numDim type="size">` alongside `type="val"` (treemap /
    sunburst encode rectangle / ring area in `size`, not `val`).
  - **Chart-ref resolver**: multi-column `categories_ref` ranges
    (e.g. `Sheet1!$A$2:$B$10` for a region→country hierarchy) are
    materialized into a new `cxCategoryLevels: string[][]` schema
    field — one inner array per nesting level, parallel to the
    values vector. The legacy 1D `categories` field gets the
    innermost (leaf) column for backward compat.
  - **TS renderer**: new `chartEx.ts` module (carved out of
    `chartAdvanced.ts` once chartEx surface area passed the per-file
    LOC budget; `chartStock.ts` likewise split). `drawChartEx`
    dispatches on `cxLayout`:
    - **Funnel**: center-aligned horizontal bars, widths scaled to
      the max value, per-bar value labels (suppressed when they'd
      overflow), category labels in a left gutter.
    - **Treemap**: squarified layout (Bruls/Huijsen/van Wijk 2000).
      Hierarchical mode groups leaves by `cxCategoryLevels[0]`,
      lays out parents across the full plot, then squarifies
      children inside each parent rect. Each branch gets one theme
      accent color; children share parent color, separated by white
      borders. Parent labels sit in the top-left of each group rect.
    - **Sunburst**: ring-per-level polar layout (innermost ring =
      level 0). DFS keeps sibling wedges angularly contiguous; per-
      branch theme accent with innermost-ring darken; tangentially-
      rotated slice labels with overflow suppression.
  - `chart.ts` suppresses the trivial single-series legend
    (`"Count"` / `"GDP"` / `"Sales"`) for these three layouts —
    Excel/hsx hide the legend too. Waterfall's synthetic three-swatch
    legend (Increase / Decrease / Total) is unchanged.
  - Fixtures: `tests/fixtures/charts/chart-{funnel,treemap,sunburst}-
    chartex.xlsx`, authored via SpreadJS (`hsx eval`) per
    `tests/fixtures/charts/build-chartex.sh`. ChartEx pareto /
    boxWhisker / clusteredColumn (histogram) / regionMap still need
    Excel-desktop authoring (SpreadJS export round-trip is unreliable
    for those four — missing `<cx:axis>` blocks, no auto-binning,
    degenerate render-as-cluster). See `docs/parity-charts.md`.
- Added chartEx (`cx:` namespace, Office 2016+) waterfall support —
  previously the largest remaining chart-parity gap. End-to-end pipeline:
  - **Rust I/O**: enabled ooxmlsdk's `mce` feature and added a textual
    `<mc:AlternateContent>` unfold in `xlcore-io::xmlns_normalize` for
    drawing parts (Excel always wraps chartEx graphic frames in MC for
    old-Excel fallback, and ooxmlsdk's typed `two_cell_anchor_choice`
    never sees MC contents otherwise).
  - **Rust extractor**: new `xlcore-export::charts::extract_chart_ex`
    resolves `drawings_part.extended_chart_parts()` and surfaces
    `chart_type="chartex"`, `cxLayout` (waterfall / funnel / treemap /
    sunburst / paretoLine / boxWhisker / regionMap), and
    `cxSubtotalIndices`.
  - **Chart-ref resolver**: dereferences Excel's `_xlchart.vN.X`
    indirection — chartEx bodies use opaque alias formulas
    (`<cx:f>_xlchart.v1.4</cx:f>`) that resolve through
    `workbook.xml`'s `<definedName hidden="1">Sheet1!$A$2:$A$7
    </definedName>` entries.
  - **TS renderer**: new `chartAdvanced.ts::drawChartEx` dispatches on
    `cxLayout`; waterfall painter draws cumulative bars (subtotal bars
    are absolute from the floor), dashed connectors between consecutive
    bars, per-bar value labels, and a synthetic 3-swatch legend
    (Increase / Decrease / Total) keyed to the workbook theme accents
    (accent1 / accent2 / accent3, matching the chartEx color-style
    part's default `cycle id="10"` palette). Other layouts still fall
    through to the placeholder plot pending fixtures.
  - Fixture: `tests/fixtures/charts/chart-waterfall-chartex.xlsx`
    (Excel-authored). hsx renders waterfall similarly; xlsx-preview was
    previously empty-bbox.
- Added a time-period conditional-formatting fixture plus a schema-drift CI
  guard, and documented the local schema regeneration / PNG fixture
  comparison workflow.
- Added `radarChart` support (ECMA-376 §21.2.2.155 / §21.2.2.176). New polar
  painter in `chartAdvanced.ts::drawRadarChart` honors `radarStyle`
  (`standard` / `marker` / `filled`) with polygon gridlines, per-spoke
  category labels, and value-axis tick labels along the top spoke. Per-series
  `<c:marker><c:symbol val="none"/>` still overrides marker visibility.
  Fixtures: `tests/fixtures/charts/chart-radar-{standard,marker,filled}.xlsx`.
- Added `stockChart` support (ECMA-376 §21.2.2.207). New painter in
  `chartAdvanced.ts::drawStockChart` infers the subtype from series count
  (3 → HLC, 4 → OHLC, 5 → VOHLC) and honors `<c:hiLowLines/>` (vertical mark
  from category low to high), `<c:upDownBars/>` (candlestick-style open→close
  rect; white-filled for up days, black-filled for down days), and
  `<c:dropLines/>`. Volume sub-plot stub for VOHLC carves off the bottom 22%
  of the plot rect. Legend swatches reflect what's actually painted: series
  with `markerSymbol === "none"` (hi-low envelope contributors) render a thin
  vertical bar in the hi-low ink color; series with markers render a colored
  dot. hsx (SpreadJS) currently renders stock charts as an empty plot, so
  xlsx-preview is the clear winner here. Fixtures:
  `tests/fixtures/charts/chart-stock-{hlc,ohlc}.xlsx`.

## [0.0.6] - 2026-05-16

### Fixed

- Avoid freezing/OOMing on sheets whose conditional-formatting ranges extend
  to full Excel row/column bounds (for example `XFD` / `1048576`) by scanning
  actual populated/numeric cells and clipping range expansion to the sheet's
  effective bounds.

## [0.0.5] - 2026-05-16

### Added

- Hidden / very-hidden sheets and tab colors (`Sheet.state`,
  `Sheet.tabColor`). `veryHidden` stays off the tab strip; `hidden`
  can be revealed with `PreviewerOptions.showHidden` / `?showHidden=1`.
  Fixture: `tests/fixtures/sheets/hidden-and-tabcolor.xlsx`.
- Node CLI `--all` skips hidden sheets; explicit `--sheet` targets still work.
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
- Added `<c:majorUnit>` extraction on primary + secondary value axes
  (`Chart.majorUnit` / `majorUnitSecondary`). When authored, the renderer
  steps ticks by exactly the authored unit and walks the cadence down to
  zero for positive-only data with no `<c:min>` (capped at 14 ticks to
  avoid pathological expansion), so workbooks pinning a `<c:max>` +
  `<c:majorUnit>` get Excel's authored cadence (e.g. 0/9/18/27/36/45)
  instead of niceTicks (10/20/30/40/45). Wired through `bar/column`,
  `line`, `area`, and `combo` painters.
- Added unit tests for `resolveAxisRange` with `majorUnit` cadence,
  including the walk-to-zero heuristic, forced-min anchoring, tick-count
  cap on tiny steps, and dataMin straddling zero.
- Added Rust unit tests for `theme_scheme_color` covering all twelve
  ECMA-376 §20.1.6.2 scheme slots (accents, bg/tx, lt/dk, hlink) plus
  workbook-theme overrides on the lt1/bg1 slot, and for
  `built_in_unit_default_label` /​ `built_in_unit_factor` consistency.

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

- Fixed scheme-color resolution on chart `<c:spPr>` and `<c:dPt>` blocks to
  handle every ECMA-376 §20.1.6.2 SchemeColor variant — not just
  `accent1..accent6`. `bg1`/`tx1`/`bg2`/`tx2`, `lt1`/`dk1`/`lt2`/`dk2`,
  `hlink`/`folHlink`, and the `windowText`/`window` system aliases now
  route through the workbook theme (with the ECMA-default `<a:clrMap>`
  fallback `bg1↔lt1`, `tx1↔dk1`, `bg2↔lt2`, `tx2↔dk2`). Fixes the
  "fake-waterfall" idiom where invisible stack segments are painted with
  `<a:schemeClr val="bg1"/>` (white-on-white) instead of `<a:noFill/>`;
  those segments used to inherit their parent series's accent color and
  break the floating-bar illusion. Refactored into a single shared
  `theme_scheme_color()` helper used by both fill and outline resolvers.
- Fixed chart title auto-generation from the series name. Per ECMA-376
  §21.2.2.211 + §21.2.2.4, when `<c:title>` is present without an explicit
  `<c:tx>` and `<c:autoTitleDeleted val="0"/>` (or the element is absent,
  which defaults to false) and the chart has exactly one series, Excel
  auto-uses the series name as the title; we used to render no title.
  `<c:autoTitleDeleted val="1"/>` continues to suppress.
- Fixed `<c:dispUnitsLbl>` default caption resolution. When the label
  element is present without an inner `<c:tx>` and the unit is a built-in
  (e.g. `<c:builtInUnit val="thousands"/>`), the extractor now falls back
  to the localized unit name ("Thousands", "Millions", … per
  `built_in_unit_default_label`) instead of dropping the caption. Excel
  paints this default even though the XML carries no text node.
- Fixed value-axis gridline rendering so gridlines only paint when authored
  and do not double-paint the zero line.
- Fixed chart data labels across bar, line, area, pie/doughnut, scatter, and
  combo renderers to respect per-point delete/text/format/position overrides.
- Fixed generated TypeScript schema exports for the new chart and data-label
  fields.
- Fixed bar, column, line, and area charts to clip series geometry to the
  plot rectangle when data exceeds a workbook-pinned `<c:scaling><c:max>`
  (or falls below `<c:min>`). Stacked column totals larger than the axis
  max, line strokes crossing an outlier, and area fills with a peak past
  the topmost gridline now match Excel and SpreadJS instead of painting
  past the plot frame. Added `chart-stacked-overflow-clip` and
  `chart-line-area-overflow-clip` regression fixtures.

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
