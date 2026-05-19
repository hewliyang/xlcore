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
| Fixture corpus | ✅ | `tests/fixtures/shapes/`: `basic-autoshapes.xlsx`, `textbox-wrap-align.xlsx`, `connectors.xlsx`, `style-refs-themed.xlsx`, `groups-and-pictures.xlsx`, `list-style-inheritance.xlsx`, plus three EPPlus-authored gap fixtures — `gradient-fills.xlsx`, `outer-shadow.xlsx`, `shape-flips.xlsx`. Each with `.hsx.png` ground truth + `.ours.png` baseline. The EPPlus path lives at `tests/fixtures/shapes/dotnet-builder/FixtureBuilder/` for features SpreadJS's public API can't author (gradients, shape effects, non-connector flips). |

## Known v0 shortcuts

Consolidated list of deliberate carve-outs. Most landed under e-007. Each item is shipped as far as the bullet describes; the rest is the v0 cheat.

*(2026-05-19: items #7 and the new "missing xfrm / xfrm without off+ext" extractor bug were resolved while adding the EPPlus fixture corpus — the painter now honors `flipH`/`flipV` on every shape kind, and the extractor falls back to anchor geometry when xfrm is absent or partial. Both were exposed by `shapes/shape-flips.xlsx`.)*

1. **`stCxn`/`endCxn` connection sites** — only the 4 cardinal sites (top/right/bottom/left center) are resolved against the target bbox. Enough for `rect`/`roundRect`/`ellipse` receivers (org-chart / SOTP). Skipped: preset-aware sites (chevron tip, star points, flowchart non-cardinal), custom `cxnLst` declared on the shape XML, multi-segment `bentConnector{2,4,5}` re-routing.
2. **Brace/bracket presets** — only `leftBrace` / `rightBrace` / `leftBracket` / `rightBracket`. Missing: `bracePair`, `bracketPair`, diagonal bracket variants.
3. **`avLst` adjust values** — `adj1`+`adj2` extracted on every shape, but only the brace painter honors them. `roundRect` still uses a fixed 16% radius; arrow heads/tails, callouts, stars, arcs keep hardcoded defaults.
4. **`vertOverflow`** — hardcoded DrawingML default `overflow` (line paints if its top is inside the body rect). Explicit `vertOverflow="clip"` and `horzOverflow` unmodeled.
5. **`lstStyle` cascade** — inherits only size / bold / italic / underline / strike / solidFill color / latin font. Ignores `marL`, `indent`, `lnSpc`, `spcBef`, `spcAft`, kerning, baseline, run-`u="none"`-as-disable-inherited, and the entire bullet list.
6. **Style refs (`a:style`)** — does not walk `<a:fmtScheme><a:fillStyleLst>` / `<a:lnStyleLst>`. Every `fillRef idx≥1` is treated as flat `solidFill phClr` (correct for the standard theme's idx=1; loses gradients on idx 2/3 and per-style line dashes).
7. ~~**`flipH/V`** — applied to connectors only. Non-connector shape flips ignored.~~ **Shipped.** Painter applies `ctx.scale(±1,±1)` around shape centre before geometry; unflips before text. Text body rect doesn't follow the flip yet (caption position is off on asymmetric presets like arrows), tracked as a follow-up under P1 #4.
8. **`prstDash`** — extracted+rendered on connectors/lines only. Non-connector shape outlines don't read dash. `custDash` fully ignored.
9. **Line cap/join** — connector painter hardcodes `cap=butt, join=miter`; brace painter forces `round`. No reading of `a:ln@cap`, `a:round`/`a:bevel`/`a:miter`.
10. **`a:fld`** — handled as a cached-text run (we display the cached `<a:t>`). The `textlink` formula is not evaluated; harmless for preview because OOXML stores the latest evaluated value in the field.

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
| Group rotation | ❌ | ❌ | P1 | `CT_GroupTransform2D@rot` ignored by frame mapping. |
| Flip H/V on shape `xfrm` | ✅ | 🟡 | P1 | Geometry now flips on both connector and non-connector paths. The painter applies `ctx.scale(±1, ±1)` around the shape centre before drawing the path, then unflips before drawing text so labels stay readable — matches HSX/Excel for `flipH`. **Remaining divergence:** the text body rect is not also flipped, so on a `flipH` right-arrow the caption sits over the left half (in HSX it sits over the right half because the body rect follows the geometry). Visible only on shapes whose preset path defines a non-centred text rect (arrows, callouts). Locked in by `shapes/shape-flips.xlsx`. |
| Z-order | ✅ | ✅ | P0 | Preserved from XML traversal order. |
| Clipping to group/shape | ❌ | ❌ | P2 | Flattened model does not clip children to group bounds. |
| `bwMode` | ❌ | ❌ | P3 | Rare in spreadsheets. |

### Geometry

| Feature | Extract | Render | Priority | Notes |
| --- | --- | --- | --- | --- |
| `prstGeom`: `rect`, `roundRect`, `ellipse`, `triangle`, `diamond` | ✅ | ✅ | P0 | `roundRect` uses hardcoded default radius — see shortcut #3. |
| Basic block arrows (`leftArrow`/`rightArrow`/`upArrow`/`downArrow`) | ✅ | ✅ | P0 | Hardcoded default adjusts. |
| Lines (`line` / `lineInv`) | ✅ | ✅ | P0 | Routed through connector painter; honors `flipH/V`, dash, arrowheads. |
| Common extras (`chevron`, `pentagon`, `hexagon`, `star5`, `leftRightArrow`) | ✅ | ✅ | P1 | Shipped in `abbebdd`. |
| Braces / brackets | 🟡 | 🟡 | P1 | See shortcut #2. |
| Long-tail presets | ❌ | ❌ | P1 | Spec lists 187. Big ones still missing: flowchart symbols, action buttons, callouts, stars beyond star5, arc/donut, plaque/bevel. |
| `avLst` adjust values | 🟡 | 🟡 | P1 | See shortcut #3. |
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
| Blip fills `blipFill` | ❌ | ❌ | P1 | Shape-as-image-fill; distinct from `xdr:pic`. Textured buttons/banners. |
| Group fill `grpFill` | ❌ | ❌ | P2 | Inherit/transform from parent group. |
| Basic line color + width | ✅ | ✅ | P0 | `a:ln` solid/noFill + width. |
| Line dash (`prstDash`) | 🟡 | 🟡 | P1 | See shortcut #8. |
| Line cap/join | ❌ | ❌ | P1 | See shortcut #9. |
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
| `vertOverflow` / `horzOverflow` | 🟡 | 🟡 | P1 | See shortcut #4. |
| Body margins/insets (`bodyPr lIns/tIns/rIns/bIns`) | ✅ | ✅ | P0 | DrawingML defaults backfilled (91440 / 45720 / 91440 / 45720 EMU). |
| Text autofit (`normAutofit` / `spAutoFit`) | ❌ | ❌ | P1 | Affects cramped boxes. |
| Text rotation / vertical text (`bodyPr@rot`, `vert`) | ❌ | ❌ | P1 | Separate from shape rotation. |
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
| SVG sidecar (`asvg:svgBlip`) | ❌ | ❌ | P1 | Modern Office stores raster fallback + SVG inside `blip/extLst`. We pick raster; SVG would be crisper at scale. |
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
4. **Group rotation + body-rect-follows-flip** — the non-connector `flipH/V` painter shipped; remaining gaps are (a) `<a:xfrm rot="..."/>` on `<xdr:grpSpPr>` (children should rotate as a rigid body), (b) flipping the shape's text body rect so captions on asymmetric presets (arrows, callouts) sit on the visually-correct side.
5. **Preset dash + line cap/join on non-connector outlines** — small surface, fixes thin-line shape appearance. Resolves shortcuts #8, #9.
6. **`avLst` for `roundRect` / arrows / callouts** — small surface, fixes already-supported presets. Resolves shortcut #3.
7. **Text autofit (`normAutofit` font scaling, `spAutoFit`)** — fixes cramped labels everywhere.
8. **Text rotation / vertical text (`bodyPr@rot`, `vert`)** — common on chart-adjacent labels.
9. **`vertOverflow="clip"` + `horzOverflow`** — finish off shortcut #4.
10. **Blip fills (`blipFill`) + SVG sidecar (`asvg:svgBlip`)** — modern-Office icon and textured-banner fidelity.
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

Shapes remain 🟡 until the style-ref matrix walk lands (P1 #3; gradient fills and direct-`<a:effectLst>` outer shadow shipped in P1 #1–2). Current v0 is good for basic callouts/buttons, themed gradients + drop shadows on direct DrawingML shapes, grouped screenshot chrome, and Office-authored org-chart / SOTP diagrams; not yet broad DrawingML parity (theme-driven `effectRef` shadows are the next obvious gap).
