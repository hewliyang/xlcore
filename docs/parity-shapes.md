# Shape / DrawingML parity triage

Status of SpreadsheetML DrawingML shapes in `xlcore` vs Excel / `hsx`.

Legend: ✅ done · 🟡 partial · ❌ missing · n/a out of scope.

## Spec map

Primary ECMA-376 Part 1 clauses / schema areas:

| Area | Spec hook | Notes |
| --- | --- | --- |
| Spreadsheet drawing host | §20.5 `DrawingML - SpreadsheetML Drawing` | Worksheet drawing part (`xl/drawings/drawing*.xml`) positions DrawingML objects on sheets. |
| Anchor types | §20.5.2.33 `twoCellAnchor`, §20.5.2.1 `absoluteAnchor`; schema `CT_OneCellAnchor` | `twoCellAnchor` moves/resizes with cells (`editAs` default `twoCell`); `oneCellAnchor` uses start marker + EMU extent; `absoluteAnchor` uses absolute EMU pos + extent. |
| Object choices | schema `EG_ObjectChoices` | `sp`, `grpSp`, `graphicFrame`, `cxnSp`, `pic`, `contentPart`. |
| Shape | §20.5.2.29 `sp`; schema `CT_Shape` | `nvSpPr`, `spPr`, optional `style`, optional `txBody`, attrs `macro`, `textlink`, `fLocksText`, `fPublished`. |
| Group shape | §20.5.2.17 `grpSp`, §20.5.2.18 `grpSpPr`; schema `CT_GroupShape` | Groups can nest `sp`, `grpSp`, `graphicFrame`, `cxnSp`, `pic`; `xfrm/off/ext/chOff/chExt` maps child logical coordinates. |
| Connectors | §20.5.2.19 `nvCxnSpPr`; schema `CT_Connector` | Connector shape with normal shape properties and connection non-visual props. |
| Shape properties | §20.5.2.30 `spPr`; main schema `CT_ShapeProperties` | `xfrm`, geometry, fill, line, effects, 3D. |
| Geometry | §20.1.9.18 `prstGeom`, §20.1.10.56 `ST_ShapeType` | 187 preset shape types in strict schema; `custGeom` supports explicit paths. `OfficeOpenXML-DrawingMLGeometries.zip` contains informative preset path definitions. |
| Fills / lines | §20.1.8.54 `solidFill`; schema `EG_FillProperties`, `CT_LineProperties` | Fill choices: `noFill`, `solidFill`, `gradFill`, `blipFill`, `pattFill`, `grpFill`. Line has fill, dash, join, head/tail ends, width/cap/compound/alignment. |
| Text body | §20.5.2.34 `txBody`; main schema `CT_TextBody`, `CT_TextBodyProperties` | `bodyPr`, optional list style, one or more paragraphs; `bodyPr` covers wrap (`none`/`square`), margins, rotation, vertical text, overflow, autofit, anchor. |

Useful commands used for this audit:

```bash
cd ecma-376
./ecma show p1-20-5-drawingml-spreadsheetml-drawing
./ecma show p1-20-5-2-29-sp-shape
./ecma search prstGeom
./ecma search ST_ShapeType
```

## Current implementation snapshot

| Layer | Status | Files |
| --- | --- | --- |
| Extraction | 🟡 | `crates/xlcore-export/src/charts.rs` surfaces top-level `xdr:sp`, `xdr:grpSp`, `xdr:cxnSp` from `twoCellAnchor` / `oneCellAnchor`. `absoluteAnchor` + `contentPart` still ignored. |
| Shape tree | 🟡 | `crates/xlcore-export/src/shapes.rs` (+ `shapes_style.rs`, `shapes_text.rs`) flattens `sp` / nested `grpSp` / `cxnSp`; maps group `xfrm/off/ext/chOff/chExt`; nested `xdr:pic` in groups becomes image nodes. |
| Schema | 🟡 | JSON model is painter-oriented (`Shape { nodes }`), not a full DrawingML AST. Good for preview, not for round-trip editing. |
| Rendering | 🟡 | `packages/xlsx-preview/src/shape.ts` + `shapePaths.ts`. Small preset subset, solid fills, basic outlines, text, rotation, connectors, nested pictures. Unknown presets fall back to rectangle. |
| Fixture corpus | ✅ | `tests/fixtures/shapes/`: `basic-autoshapes.xlsx`, `textbox-wrap-align.xlsx`, `connectors.xlsx`, `style-refs-themed.xlsx`, `groups-and-pictures.xlsx`, `list-style-inheritance.xlsx`, `avlst-adjusts.xlsx`, plus three EPPlus-authored gap fixtures — `gradient-fills.xlsx`, `outer-shadow.xlsx`, `shape-flips.xlsx`, `style-refs-matrix.xlsx`. Each with `.hsx.png` ground truth + `.ours.png` baseline. The EPPlus path lives at `tests/fixtures/shapes/dotnet-builder/FixtureBuilder/` for features SpreadJS's public API can't author (gradients, shape effects, non-connector flips). |

## Known v0 shortcuts

Consolidated list of deliberate carve-outs. Most landed under e-007. Each item is shipped as far as the bullet describes; the rest is the v0 cheat.

*(2026-05-19: items #7 and the new "missing xfrm / xfrm without off+ext" extractor bug were resolved while adding the EPPlus fixture corpus — the painter now honors `flipH`/`flipV` on every shape kind, and the extractor falls back to anchor geometry when xfrm is absent or partial. Both were exposed by `shapes/shape-flips.xlsx`.)*

*(2026-05-20: P1 #6 ("`avLst` for `roundRect` / arrows / callouts") shipped — the cardinal-arrow and `roundRect` arms. `pathForPreset` in `packages/xlsx-preview/src/shapePaths.ts` now reads `node.adj1` / `node.adj2` for: `roundRect` (corner offset = `min(w,h) * clamp(adj1, 0..50000) / 100000`, spec-default 16667 = ~16.667%), the four cardinal arrows (`leftArrow`/`rightArrow`/`upArrow`/`downArrow`: `adj1` = tail thickness as fraction of cross-axis, `adj2` = head length as fraction of along-axis; both clamped to `[0,1]`), and `leftRightArrow` (`adj1` = tail height fraction of `h`, `adj2` = per-side head length fraction of `w/2`, head capped at `w/2` so the two heads never overlap past centre). Defaults were tuned to match the prior hardcoded values modulo the `roundRect` 0.16 → spec-exact 0.16667 nudge (sub-pixel; baselines refreshed). Locked in by `shapes/avlst-adjusts.xlsx`. The `roundRect` / arrow rows pre-fix collapsed all four adj-sweep columns to the same picture; post-fix they render the full sweep matching HSX direction. Callouts + arc/donut + stars still ignore `avLst` — see Known v0 shortcuts #3.)*

*(2026-05-20: P1 #7 ("Text autofit") shipped. `<a:bodyPr>`'s autofit choice decodes through a new `body_autofit` helper in `shapes_text.rs` into three new `ShapeNode` fields (`textAutofit`, `textFontScale`, `textLineSpaceReduction`); `a:noAutofit` / absent choice both resolve to `None` (no scaling). The painter (`drawShapeText` in `shape.ts`) multiplies every run's font size by `fontScale/100000` and reduces every line height by `1 - lnSpcReduction/100000`; `spAutoFit` is recorded for round-trip but doesn't trigger paint-time work because by spec the shape `ext` already reflects the author-time auto-fit. Locked in by `shapes/text-autofit.xlsx`. Like `outer-shadow.xlsx`, the `.hsx.png` ground truth **intentionally diverges** from `.ours.png` — SpreadJS ignores `normAutofit` and renders all six boxes at unscaled 11pt; we render the spec-correct scaled sizes.)*

*(2026-05-20: P1 #8 ("Text rotation / vertical text") shipped. `<a:bodyPr rot>` and `<a:bodyPr vert>` now extract through two new `TextBodyOut` fields (`rotation`, `vert`) into two new `ShapeNode` fields (`textRotation` in 1/60000 deg — same units as `<a:xfrm rot>` — and `textVert` as the spec token). The painter (`drawShapeText` in `packages/xlsx-preview/src/shape.ts`) collapses both into a single effective text-body angle, then `ctx.save()` / `translate` to the inner-rect center / `rotate(thetaRad)` and recomputes layout in the rotated frame; perpendicular variants (|effDeg| ≈ 90° mod 180°) additionally swap layout width/height so wrap + anchor still operate along the reading direction. Every `vert` variant (`vert` / `wordArtVert` / `eaVert` / `mongolianVert` → +90°; `vert270` / `wordArtVertRtl` → -90°) collapses onto a flat ±90° — readable and visually correct for the labels Excel actually produces. Locked in by `shapes/text-rotation-vert.xlsx`. Like `outer-shadow.xlsx` and `text-autofit.xlsx`, the `.hsx.png` ground truth **intentionally diverges** from `.ours.png` — SpreadJS ignores `<a:bodyPr rot>` (rows 0+1 in the fixture render flat in HSX, rotated correctly in ours) and collapses `vert270` onto the same orientation as `vert`.)*

*(2026-05-20: P1 #10 (`<a:blipFill>` on `<xdr:sp>/<xdr:spPr>` + `asvg:svgBlip` sidecar) shipped. The `<xdr:sp>` extractor in `crates/xlcore-export/src/shapes.rs` now decodes `ShapePropertiesChoice2::ABlipFill` through a new `blip_fill` helper into a `ShapeBlipFill` (`data_uri`, `src_rect`, `kind`) on `ShapeNode.fill_blip`. The embed-id is resolved through the same `ImageUriResolver` already plumbed for `<xdr:pic>`, and when the blip carries `<a:extLst><a:ext uri="{96DAC541-...}"><asvg:svgBlip r:embed="..."/></a:ext></a:extLst>` the SVG sidecar's embed wins (vector → crisper at scale). `<a:srcRect>` decodes to the same 1/100000 `[l,t,r,b]` model as the existing picture-crop path; all-zero rects are dropped. The painter (`drawShape` in `packages/xlsx-preview/src/shape.ts`) adds a new `drawBlipFillImage` helper: when `fillBlip` is set on a non-`<xdr:pic>` shape, it traces the preset path, `ctx.clip()`s to it, draws the resolved image (raster or SVG) stretched into the bbox honoring `srcRect`, then re-traces the path so the outline stroke below still picks up the geometry. `tile` is parsed but painted as `stretch` for v0 — no real workbooks we've seen use tile on shape blip fills. Locked in by `shapes/blip-fills.xlsx` (EPPlus author + post-save XML splice for `asvg:svgBlip` + svg part + drawing rels + `[Content_Types].xml` `Default Extension=svg`); like `outer-shadow.xlsx` / `text-autofit.xlsx` / `text-rotation-vert.xlsx` / `text-overflow.xlsx`, the `.hsx.png` ground truth **intentionally diverges** from `.ours.png` on panel b5 — SpreadJS drops the SVG sidecar and uses the raster fallback (checkerboard), we paint the spec-correct SVG (blue rect + yellow circle).)*

*(2026-05-20: P1 #9 (`vertOverflow="clip"` + `horzOverflow`) shipped. `<a:bodyPr>` `vertOverflow` (`overflow`/`clip`/`ellipsis`) and `horzOverflow` (`overflow`/`clip`) now extract through two new `TextBodyOut` fields (`vert_overflow`, `horz_overflow`) into two new `ShapeNode` fields (`textVertOverflow`, `textHorzOverflow`) via `body_vert_overflow_token` / `body_horz_overflow_token` in `crates/xlcore-export/src/shapes_text.rs`. The painter (`drawShapeText` in `packages/xlsx-preview/src/shape.ts`) now treats the spec default `overflow` as "paint every line, don't clip" — previously the renderer hardcoded a clip-like rule that broke when a line's top moved past the body rect. When either `vertOverflow != overflow` or `horzOverflow = clip`, the painter `ctx.clip()`s to the inner rect before drawing. `vertOverflow="ellipsis"` additionally rewrites the last fully-visible line's tail run as `…` (with character-by-character truncation until the trailing ellipsis fits `innerW`). Locked in by `shapes/text-overflow.xlsx` — like `outer-shadow.xlsx` / `text-autofit.xlsx` / `text-rotation-vert.xlsx`, the `.hsx.png` ground truth **intentionally diverges** from `.ours.png` because SpreadJS drops both overflow attrs. Also fixed an unrelated bug exposed by this work: `body_wrap_token` had been pattern-matching the Debug repr of `TextWrappingValues` for the string `"None_"` (no such variant) so `<a:bodyPr wrap="none">` always fell through to the default `square` wrap; replaced with a direct `match` against the enum. Same shape was triggering this for connectors / lines / textboxes ever since wrap extraction landed, but no fixture exercised wrap=none until now.)*

*(2026-05-20: P1 #5 ("Preset dash + line cap/join on non-connector outlines") shipped. `<a:ln>` now extracts `cap` (flat/sq/rnd) via `line_cap_token`, join (round/bevel/miter) via `line_join_token`, and `prstDash` via the existing `line_dash_token` on the **non-connector** shape path too — the visit_shape extractor previously left `line_dash = None` and never touched cap/join. The painter (`drawShape` in `shape.ts`) now honors `node.lineCap` / `node.lineJoin` / `node.lineDash` when stroking; brace-like presets keep their forced `round` cap+join as a fallback only when no explicit value is set. The connector path falls back to the style-ref matrix walk for cap/join too, so themed line refs that declare `cap` / `join` (already extracted by `fmt_scheme::extract_line`) finally reach the canvas. Locked in by `shapes/line-cap-join-dash.xlsx`.)*

*(2026-05-20: P1 #4 ("Group rotation + body-rect-follows-flip") shipped. The extractor now composes a full 2D affine per group `<a:xfrm>` (including `rot`) and pushes the resulting accumulated rotation down to every child shape / picture / connector — children of a rotated `<xdr:grpSp>` rotate as a rigid body. The painter mirrors the preset text body rect for flipped shapes so labels on asymmetric presets (right-arrow, pentagon, callouts) sit over the visually-correct half. Locked in by `shapes/groups-rotated.xlsx` (rotation) and the text-position diff on `shapes/shape-flips.xlsx` baseline (flip body rect). Remaining nested-group rotation is approximate — see Known v0 shortcuts.)*

1. **`stCxn`/`endCxn` connection sites** — only the 4 cardinal sites (top/right/bottom/left center) are resolved against the target bbox. Enough for `rect`/`roundRect`/`ellipse` receivers (org-chart / SOTP). Skipped: preset-aware sites (chevron tip, star points, flowchart non-cardinal), custom `cxnLst` declared on the shape XML, multi-segment `bentConnector{2,4,5}` re-routing.
2. **Brace/bracket presets** — only `leftBrace` / `rightBrace` / `leftBracket` / `rightBracket`. Missing: `bracePair`, `bracketPair`, diagonal bracket variants.
3. **`avLst` adjust values** — `adj1`+`adj2` extracted on every shape; honored by the brace painter, `roundRect`, the four cardinal arrows, and `leftRightArrow`. Callouts, stars, arcs, chevron / pentagon point depth, and the rest of the long-tail presets still keep hardcoded defaults.
4. ~~**`vertOverflow`** — hardcoded DrawingML default `overflow` (line paints if its top is inside the body rect). Explicit `vertOverflow="clip"` and `horzOverflow` unmodeled.~~ **Shipped.** `<a:bodyPr>` `vertOverflow` / `horzOverflow` now extract through `textVertOverflow` / `textHorzOverflow` on every shape path. Painter treats the spec default `overflow` as no-clip (previously approximated as clip); `clip` and `ellipsis` push a `ctx.clip()` over the inner rect; `ellipsis` additionally rewrites the last fully-visible line's tail run with `…`.
5. **`lstStyle` cascade** — inherits only size / bold / italic / underline / strike / solidFill color / latin font. Ignores `marL`, `indent`, `lnSpc`, `spcBef`, `spcAft`, kerning, baseline, run-`u="none"`-as-disable-inherited, and the entire bullet list.
6. ~~**Style refs (`a:style`)**~~ — **shipped.** The matrix walk in `shapes_style::resolve_style_refs` now consults a parsed `FmtScheme` (`crates/xlcore-export/src/fmt_scheme.rs`) and substitutes `phClr` per-entry. `fillRef idx≥1` resolves to themed gradient/solid, `lnRef idx≥1` picks up width + dash (cap/join extracted but not yet consumed by the painter), and `effectRef idx≥1` resolves a themed `<a:outerShdw>`. Locked in by `shapes/style-refs-matrix.xlsx`.
7. ~~**`flipH/V`** — applied to connectors only. Non-connector shape flips ignored.~~ **Shipped.** Painter applies `ctx.scale(±1,±1)` around shape centre before geometry; unflips before text. Text body rect doesn't follow the flip yet (caption position is off on asymmetric presets like arrows), tracked as a follow-up under P1 #4.
8. ~~**`prstDash`** — extracted+rendered on connectors/lines only. Non-connector shape outlines don't read dash.~~ **Shipped.** `line_dash_token` is now wired into `visit_shape`; the painter calls `setLineDash(dashPattern(node.lineDash, …))` before stroking non-connector outlines. `custDash` still fully ignored.
9. ~~**Line cap/join** — connector painter hardcodes `cap=butt, join=miter`; brace painter forces `round`.~~ **Shipped.** `a:ln@cap` (flat/sq/rnd) extracts to `line_cap` and `a:round`/`a:bevel`/`a:miter` to `line_join` on every shape path; painter maps them onto `ctx.lineCap` / `ctx.lineJoin`. Brace presets keep `round` as a no-explicit-value fallback. Themed cap/join from the style-ref matrix walk also propagates now.
10. **`a:fld`** — handled as a cached-text run (we display the cached `<a:t>`). The `textlink` formula is not evaluated; harmless for preview because OOXML stores the latest evaluated value in the field.
11. **Nested group rotation** — each group level's rotation composes into a single 2D affine that maps child-coord-space to world; the child's accumulated rotation comes from `atan2(b, a)` of that affine. For a single rotated group with arbitrarily nested children this is exact. For *nested* `<xdr:grpSp>` where two or more levels each carry a non-zero `rot`, this is a sound approximation only when the two rotations share the same center; otherwise it under-shoots the off-axis displacement that the proper rotation-composition would induce. Real workbooks almost never nest rotated groups; revisit if a fixture shows divergence.
12. **Group `flipH`/`flipV`** — `<a:xfrm flipH="1"/>` on `<xdr:grpSpPr>` is read off (and would compose into the per-frame affine as a signed scale), but is not propagated to children today. Group rotation alone covers the vast majority of real workbooks; group flip would additionally need to flip each child's `flip_h` / `flip_v` flags and adjust text body rects through the cumulative chain. Tracked under P1 #4 follow-up.

## Parity matrix

### Anchoring / object model

| Feature | Extract | Render | Priority | Notes |
| --- | --- | --- | --- | --- |
| `twoCellAnchor` | ✅ | ✅ | P0 | `editAs` (twoCell/oneCell/absolute) not modeled; renderer uses the resolved anchor rect. Matters once agent edits insert rows/columns. |
| `oneCellAnchor` | ✅ | ✅ | P0 | Pixel-accurate via EMU extents. |
| `absoluteAnchor` | ❌ | ❌ | P1 | Absolute EMU `pos` + `ext`. Rare but easy to surface. |
| `clientData` flags | ❌ | n/a | P2 | `fLocksWithSheet`, `fPrintsWithSheet`. No preview impact; needed for round-trip. |
| `editAs` round-trip | ❌ | n/a | P1 | Pass-through is fine for preview; required once shape moves enter the agent mutation API. |
| Top-level `sp` | ✅ | 🟡 | P0 | Geometry/fill/text coverage partial — see geometry / fill / text sections. |
| Top-level `grpSp` | ✅ | 🟡 | P0 | Nested `sp` / `grpSp` / `pic` supported; nested `cxnSp` ✅; nested `graphicFrame` ignored. |
| Top-level `pic` | ✅ | ✅ | P0 | Image path; both `sheet.pictures.add` (no `xfrm`) and `sheet.shapes.addPictureShape` (explicit `xfrm`) covered. |
| Nested `pic` in groups | ✅ | ✅ | P0 | Incl. `<a:srcRect>` crop. |
| `cxnSp` connectors | ✅ | ✅ | P0 | Top-level + nested. Straight + `bentConnector3` with `adj1`; `line` / `lineInv` routed through connector painter. |
| Connection sites `stCxn`/`endCxn` | 🟡 | 🟡 | P1 | Cardinal-only — see shortcut #1. |
| Connection sites `cxnLst` (shape-declared) | ❌ | ❌ | P1 | Pairs with the `stCxn`/`endCxn` work for non-cardinal receivers. |
| `graphicFrame` in groups | ❌ | ❌ | P2 | Charts / diagrams / tables nested inside a group. |
| `contentPart` | ❌ | ❌ | P3 | Extension payload. |
| Non-visual props / alt text | ❌ | n/a | P2 | `cNvPr name/descr/title`, locks, hidden. |
| `cNvPr/hlinkClick` + `hlinkHover` | ❌ | ❌ | P1 | Click-the-shape hyperlinks. Reuse the workbook hyperlink event channel. |
| `macro`, `textlink` attrs | ❌ | ❌ | P2 | `textlink` binds shape text to a cell formula. |
| Adjust handles `ahLst` | ❌ | n/a | P3 | Authoring-time UI only. |
| Shape locks (`spLocks`, etc.) | ❌ | n/a | P3 | Edit-time gates. |
| Legacy VML drawings (`vmlDrawing*.vml`) | ❌ | ❌ | P1 | Comment indicators, form controls, pre-2007 autoshapes. Out of scope for pure DrawingML; `legacyDrawing` r:id is currently ignored. |
| Form controls / OLE (`<controls>`, `<oleObjects>`) | ❌ | ❌ | P2 | Anchored like drawings; static placeholder is enough for v0. |

### Transforms / z-order

| Feature | Extract | Render | Priority | Notes |
| --- | --- | --- | --- | --- |
| Shape offset / extent | ✅ | ✅ | P0 | `<a:xfrm>` is optional in DrawingML; `shape_world` / `connector_world` fall back to a unit-box outer (renderer normalises to anchor rect) when `<xdr:spPr>` has no `<a:xfrm>` at all, or when `<a:xfrm>` carries only `flipH`/`flipV`/`rot` attributes with no `<a:off>` + `<a:ext>` children. EPPlus, OpenXML SDK, and Excel itself for plain anchored shapes all emit one of these shapes. Locked in by `shapes/shape-flips.xlsx` (would extract 0/5 shapes pre-fix). |
| Group `chOff` / `chExt` mapping | ✅ | ✅ | P0 | |
| Shape rotation `xfrm@rot` | ✅ | ✅ | P0 | 1/60000 deg; rotated around center. |
| Group rotation | ✅ | ✅ | P1 | `<a:xfrm rot>` on `<xdr:grpSpPr>` now composes into a per-frame 2D affine (`Frame` in `shapes.rs`); children rotate as a rigid body. Single-level rotation is exact; nested rotated groups approximate (see Known v0 shortcuts #11). Locked in by `shapes/groups-rotated.xlsx`. |
| Flip H/V on shape `xfrm` | ✅ | ✅ | P1 | Geometry flips on both connector and non-connector paths. The painter applies `ctx.scale(±1, ±1)` around the shape centre before drawing the path; the text body rect is **also** mirrored within the shape bbox so labels on asymmetric presets (arrows, callouts, pentagons) sit over the visually-correct half. Glyphs themselves are not mirrored — by design, captions stay readable. Locked in by `shapes/shape-flips.xlsx`. |
| Z-order | ✅ | ✅ | P0 | Preserved from XML traversal order. |
| Clipping to group/shape | ❌ | ❌ | P2 | Flattened model does not clip children to group bounds. |
| `bwMode` | ❌ | ❌ | P3 | Rare in spreadsheets. |

### Geometry

| Feature | Extract | Render | Priority | Notes |
| --- | --- | --- | --- | --- |
| `prstGeom`: `rect`, `roundRect`, `ellipse`, `triangle`, `diamond` | ✅ | ✅ | P0 | `roundRect` reads `adj1` from `<a:avLst>` (radius = `min(w,h) * clamp(adj1, 0..50000)/100000`); spec-default 16667. |
| Basic block arrows (`leftArrow`/`rightArrow`/`upArrow`/`downArrow`) | ✅ | ✅ | P0 | `adj1` (tail thickness) + `adj2` (head length) honored; defaults match prior 50/50 hardcoded values. |
| Lines (`line` / `lineInv`) | ✅ | ✅ | P0 | Routed through connector painter; honors `flipH/V`, dash, arrowheads. |
| Common extras (`chevron`, `pentagon`, `hexagon`, `star5`, `leftRightArrow`) | ✅ | ✅ | P1 | Shipped in `abbebdd`. |
| Braces / brackets | 🟡 | 🟡 | P1 | See shortcut #2. |
| Long-tail presets | ❌ | ❌ | P1 | Spec lists 187. Big ones still missing: flowchart symbols, action buttons, callouts, stars beyond star5, arc/donut, plaque/bevel. |
| `avLst` adjust values | 🟡 | 🟡 | P1 | Brace family, `roundRect`, cardinal arrows, and `leftRightArrow` honor `adj1`/`adj2`. Callouts, stars, arcs, chevron/pentagon point depth still hardcoded. See shortcut #3. |
| `custGeom` paths | ❌ | ❌ | P2 | Requires DrawingML path interpreter (`moveTo`, `lnTo`, `arcTo`, bezier, close) and guide formulas. |
| Informative preset path corpus | n/a | n/a | P2 | `presetShapeDefinitions.xml` can seed preset rendering instead of hand-writing 187 shapes. |

### Fill / outline / effects

| Feature | Extract | Render | Priority | Notes |
| --- | --- | --- | --- | --- |
| `noFill`, `solidFill srgbClr`, `solidFill schemeClr` | ✅ | ✅ | P0 | Uses workbook theme + modifier resolver. |
| `solidFill` `prstClr`, `sysClr` | 🟡 | ✅ | P1 | Small preset table / `lastClr`; expand. |
| `scrgbClr`, `hslClr` | ❌ | ❌ | P2 | Already solved for cells/charts; reuse conversion. |
| Alpha / transparency | 🟡 | 🟡 | P1 | Some alpha parses via theme path; renderer does not generally model opacity on shapes. |
| Full color modifier set | 🟡 | 🟡 | P1 | §20.1.2.3. Cell-side resolver covers most; verify shape path uses it end-to-end. |
| Gradient fills `gradFill` | ✅ | ✅ | P1 | Linear (`lin@ang`) + path (`path@path` w/ `fillToRect`); `gsLst` stops resolved through the same theme/color-modifier path as solid fills. Painter mirrors cell-side gradient math. Locked in by `shapes/gradient-fills.xlsx`. Tile flip, `tileRect`, `rotWithShape` still unmodeled. |
| Pattern fills `pattFill` | ❌ | ❌ | P2 | Reuse cell pattern tile renderer. |
| Blip fills `blipFill` | ✅ | ✅ | P1 | Shape-as-image-fill; distinct from `xdr:pic`. Painter traces the preset path, `ctx.clip()`s, then stretches the image into the bbox honoring `<a:srcRect>` crop. `tile` parsed but painted as `stretch` (no fixture authors tile on a shape blip fill). Locked in by `shapes/blip-fills.xlsx`. |
| Group fill `grpFill` | ❌ | ❌ | P2 | Inherit/transform from parent group. |
| Basic line color + width | ✅ | ✅ | P0 | `a:ln` solid/noFill + width. |
| Line dash (`prstDash`) | ✅ | ✅ | P1 | Non-connector path wired up; `custDash` still unmodeled. Resolves shortcut #8. |
| Line cap/join | ✅ | ✅ | P1 | Reads `a:ln@cap` + `a:round`/`a:bevel`/`a:miter` on all shape kinds; themed cap/join from style-ref matrix walk also propagates. Resolves shortcut #9. |
| Compound lines / alignment | ❌ | ❌ | P2 | `cmpd`, `algn`. |
| Arrowheads (`headEnd` / `tailEnd`) | ✅ | ✅ | P0 | Connectors/lines. `triangle`, `stealth`, `diamond`, `oval`, `arrow`, `none`; `w`/`len` enums scale. |
| Outer shadow (`outerShdw`) | ✅ | ✅ | P1 | `<a:effectLst><a:outerShdw>` only (effectDag deferred). Maps `dist`/`dir` to canvas `shadowOffsetX/Y`, `blurRad` to `shadowBlur`, and threads the color (incl. theme/scheme + alpha modifier) through the same resolver as solid fills. `algn` and `rotWithShape` ignored — not visually significant on standalone shapes. Locked in by `shapes/outer-shadow.xlsx`; **`.hsx.png` diverges from `.ours.png` intentionally** (SpreadJS drops outerShdw), exactly as the fixture builder predicted. |
| Inner shadow / glow / soft edge | ❌ | ❌ | P2 | |
| Reflection / blur | ❌ | ❌ | P3 | |
| `effectLst` vs `effectDag` | ❌ | ❌ | P2 | Only `effectLst` realistically needed first. |
| Blip image effects (`alphaModFix`, `lum`, `clrChange`, `duotone`, `biLevel`, `grayscl`) | ❌ | ❌ | P2 | Apply to `blipFill` and `xdr:pic`. |
| 3D / scene3d / sp3d | ❌ | ❌ | P3 | Defer; preview stays 2D. |

### Shape text

| Feature | Extract | Render | Priority | Notes |
| --- | --- | --- | --- | --- |
| Paragraphs + runs | ✅ | ✅ | P0 | |
| Field runs `<a:fld>` (incl. `TxLink` cached value) | ✅ | ✅ | P0 | See shortcut #10. |
| `u`/`strike` as enum (not bool) | ✅ | ✅ | P0 | `underline_is_visible` / `strike_is_visible` helpers. |
| Run font size / bold / italic / underline / strike / color | ✅ | ✅ | P0 | |
| Latin font + theme refs | ✅ | ✅ | P0 | `+mn-lt` / `+mj-lt` via theme. |
| Paragraph alignment | ✅ | 🟡 | P0 | `l/ctr/r/just` mapped; `dist`, `thaiDist`, low-just not. |
| Body vertical anchor | ✅ | ✅ | P0 | `t/ctr/b`. |
| Word wrap (`wrap=square` / `wrap=none`) | ✅ | ✅ | P0 | |
| `vertOverflow` / `horzOverflow` | ✅ | ✅ | P1 | `vertOverflow` (`overflow`/`clip`/`ellipsis`) + `horzOverflow` (`overflow`/`clip`) extract through `textVertOverflow` / `textHorzOverflow`; painter `ctx.clip()`s the inner rect when either is set to clip/ellipsis and rewrites the last visible line's tail with `…` for ellipsis mode. Default `overflow` paints every line past the body rect (previously the painter incorrectly clipped). Locked in by `shapes/text-overflow.xlsx`. Resolves shortcut #4. |
| Body margins/insets (`bodyPr lIns/tIns/rIns/bIns`) | ✅ | ✅ | P0 | DrawingML defaults backfilled (91440 / 45720 / 91440 / 45720 EMU). |
| Text autofit (`normAutofit` / `spAutoFit`) | ✅ | 🟡 | P1 | `normAutofit fontScale` / `lnSpcReduction` extracted on every shape and applied at paint time (run sizes multiplied by `fontScale/100000`, line height by `1 - lnSpcReduction/100000`). `spAutoFit` extracted as a marker only — by spec the shape `ext` is already author-resized to fit the text, so the painter has nothing extra to do. Locked in by `shapes/text-autofit.xlsx`; **`.hsx.png` diverges from `.ours.png` intentionally** (SpreadJS ignores `normAutofit` and renders every box at the unscaled 11pt). |
| Text rotation / vertical text (`bodyPr@rot`, `vert`) | ✅ | ✅ | P1 | `<a:bodyPr rot>` (1/60000 deg) + `<a:bodyPr vert>` (`vert`/`vert270`/`wordArtVert`/`eaVert`/`mongolianVert`/`wordArtVertRtl`) extract through `textRotation` + `textVert` and compose into a single effective angle in the painter. Layout width/height swap on the perpendicular variants so wrap + anchor still operate along the reading direction. Locked in by `shapes/text-rotation-vert.xlsx`. |
| Text columns (`numCol`, `spcCol`, `rtlCol`) | ❌ | ❌ | P2 | |
| Text rect override (`a:rect`, `useSpRect`) | ❌ | ❌ | P2 | Matters for callouts. |
| Preset text warp (`prstTxWarp`) | ❌ | ❌ | P2 | WordArt-style; low priority in sheets. |
| `lstStyle` paragraph defaults | 🟡 | 🟡 | P1 | See shortcut #5. (Was overstated as ✅; downgraded.) |
| Bullets / numbering | ❌ | ❌ | P2 | `buChar`, `buAutoNum`, `buBlip`, `buNone`, `buClr`, `buSzPct/Pts`, `buFont`. |
| Paragraph spacing / indents / tabs | ❌ | ❌ | P2 | `lnSpc`, `spcBef`, `spcAft`, `marL`, `marR`, `indent`, `defTabSz`, `tabLst`, `fontAlgn`, `lvl`. |
| Run extras | ❌ | ❌ | P2 | `kern`, `spc`, `baseline`, `cap`, `lang`, `dirty`, `highlight`, etc. |
| Hyperlinks in shape text (`rPr/hlinkClick`) | ❌ | ❌ | P2 | Could reuse workbook hyperlink plumbing. |
| RTL paragraph (`pPr@rtl`) | ❌ | ❌ | P2 | Tied to broader RTL work. |
| `textlink` formula text | ❌ | ❌ | P2 | Needs engine wiring; `<a:fld>` cached value is enough for now. |

### Picture / blip details

| Feature | Extract | Render | Priority | Notes |
| --- | --- | --- | --- | --- |
| `srcRect` crop | ✅ | ✅ | P0 | Nested-pic path. Confirm top-level pic path too. |
| `stretch` vs `tile` blip fill | 🟡 | 🟡 | P1 | Effectively stretch today; `tile@tx/ty/sx/sy/flip/algn` + `tileRect` unmodeled. |
| `rotWithShape`, `dpi` | ❌ | ❌ | P2 | |
| SVG sidecar (`asvg:svgBlip`) | ✅ | ✅ | P1 | Decoded as part of `<a:blipFill>` extraction; when `<a:blip><a:extLst><a:ext uri="{96DAC541-...}"><asvg:svgBlip r:embed="..."/>` is present, the SVG embed wins over the raster fallback (vector → crisper at scale). Locked in by panel `b5` of `shapes/blip-fills.xlsx` — SpreadJS drops the sidecar and renders the raster, we render the SVG; intentional divergence. |
| Modern blip extensions (`a14:useLocalDpi`, ink, model3d, camera) | ❌ | ❌ | P3 | Long-tail. |

### Theme / style inheritance

| Feature | Extract | Render | Priority | Notes |
| --- | --- | --- | --- | --- |
| Direct `spPr` | 🟡 | 🟡 | P0 | Current implementation is mostly direct-properties only. |
| Shape `style` refs (`fillRef`/`lnRef`/`fontRef`/`effectRef`) | 🟡 | 🟡 | P1 | Minimal resolver — see shortcut #6. Full matrix walk gated on a `Theme` extension that surfaces `<a:fmtScheme>` alongside the color scheme. |
| Default shape definitions (`objectDefaults/spDef|lnDef|txDef`) | ❌ | ❌ | P2 | |
| Group property inheritance | ❌ | ❌ | P2 | Spec: individual shape props take precedence over group props. Current flattening does not inherit group fill/effects. |

## Implementation queue

Ordered by impact × cost. Pick from the top.

### P1 — visual fidelity that real workbooks need

1. ~~**Gradient fills (`gradFill`)**~~ — **shipped.** Linear + path painters reuse the cell-side gradient math; stop colors resolve through the same theme + color-modifier path as solid fills. Locked in by `shapes/gradient-fills.xlsx`. Remaining: tile flip, `tileRect`, `rotWithShape`.
2. ~~**Outer shadow (`outerShdw`)**~~ — **shipped.** Direct `<a:effectLst><a:outerShdw>` only; `effectDag` and effectRef-driven shadows still deferred. `dist`/`dir`/`blurRad` map straight onto canvas `shadow*` primitives; color resolves through the existing srgb/scheme/preset/sys path with `<a:alpha>` mod respected. `algn` and `rotWithShape` ignored (negligible on standalone shapes). Locked in by `shapes/outer-shadow.xlsx` — the fixture intentionally has `.hsx.png` (SpreadJS, drops the effect) diverging from `.ours.png` (we paint it).
3. **Style-ref matrix walk** — read `<a:fmtScheme><a:fillStyleLst>` / `<a:lnStyleLst>` / `<a:effectStyleLst>`. Unlocks themed gradient idx 2/3, per-style line dashes, and theme-driven outer shadows on `effectRef idx≥1` (paired need with #2 above) "for free" now that the direct paths exist. Resolves shortcut #6.
4. ~~**Group rotation + body-rect-follows-flip**~~ — **shipped.** The extractor builds a 2D affine per group `<a:xfrm>` (including `rot`) and propagates the accumulated rotation to every nested shape / picture / connector; children of a rotated `<xdr:grpSp>` now rotate as a rigid body. The painter mirrors the preset text body rect for `flipH` / `flipV` so captions on asymmetric presets (right-arrow, pentagon, callouts) sit over the visually-correct half of the shape. Locked in by `shapes/groups-rotated.xlsx` (rotation, with `rot` of 0°/30°/90°) + the text-position diff on `shapes/shape-flips.xlsx` (flip body rect). Remaining: nested rotated groups are approximate (see Known v0 shortcuts #11), and group `flipH`/`flipV` is parsed but not yet propagated (#12).
5. ~~**Preset dash + line cap/join on non-connector outlines**~~ — **shipped.** `<a:ln>` `cap` / join / `prstDash` extract on every shape path (`visit_shape`, `visit_connector`); painter `drawShape` honors `node.lineDash` / `node.lineCap` / `node.lineJoin`. Themed cap/join values that the style-ref matrix walk already extracted now reach the canvas too. Locked in by `shapes/line-cap-join-dash.xlsx`. Resolves shortcuts #8, #9.
6. ~~**`avLst` for `roundRect` / arrows / callouts**~~ — **shipped (arrow + `roundRect` arms).** `pathForPreset` reads `node.adj1` / `node.adj2` for `roundRect` (corner offset), the four cardinal arrows (`adj1`=tail thickness fraction, `adj2`=head length fraction), and `leftRightArrow` (`adj1`=tail height fraction, `adj2`=per-side head length fraction, head capped at `w/2`). Locked in by `shapes/avlst-adjusts.xlsx`. Callouts remain in shortcut #3 — no callout preset is currently rendered, so they'll pick up adjust handling as part of the long-tail preset corpus work (queue #13).
7. ~~**Text autofit (`normAutofit` font scaling, `spAutoFit`)**~~ — **shipped.** `<a:bodyPr>`'s autofit choice (`a:noAutofit` / `a:normAutofit` / `a:spAutoFit`) now decodes through `body_autofit` in `crates/xlcore-export/src/shapes_text.rs` into three new `ShapeNode` fields (`textAutofit`, `textFontScale`, `textLineSpaceReduction`). The painter (`drawShapeText` in `packages/xlsx-preview/src/shape.ts`) multiplies every run's font size by `fontScale/100000` and every line height by `1 - lnSpcReduction/100000`; `spAutoFit` is recorded but doesn't trigger paint-time work because the shape `ext` already reflects the author-time fit. Locked in by `shapes/text-autofit.xlsx` — the fixture intentionally has `.hsx.png` (SpreadJS, drops the effect) diverging from `.ours.png` (we paint it), exactly like the `outer-shadow.xlsx` divergence.
8. ~~**Text rotation / vertical text (`bodyPr@rot`, `vert`)**~~ — **shipped.** `<a:bodyPr rot>` (1/60000 deg) + `<a:bodyPr vert>` (full spec enum — `vert`/`vert270`/`wordArtVert`/`eaVert`/`mongolianVert`/`wordArtVertRtl`) now extract through `textRotation` + `textVert` on every shape path (`body_vert_token` in `crates/xlcore-export/src/shapes_text.rs`); painter `drawShapeText` collapses both into a single effective angle, rotates around the inner-rect center, and swaps layout width/height on perpendicular variants so wrap + anchor still operate along the reading direction. Locked in by `shapes/text-rotation-vert.xlsx` — the fixture intentionally has `.hsx.png` (SpreadJS, drops `<a:bodyPr rot>` and collapses `vert270` onto `vert`) diverging from `.ours.png`, exactly like `outer-shadow.xlsx` and `text-autofit.xlsx`.
9. ~~**`vertOverflow="clip"` + `horzOverflow`**~~ — **shipped.** `<a:bodyPr>` `vertOverflow` / `horzOverflow` now extract on every shape path through `textVertOverflow` / `textHorzOverflow`. Painter (`drawShapeText` in `packages/xlsx-preview/src/shape.ts`) treats the spec default `overflow` as no-clip; `clip` and `ellipsis` push `ctx.clip()` over the inner rect; `ellipsis` additionally rewrites the last fully-visible line's tail run with `…` (character-by-character truncation until the trailing ellipsis fits `innerW`). Locked in by `shapes/text-overflow.xlsx` — `.hsx.png` intentionally diverges from `.ours.png` (SpreadJS drops both attrs). Resolves shortcut #4. Also fixed an unrelated bug exposed by this work: `body_wrap_token` matched the wrong Debug variant name for `wrap="none"`.
10. ~~**Blip fills (`blipFill`) + SVG sidecar (`asvg:svgBlip`)**~~ — **shipped.** `<a:blipFill>` on `<xdr:sp>/<xdr:spPr>` decodes through a new `ShapeBlipFill` (`data_uri`, `src_rect`, `kind`) on `ShapeNode.fill_blip`; the painter `ctx.clip()`s to the preset path and stretches the image into the bbox honoring `srcRect`. The `asvg:svgBlip` sidecar (modern Office's vector companion) is preferred over the raster fallback when present — vector → crisper at scale. Locked in by `shapes/blip-fills.xlsx`. `tile` parsed but painted as `stretch` (no real workbook we've seen tiles a shape blip fill); `<a:blipFill>` image effects (`alphaModFix`, `lum`, `clrChange`, `duotone`, `biLevel`, `grayscl`) still ignored — they apply equally to `<xdr:pic>`, so they're tracked under the shared P2 "Blip image effects" row.
11. **`absoluteAnchor` + `editAs` round-trip** — cheap; gates agent edit flows.
12. **`cNvPr/hlinkClick` on shapes** — wire into existing workbook hyperlink event channel.
13. **Long-tail preset corpus** — flowchart symbols, callouts, action buttons, arc/donut, plaque/bevel, brace/bracket pairs (closes shortcut #2). Single biggest "more shapes work" lever; pairs naturally with `custGeom` (P2) since both want a DrawingML path interpreter driven from `presetShapeDefinitions.xml`.
14. **`lstStyle` cascade — round 2** — `marL`/`indent`/`lnSpc`/`spcBef`/`spcAft`, run-`u="none"`-as-disable-inherited, bullet list. Resolves shortcut #5.
15. **Connection sites — round 2** — preset-aware sites (chevron tip, star points, flowchart non-cardinal), custom `cxnLst`, multi-segment `bentConnector{2,4,5}`. Resolves shortcut #1.
16. **Legacy VML drawings (`vmlDrawing*.vml`)** — separate workstream; needed so comment indicators / form controls / pre-2007 shapes render at all.

### P2+ — long tail / full DrawingML

- Generic `custGeom` and preset path interpreter driven by `presetShapeDefinitions.xml`.
- Pattern/group fills and the rest of the effect stack (inner shadow / glow / reflection / softEdge / blur / `effectDag`).
- Blip image effects (`alphaModFix`, `lum`, `clrChange`, `duotone`, `biLevel`, `grayscl`).
- Group property inheritance and default shape definitions (`objectDefaults/spDef|lnDef|txDef`).
- Rich list typography: bullets, numbering, tabs, kerning/spacing, columns, RTL.
- Form controls + OLE objects (`<oleObjects>`, `<controls>`) as static placeholders.
- `contentPart`, nested `graphicFrame`, `prstTxWarp` / WordArt, SmartArt/diagrams (`dgm`) remain separate larger work.

## Suggested PARITY.md one-line status

Shapes remain 🟡 — P1 #1–10 (gradient fills, direct `<a:effectLst>` outer shadow, style-ref matrix walk, group rotation + body-rect-follows-flip, preset dash + line cap/join on non-connector outlines, `avLst` adjusts on `roundRect` / cardinal arrows / `leftRightArrow`, `normAutofit` / `spAutoFit` text autofit, `<a:bodyPr rot>` / `<a:bodyPr vert>` text-body rotation + vertical-text, `vertOverflow` / `horzOverflow` text clipping + ellipsis, and `<a:blipFill>` on `<xdr:sp>` + `asvg:svgBlip` SVG sidecar) all shipped. Current v0 is good for basic callouts/buttons, themed gradients + drop shadows on direct *and* style-ref-driven DrawingML shapes, grouped/rotated screenshot chrome, parameterised arrows + corner radii, autofit-scaled labels on cramped textboxes, vertically-stacked / arbitrarily-rotated text-body labels, Office-authored org-chart / SOTP diagrams, and modern-Office icon / textured-banner shapes with raster-or-SVG blip fills. Next obvious gap is P1 #11 (`absoluteAnchor` + `editAs` round-trip) or #13 (long-tail preset corpus driven by `presetShapeDefinitions.xml`).
