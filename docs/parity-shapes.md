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

| Layer | Status | Files / notes |
| --- | --- | --- |
| Extraction | 🟡 | `crates/xlcore-export/src/charts.rs` surfaces top-level `xdr:sp`, `xdr:grpSp`, and `xdr:cxnSp` from `twoCellAnchor` / `oneCellAnchor`. Top-level `absoluteAnchor`, `contentPart` still ignored. |
| Shape tree | 🟡 | `crates/xlcore-export/src/shapes.rs` flattens `sp` / nested `grpSp` / nested `cxnSp`; maps group `xfrm/off/ext/chOff/chExt`; nested `xdr:pic` inside groups becomes image nodes. |
| Schema | 🟡 | JSON model is intentionally painter-oriented (`Shape { nodes }`), not a full DrawingML AST. Good for preview; not enough for round-trip editing. |
| Rendering | 🟡 | `packages/xlsx-preview/src/shape.ts` paints a small preset subset, solid fills, basic outlines, text, rotation, nested pictures. Unknown presets fall back to rectangle. |
| Fixtures | ✅ | Full P0 corpus landed under `tests/fixtures/shapes/`: `basic-autoshapes.xlsx`, `textbox-wrap-align.xlsx`, `connectors.xlsx`, `style-refs-themed.xlsx`, `groups-and-pictures.xlsx`. Each ships with `.hsx.png` (ground truth) and `.ours.png` (current regression baseline). The last addition (`groups-and-pictures.xlsx`) is the dedicated z-order + grouping + picture regression baseline: 5 panels covering (A) three overlapping shapes in known XML order, (B) standalone `xdr:pic` via both `pictures.add` and `shapes.addPictureShape`, (C) a 3-shape group, (D) a group with a nested `<xdr:pic>` inside, (E) a nested group (group-in-group exercising the recursive `visit_shapes` walk + compounded `chOff/chExt`). |

## Parity matrix

### Anchoring / object model

| Feature | Extract | Render | Priority | Notes |
| --- | --- | --- | --- | --- |
| `twoCellAnchor` | ✅ | ✅ | P0 | Main path. `editAs` behavior (`twoCell` default, `oneCell`, `absolute`) is not modeled; current renderer effectively uses resolved anchor rect. Matters for any agent edit flow that inserts rows/columns. |
| `oneCellAnchor` | ✅ | ✅ | P0 | Uses EMU extents for pixel-accurate render. |
| `absoluteAnchor` | ❌ | ❌ | P1 | Spec has absolute EMU `pos` + `ext`; likely rare but easy to surface. |
| `clientData` flags | ❌ | n/a | P2 | `fLocksWithSheet`, `fPrintsWithSheet` on every anchor (§20.5.2.3). No preview impact, but needed for round-trip + sheet-protection semantics. |
| `editAs` round-trip | ❌ | n/a | P1 | Pass-through is fine for preview; the engine/bridge layer will need it once shape moves are part of the agent mutation API. |
| Top-level `sp` | ✅ | 🟡 | P0 | Geometry/fill/text coverage partial. |
| Top-level `grpSp` | ✅ | 🟡 | P0 | Nested `sp` / `grpSp` / `pic` supported; nested `cxnSp` / `graphicFrame` ignored. Recursion + compounded `chOff/chExt` locked in by the panel-E nested group in `tests/fixtures/shapes/groups-and-pictures.xlsx`. |
| Top-level `pic` | ✅ | ✅ | P0 | Separate image path; top-level image crop/rotation tracked in `PARITY.md`. Both producer paths covered by panel B of `tests/fixtures/shapes/groups-and-pictures.xlsx` — `sheet.pictures.add` (the "loose" `xdr:pic` with anchor-derived geometry, no `xfrm`) and `sheet.shapes.addPictureShape` (the `xdr:pic` with explicit `xfrm`). |
| Nested `pic` in groups | ✅ | ✅ | P0 | Includes `<a:srcRect>` crop. Locked in by panel D of `tests/fixtures/shapes/groups-and-pictures.xlsx` (label rect + nested picture under one `<xdr:grpSp>`). |
| `cxnSp` connectors | ✅ | ✅ | P0 | Top-level and group-nested. Straight + bentConnector3 with adj1; `line` / `lineInv` presets also routed through the connector painter so they no longer fall back to a blue rect. Locked in by `tests/fixtures/shapes/connectors.xlsx` (5 connectors: straight, bent Z, vertical bent, horizontal dashed-red w/ triangle, diagonal w/ oval head + triangle tail). |
| `graphicFrame` in groups | ❌ | ❌ | P2 | Could contain chart/diagram/table-like graphics inside group. |
| `contentPart` | ❌ | ❌ | P3 | Extension payload; low preview value initially. |
| Non-visual props / alt text | ❌ | n/a | P2 | `cNvPr name/descr/title`, locks, hidden metadata not surfaced. |
| `cNvPr/hlinkClick` + `hlinkHover` | ❌ | ❌ | P1 | Click-the-shape hyperlinks (separate from in-text `a:hlinkClick` on a run). Common for navigation buttons; should reuse the existing workbook hyperlink event channel. |
| `macro`, `textlink` attrs | ❌ | ❌ | P2 | `textlink` can bind shape text to a cell formula. Macro click actions are non-preview. |
| Connection sites `cxnLst` | ❌ | ❌ | P1 | Required for routing connector endpoints (`stCxn`/`endCxn` ids on `cNvCxnSpPr`) instead of falling back to bbox-center attach. Pairs with connector work. |
| Adjust handles `ahLst` (`ahPolar` / `ahXY`) | ❌ | n/a | P3 | Authoring-time UI only; no preview impact. |
| Shape locks (`spLocks`, `cxnSpLocks`, `picLocks`, `grpSpLocks`) | ❌ | n/a | P3 | Edit-time gates; not relevant for preview but needed for round-trip fidelity. |
| Legacy VML drawings (`xl/drawings/vmlDrawing*.vml`) | ❌ | ❌ | P1 | Comment indicators, form controls, and legacy autoshapes still ship as VML in many workbooks. Out of scope for pure DrawingML parity, but worth a callout — `legacyDrawing` r:id on the sheet is currently ignored. |
| Form controls / OLE (`<controls>`, `<oleObjects>`) | ❌ | ❌ | P2 | Anchored via `xdr:from/to` like drawings; rendered as static placeholder is sufficient for v0. |

### Transforms / z-order

| Feature | Extract | Render | Priority | Notes |
| --- | --- | --- | --- | --- |
| Shape offset / extent | ✅ | ✅ | P0 | EMU bbox normalized into node-relative coords. |
| Group `chOff` / `chExt` mapping | ✅ | ✅ | P0 | Important for Office-authored groups. |
| Shape rotation `xfrm@rot` | ✅ | ✅ | P0 | Stored as 1/60000 degrees; renderer rotates node around center. |
| Group rotation | ❌ | ❌ | P1 | `CT_GroupTransform2D` has `rot`; current frame mapping ignores it. |
| Flip H/V on shape `xfrm` | 🟡 | 🟡 | P1 | Surfaced + applied for connectors only (where the OOXML producer uses flipH to express line direction). Non-connector shape flips still ignored. |
| Z-order | ✅ | ✅ | P0 | XML traversal order is preserved for emitted nodes/drawings. Locked in by panel A of `tests/fixtures/shapes/groups-and-pictures.xlsx` — three overlapping shapes (rect, oval, diamond) emitted in that order; the diamond must paint on top. |
| Clipping to group/shape | ❌ | ❌ | P2 | Current flattened model does not clip children to group bounds. |
| `bwMode` (black/white render mode) | ❌ | ❌ | P3 | `spPr@bwMode`; rare in spreadsheets. |

### Geometry

| Feature | Extract | Render | Priority | Notes |
| --- | --- | --- | --- | --- |
| `prstGeom`: `rect` | ✅ | ✅ | P0 | |
| `roundRect` | ✅ | ✅ | P0 | Uses hardcoded default radius; ignores `avLst`. |
| `ellipse` / `circle` | ✅ | ✅ | P0 | |
| `triangle`, `diamond` | ✅ | ✅ | P0 | |
| Basic block arrows | ✅ | ✅ | P0 | `leftArrow`, `rightArrow`, `upArrow`, `downArrow`; hardcoded default adjusts. |
| Lines (`prstGeom=line` / `lineInv`) | ✅ | ✅ | P0 | Routed through the connector painter — `line` is a top-left→bottom-right diagonal, `lineInv` is top-right→bottom-left. Honors `flipH/V`, dash, and arrowheads. |
| More common presets | 🟡 | ❌ | P1 | Chevron, pentagon/hexagon, stars, callouts, braces/brackets, flowchart symbols, action buttons. Spec lists 187 presets. |
| `avLst` adjust values | ❌ | ❌ | P1 | Needed for arrow head/tail size, rounded rect radius, callout pointers, stars, arcs. |
| `custGeom` paths | ❌ | ❌ | P2 | Requires DrawingML path interpreter (`moveTo`, `lnTo`, `arcTo`, bezier, close) and guide formulas. |
| Informative preset path corpus | n/a | n/a | P2 | `OfficeOpenXML-DrawingMLGeometries.zip/presetShapeDefinitions.xml` can seed preset rendering instead of hand-writing 187 shapes. |

### Fill / outline / effects

| Feature | Extract | Render | Priority | Notes |
| --- | --- | --- | --- | --- |
| `noFill` | ✅ | ✅ | P0 | Falls through to no fill. |
| `solidFill` `srgbClr` | ✅ | ✅ | P0 | Basic hex. Audit color modifiers / alpha. |
| `solidFill` `schemeClr` | ✅ | ✅ | P0 | Uses workbook theme + existing modifier resolver. |
| `solidFill` `prstClr`, `sysClr` | 🟡 | ✅ | P1 | Small preset table / `lastClr`; expand preset color coverage and modifiers. |
| `scrgbClr`, `hslClr` | ❌ | ❌ | P2 | Already solved for cells/charts elsewhere; can reuse color conversion. |
| Alpha / transparency | 🟡 | 🟡 | P1 | Color modifiers may parse some alpha in theme path; shape renderer does not generally model opacity. |
| Full color modifier set | 🟡 | 🟡 | P1 | Spec §20.1.2.3 lists `lumMod/lumOff`, `shade`, `tint`, `satMod/satOff`, `alpha`, `alphaMod`, `alphaOff`, `gamma`/`invGamma`, `comp`, `inv`, `gray`, `hueMod/Off`, `red/green/blueMod/Off`. Cell-side resolver covers most; verify the shape path uses the same one end-to-end. |
| Gradient fills `gradFill` | ❌ | ❌ | P1 | Reuse cell/chart gradient logic where possible. Common in themed shapes. |
| Pattern fills `pattFill` | ❌ | ❌ | P2 | Reuse cell pattern tile renderer. |
| Blip fills `blipFill` | ❌ | ❌ | P1 | Shape-as-image-fill; distinct from `xdr:pic`. Common for textured buttons/banners. |
| Group fill `grpFill` | ❌ | ❌ | P2 | Inherit/transform fill from parent group. |
| Basic line color + width | ✅ | ✅ | P0 | `a:ln` solid/noFill + width. |
| Line dash (`prstDash`) | 🟡 | 🟡 | P1 | Surfaced + rendered for connectors/lines: `dash`, `dot`, `dashDot`, `lgDash`/`lgDashDot`/`lgDashDotDot`, `sysDash`/`sysDot`/`sysDashDot`/`sysDashDotDot`. `custDash` still ignored; non-connector shape outlines don't yet read dash. |
| Line cap/join | ❌ | ❌ | P2 | `cap`, round/bevel/miter — connector painter currently fixes cap=butt, join=miter. |
| Compound lines / alignment | ❌ | ❌ | P2 | `cmpd`, `algn`. |
| Arrowheads (`headEnd` / `tailEnd`) | ✅ | ✅ | P0 | Surfaced + rendered for connectors/lines. Types: `triangle`, `stealth`, `diamond`, `oval`, `arrow` (open V), `none`. `w`/`len` enums (`sm`/`med`/`lg`) scale the head size relative to stroke width. |
| Outer shadow (`outerShdw`) | ❌ | ❌ | P1 | Most visually impactful effect on themed buttons/cards. |
| Inner shadow (`innerShdw`) | ❌ | ❌ | P2 | |
| Glow (`glow`) | ❌ | ❌ | P2 | |
| Soft edge (`softEdge`) | ❌ | ❌ | P2 | |
| Reflection (`reflection`) | ❌ | ❌ | P3 | |
| Blur (`blur`) | ❌ | ❌ | P3 | |
| `effectLst` vs `effectDag` | ❌ | ❌ | P2 | List form vs DAG form (§20.1.8.26 effectLst / §20.1.8.25 effectDag); only `effectLst` realistically needed first. |
| Blip image effects on shape-as-image (`alphaModFix`, `lum`, `clrChange`, `duotone`, `biLevel`, `grayscl`) | ❌ | ❌ | P2 | Apply to `blipFill` and to `xdr:pic`. Reuse the shared decode cache plus a post-process layer. |
| 3D / scene3d / sp3d | ❌ | ❌ | P3 | Defer; preview can stay 2D. |

### Shape text

| Feature | Extract | Render | Priority | Notes |
| --- | --- | --- | --- | --- |
| Paragraphs + runs | ✅ | ✅ | P0 | Multiple paragraphs and runs. |
| Run font size / bold / italic | ✅ | ✅ | P0 | `a:rPr sz b i`. |
| Run underline / strike | ✅ | ✅ | P1 | Extracted; renderer support should be verified with fixture. |
| Run color solidFill | ✅ | ✅ | P0 | Partial color choice support follows shape solidFill. |
| Latin font + theme refs | ✅ | ✅ | P0 | `+mn-lt` / `+mj-lt` resolve via theme. |
| Paragraph alignment | ✅ | 🟡 | P0 | `l/ctr/r/just` mapped; `dist`, `thaiDist`, low-just not. |
| Body vertical anchor | ✅ | ✅ | P0 | `t/ctr/b`; default top. |
| Word wrap | ✅ | ✅ | P0 | `wrap=square` / default wraps; `wrap=none` overflows. |
| Body margins/insets | ✅ | ✅ | P0 | `<a:bodyPr lIns/tIns/rIns/bIns/>` extracted as `ShapeNode.textInsetsEmu` (length-4 EMU vec, missing slots back-filled with the DrawingML defaults 91440 / 45720 / 91440 / 45720 EMU). Renderer in `shape.ts` consumes them at `PX_PER_EMU` and falls back to those same defaults when the field is `None` — replacing the old 4%-of-shape magic margin. Locked in by `tests/fixtures/shapes/textbox-wrap-align.xlsx` (row 3: default / tight / loose / asym). |
| Text autofit | ❌ | ❌ | P1 | `noAutofit`, `normAutofit` (with `fontScale` / `lnSpcReduction`), `spAutoFit`; affects cramped boxes. |
| Text overflow | ❌ | ❌ | P1 | `vertOverflow`, `horzOverflow`. |
| Text rotation / vertical text | ❌ | ❌ | P1 | `bodyPr@rot`, `vert` enum (`horz`, `vert`, `vert270`, `eaVert`, `mongolianVert`, `wordArtVert`, `wordArtVertRtl`), `upright`; separate from shape rotation. |
| Text columns | ❌ | ❌ | P2 | `bodyPr@numCol`, `spcCol`, `rtlCol`. |
| Text rect override (`a:rect`, `useSpRect`) | ❌ | ❌ | P2 | Custom text box inside the shape; matters for callouts and presets whose preset path defines its own text rect. |
| Preset text warp (`prstTxWarp`) | ❌ | ❌ | P2 | WordArt-style geometry warp on `bodyPr`. Low priority for spreadsheets but cheap to detect/skip. |
| `lstStyle` paragraph defaults | ✅ | ✅ | P0 | **Shipped.** `shapes.rs::text_body_to_paragraphs` now cascades run/paragraph props in spec order (lowest → highest precedence): (1) `<a:lstStyle><a:defPPr><a:defRPr>`, (2) `<a:lstStyle><a:lvl{N+1}pPr><a:defRPr>` matching the paragraph's `pPr@lvl` (default 0 → lvl1pPr; clamped to 0…8), (3) `<a:p><a:pPr><a:defRPr>` (previously ignored — this was the real gap on themed templates, since SpreadJS-emitted shape XML always sets pPr/defRPr and only sometimes echoes it on the run rPr), (4) `<a:r><a:rPr>`. Same cascade for paragraph alignment. Scope-limited for v0: font size / bold / italic / underline / strike / solidFill color / latin font — same fields the run-rPr path supports today. `marL`, `indent`, `lnSpc`, `spcBef`, `spcAft`, kerning, baseline, run-`u`-as-disable (`<a:rPr u="none">` clearing inherited underline), and the bullet list are still ignored. Locked in by `tests/fixtures/shapes/list-style-inheritance.xlsx` (4 panels: control / lstStyle-defPPr / lstStyle-lvl1pPr / cascade-precedence). |
| Bullets / numbering | ❌ | ❌ | P2 | `buChar`, `buAutoNum`, `buBlip`, `buNone`, `buClr`, `buSzPct/Pts`, `buFont`, `buFontTx`. |
| Paragraph spacing / indents / tabs | ❌ | ❌ | P2 | `lnSpc`, `spcBef`, `spcAft`, `marL`, `marR`, `indent`, `defTabSz`, `tabLst`, `fontAlgn`, `algn=dist`/`thaiDist`, `lvl`. |
| Run extras | ❌ | ❌ | P2 | `kern`, `spc` (char spacing), `baseline`, `cap` (none/small/all), `lang`/`altLang`, `dirty`, `err`, `noProof`, `smtClean`/`smtId`, `highlight`. |
| Hyperlinks in shape text (`rPr/hlinkClick` + `hlinkMouseOver`) | ❌ | ❌ | P2 | Could reuse workbook hyperlink event plumbing. |
| RTL paragraph (`pPr@rtl`) | ❌ | ❌ | P2 | Tied to broader RTL sheet work. |
| `textlink` formula text | ❌ | ❌ | P2 | Needs formula/cell display integration; gated on the engine. |

### Picture / blip details (shapes + nested pics)

| Feature | Extract | Render | Priority | Notes |
| --- | --- | --- | --- | --- |
| `srcRect` crop | ✅ | ✅ | P0 | Nested-pic path shipped. Confirm top-level pic path too. |
| `stretch` vs `tile` blip fill | 🟡 | 🟡 | P1 | Currently effectively stretch; `tile@tx/ty/sx/sy/flip/algn` and `tileRect` not modeled. |
| `rotWithShape`, `dpi` | ❌ | ❌ | P2 | Affects rotated picture fills and high-DPI sources. |
| SVG sidecar (`asvg:svgBlip`) | ❌ | ❌ | P1 | Modern Office stores both `r:embed` (raster fallback) + SVG. We pick raster today; SVG would render crisper at scale. Sits inside `blip/extLst` (`a14`/`asvg` namespaces). |
| Modern blip extensions (`a14:useLocalDpi`, `a:duotone`, ink, model3d, camera) | ❌ | ❌ | P3 | Long-tail. |

### Theme / style inheritance

| Feature | Extract | Render | Priority | Notes |
| --- | --- | --- | --- | --- |
| Direct `spPr` | 🟡 | 🟡 | P0 | Current implementation is mostly direct-properties only. |
| Shape `style` refs | 🟡 | 🟡 | P1 | **Minimal resolver shipped.** `shapes.rs::resolve_style_refs` walks `<xdr:style>` and falls back to `fillRef` color choice (any idx → solid fill of the resolved color; idx=0 → noFill), `lnRef` color + standard subtle/moderate/intense widths (6350/12700/19050 EMU for idx 1/2/3), and `fontRef` (`major`/`minor` → theme font + color override on runs that don't set their own). Line presets (`line`/`lineInv`) deliberately skip the fill fallback. Skipped for v0: actually walking the theme's `<a:fmtScheme><a:fillStyleLst>`/`<a:lnStyleLst>` matrix entries (we treat every fillRef idx>=1 as if the matrix entry were `solidFill phClr` — correct for the standard theme's idx=1 but loses the gradients on idx 2/3 and the per-style line dashes). Locked in by `tests/fixtures/shapes/style-refs-themed.xlsx` (basic-autoshapes with all direct `<a:solidFill>` / `<a:ln>` stripped, lnRef/fillRef rewritten to cycle accent1..6) and `cargo test -p xlcore-export shapes::`. |
| Default shape definitions | ❌ | ❌ | P2 | `themeElements/objectDefaults/spDef`, `lnDef`, `txDef`. |
| Group property inheritance | ❌ | ❌ | P2 | Spec says individual shape props take precedence over group props; current flattening does not inherit group fill/effects. |

## Recommended implementation plan

### P0 — make common worksheet chrome dependable

1. ~~**Add fixture corpus first**~~ **Shipped.** All five P0 fixtures are committed under `tests/fixtures/shapes/` with `.hsx.png` + `.ours.png` baselines: `basic-autoshapes.xlsx`, `textbox-wrap-align.xlsx`, `connectors.xlsx`, `style-refs-themed.xlsx`, `groups-and-pictures.xlsx`. `style-refs-themed` is derived from `basic-autoshapes.xlsx` via pure XML rewrite (committed `tests/fixtures/shapes/build-style-refs-themed.sh`) since the current `hsx eval` no longer persists shape mutations to disk — documented inline in that script. `groups-and-pictures` was the last gap; it exercises top-level `xdr:pic` (both `pictures.add` and `shapes.addPictureShape` paths), `xdr:grpSp` flattening, the nested-`xdr:pic`-in-group special case, nested groups, and z-order = XML traversal order in a single sheet. Each panel is labelled in column A so the visual diff is self-describing.
2. **Connectors / line primitives**: ~~surface top-level and nested `xdr:cxnSp`, plus `prstGeom=line/lineInv`, as a `ShapeNode` line kind with stroke, dash, arrowheads.~~ **Shipped.** `ShapeNode` gained `isConnector` / `flipH` / `flipV` / `lineDash` / `headEnd` / `tailEnd` / `adj1` (`schema/charts.rs`); `shapes::visit_connector` walks `xdr:cxnSp` at root and inside groups; the renderer paints stroked polylines (straight, `bentConnector3` Z-route, `line`/`lineInv` diagonal) with dash patterns scaled to stroke width and the five OOXML arrowhead kinds (triangle / stealth / diamond / oval / arrow). `anchorToRect` was relaxed so axis-degenerate connector anchors (h==0 for horizontal lines, w==0 for vertical) survive layout. `stCxn`/`endCxn` ids still TODO — we currently route bbox-to-bbox via the connector's own `xfrm` (which Excel snaps to attached-shape edges on save, so visually this matches Office output).
3. **Text insets**: ~~extract `bodyPr lIns/tIns/rIns/bIns`; renderer should use EMU→px padding instead of a fixed magic value.~~ **Shipped.** Schema gained `ShapeNode.textInsetsEmu`; extractor reads `BodyProperties.{left,top,right,bottom}_inset`; renderer applies them in `drawShapeText` with DrawingML-default fallback. Visible win: text inside narrow shapes (triangle / chevron / decision in `basic-autoshapes.xlsx`) no longer fragments into single-character vertical strips, and the inset row of `textbox-wrap-align.xlsx` produces four distinct paintings.
4. ~~**Style refs minimal resolver**: resolve `a:style/fillRef/lnRef/fontRef`/`effectRef` against theme format scheme for cases without direct `solidFill` / `ln`. Office-authored shapes lean on this heavily.~~ **Shipped (minimal).** See the `Shape style refs` row above for the v0 scope (color + standard widths, no matrix walk yet). The matrix walk — reading the actual `<a:fmtScheme><a:fillStyleLst>` / `<a:lnStyleLst>` entries so idx=2/3 produce gradient fills / heavier dashed lines exactly like Office — is now a P1 follow-up, gated on a `Theme` extension that surfaces the format-scheme matrix in addition to the color scheme.
5. ~~**`lstStyle` paragraph inheritance**~~ **Shipped.** Cascade implemented in `shapes.rs::text_body_to_paragraphs` (see the `lstStyle paragraph defaults` row above for scope + fixture). The same cascade also fixes the more common case of a paragraph whose `<a:pPr><a:defRPr>` carries the real font/size/color while the run `<a:rPr>` is empty — the actual "real fidelity gap on themed templates" the parity doc was warning about. **Divergence to be aware of:** HSX (SpreadJS) does not honor `<a:lstStyle><a:defPPr>` at all and only partially honors `<a:lvl1pPr>` (size doesn’t inherit). Our renderer is now more spec-correct than HSX on synthetic lstStyle-only fixtures (panel s2 in particular); we never render LESS than HSX on real Excel-authored workbooks since those always set the run rPr explicitly.
6. ~~**Lock in z-order and group mapping** with visual tests; current group mapping is valuable and should not regress.~~ **Shipped** — see `tests/fixtures/shapes/groups-and-pictures.xlsx` (panels A / C / E) and the matrix rows above.

### P1 — noticeably improve visual fidelity

1. `absoluteAnchor` support in `charts.rs` / drawing extraction.
2. Transform flips and group rotation.
3. Line dash/cap/join + arrowheads.
4. Gradient fills and blip fills (incl. `srcRect`/`stretch` on shape fill path).
5. `avLst` adjust values for the already-supported presets, then add the next 20 common presets: `line`, `chevron`, `pentagon`, `hexagon`, `star5`, `leftRightArrow`, arrow callouts, braces/brackets, cloud/callout, flowchart process/decision/data.
6. Text autofit (`normAutofit` font scaling), overflow, body rotation / vertical text.
7. Outer shadow effect — by far the most visible omitted effect on themed buttons/cards.
8. Click-the-shape hyperlinks (`cNvPr/hlinkClick`) wired through the existing workbook hyperlink event channel.
9. SVG sidecar pick (`asvg:svgBlip`) for crisp rendering of modern icon pictures.

### P2+ — long tail / full DrawingML

- Generic `custGeom` and preset path interpreter driven by `presetShapeDefinitions.xml`.
- Pattern/group fills and the rest of the effect stack (inner shadow / glow / reflection / softEdge / blur / `effectDag`).
- Blip image effects (`alphaModFix`, `lum`, `clrChange`, `duotone`, `biLevel`, `grayscl`).
- Group property inheritance and default shape definitions (`objectDefaults/spDef|lnDef|txDef`).
- Rich list typography: bullets, numbering, line spacing, tabs, paragraph defaults, run kerning/spacing, columns.
- Connection sites (`cxnLst`) for shape-attached connectors.
- Legacy VML drawings (`xl/drawings/vmlDrawing*.vml`) so comment indicators / form controls / pre-2007 shapes render.
- Form controls + OLE objects (`<oleObjects>`, `<controls>`) as static placeholders.
- `contentPart`, nested `graphicFrame`, `prstTxWarp` / WordArt, SmartArt/diagrams (`dgm`) remain separate larger work.

## Suggested PARITY.md one-line status

Shapes should remain 🟡 until at least connectors, style refs, text insets, and a dedicated fixture corpus land. Current v0 is good for basic callouts/buttons and grouped screenshot chrome, but it is not yet broad DrawingML parity.
