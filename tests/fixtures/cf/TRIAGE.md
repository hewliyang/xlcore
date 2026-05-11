# cf/ fixtures — known divergences

Per-fixture notes on places where our extractor or renderer departs from
either the OOXML spec or the `hsx` (SpreadJS) ground truth, and why.
Each entry should resolve to either "fix it" or "punt and document".

## icon-set.xlsx

### `hsx` writes user-set iconCriteria as empty `<cfvo/>`

The `IconSetRule.iconCriteria([...])` setter does not round-trip into
the legacy `<x:iconSet>` block. The xlsx hsx emits looks like

```xml
<iconSet iconSet="3Symbols">
  <cfvo type="percent" val="0"/>
  <cfvo/>
  <cfvo/>
</iconSet>
```

The real thresholds live in the x14 extension (which we don't parse
yet), and `ooxmlsdk` rejects the malformed legacy block entirely —
`x_icon_set` comes back as `None`, dropping the rule.

Workaround in `build-icon-set.sh`: don't override `iconCriteria()`. The
SpreadJS defaults (3-set 33/67%, 4-set 25/50/75%, 5-set 20/40/60/80%)
are what Excel uses out of the box, so the fixture exercises the
realistic case anyway. When x14 parsing lands, expand the fixture with
custom thresholds.

### Renderer

- **Glyphs are hand-drawn canvas paths, not the Office-2007 PNG
  atlas.** Visual flavor (curve angles on arrows, exclamation-circle
  proportions on `3Symbols`, gradient rim on traffic lights) differs
  from `hsx`. Bucket assignment, color ramp, and reverse semantics all
  match.
- **Unsupported preset → fallback colored circle.** Anything we
  haven't carved a path for (`4RedToBlack` partial, `3Triangles`,
  `3Stars`, `5Boxes`) falls back to either the closest neighbor
  drawer or a plain colored circle ramped along the set's index.
  Tracked here so a regression doesn't go unnoticed once the corpus
  expands.
- **Reserved icon-strip width is fixed at 18px.** Excel scales icon
  size with row height; we don't yet. Tall rows look icon-loose,
  shrunken rows can clip the right edge of the glyph.
- **No `iconSet` inside x14 extension.** Same blocker as data-bar:
  the canonical block lives in `<x14:cfRule>` and we ignore it.
  Custom icon assignments (mixing icons across sets) and `icons[]`
  reorder both stored there.

## data-bar.xlsx

### hsx writes incomplete `<dataBar>` XML

The legacy `<x:cfRule type="dataBar"><dataBar>` block hsx produces
omits the **required** `<color>` child element and the
`minLength`/`maxLength` attributes. The canonical color and length
bounds live in the x14 extension (`<x14:dataBar minLength="0"
maxLength="100"><x14:negativeFillColor .../></x14:dataBar>`) which
ships alongside the legacy block.

Two consequences:

1. **`ooxmlsdk` silently drops the entire `<dataBar>` element** when
   `<color>` is missing — `x_data_bar` comes back as `None` even
   though the CF rule is clearly there. The build script
   (`build-data-bar.sh`) injects a default `<color rgb="FF638EC6"/>`
   into each `<dataBar>` block via a post-process zip-patch so the
   fixture is conformant. Same workaround `themes/` uses.

2. **The legacy `minLength`/`maxLength` defaults (`10`/`90` per ECMA-376
   §18.3.1.28) don't match what users see.** Excel itself, SpreadJS,
   LibreOffice, and Google Sheets all author the x14 extension with
   `0`/`100`, and that's what their renderers use. Our extractor
   defaults the legacy attrs to `0`/`100` to match observed behavior;
   strict-spec defaults are tracked here as a follow-up.

Open work tracked under "Bigger lifts" in PARITY.md → "x14 extension
parsing": when we add it, drop both workarounds.

### Renderer punts

- ~~**`gradient` (Excel 2010+ default) renders as solid.**~~ **DONE.**
  Renderer now paints a `createLinearGradient` from the bar's anchor
  edge to its tip with stops `color@1.0 → color@0.8 at 70% → color@0.05`,
  matching the visual character SpreadJS produces. New schema field
  `CfDataBar.gradient: bool` defaults true; when x14 parsing lands
  the extractor will read `<x14:dataBar gradient="..."/>` and only
  fall back to a flat fill when explicitly disabled.
- **Negative bar color = hard-coded `#FF0000`.** Real files store the
  per-rule negative color in the x14 extension's `negativeFillColor`.
  Until that's parsed, all negative bar segments paint pure red,
  matching Excel's default but ignoring user customization.
- **Axis tick when range straddles zero is 1px black.** Excel paints
  the axis at the user-specified `axisColor` (also in x14). Same
  fate — defaults until x14 lands.
- **`min`/`max` cfvo types zero-clamp.** Per a strict reading of
  ECMA-376, `<cfvo type="min"/>` resolves to the actual minimum value
  in the range. Excel's renderer reads the parallel x14
  `<cfvo type="automin"/>` instead, which clamps at zero
  (`min(0, dataMin)`). We apply the clamp to legacy `min`/`max` to
  match what every real-world file actually displays. Toggle when we
  parse x14.

## cf-non-recalc.xlsx

### `hsx` CF emission has four bugs that ooxmlsdk strict-rejects

`build-cf-non-recalc.sh` post-patches the worksheet XML to fix:

1. **Empty `sqref=""` on top10 rules.** `addTop10Rule(type, rank, dxf, ranges)`
   takes the ranges as the *fourth* arg. SpreadJS's docs and several
   community examples list a `percent` flag in there too; that signature
   doesn't exist in our `hsx` build, so calls that pass `(type, rank,
   percent, dxf, ranges)` silently land `percent` in the `dxf` slot,
   skip the ranges arg, and emit `sqref=""`. Confirmed by inspecting
   `addTop10Rule.toString()` in the `hsx eval` REPL.

2. **Missing `dxfId` on top10 rules.** Even with the correct signature,
   the styled rule writes `<cfRule type="top10" rank="N" text="null"/>`
   with no `dxfId` attribute. We inject the right index based on
   creation order.

3. **`type="containsText"` for all four text-rule kinds.** `addSpecificTextRule`
   hard-codes the type attribute regardless of the `TextCompareType`
   passed in (the actual semantics live in the rule's `<formula>`).
   We rewrite to `containsText` / `notContainsText` / `beginsWith` /
   `endsWith` based on declaration-order priority.

4. **`operator="contains"` is not a valid enum value.** ECMA-376 says
   `containsText`. We rewrite the attribute.

5. **Bogus `text="null"` literal attribute.** Stripped wholesale.
