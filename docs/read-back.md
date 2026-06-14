# Read-back: Info↔Patch symmetry

The write API is partial and merging; the read API should be its total inverse.
Today it isn't — `set_style` takes a `StylePatch` and merges it onto the cell's
xf, but `get_cell` returns only `style_index: u32`, an opaque pointer into
`cellXfs`. Writers are flat and human; readers leak the XML index. That gap is
why a mutable object wrapper (`c.font.bold`) can't have getters.

## Principle

For every domain, `Info` is the total resolution of the same shape a `Patch`
writes partially:

    resolve(get(cell).style) ⊇ every field set by set(cell, patch)

`Info ≈ Patch with all defaults filled in`. The wrapper never sees indices,
xfIds, theme refs, or numFmtIds — only resolved scalars it can also write back.

## What it takes

One keystone: the inverse of the write path. `apply_patch_to_xf` /
`build_font` / `build_fill` / `build_border` each gain a mirror
(`xf_to_style_patch`, `font_to_patch`, …) that walks an interned xf back to a
flat `StylePatch`, resolving the `xfId` named-style master first, then layering
the cell's own xf on top. Expose it as `CellInfo.style: Option<StylePatch>` so
`get_cell` is total.

Setters need nothing new: `set_style` already merges, so `c.font.bold = True`
sends `{font: {bold: True}}`. **Read-back is the only missing direction.**

## The hard part

Resolution must collapse indirection the writer accepts but the reader must
flatten:

- colors: `indexed` / `theme + tint` → `RRGGBB` (needs `theme1.xml` + palette).
- numbers: built-in `numFmtId` → format code (id table; no stored string).
- named styles: `xfId` master + direct overrides → one merged patch.

Decide the contract: resolve to concrete values (great DX, round-trips
*semantically* not byte-for-byte) or pass refs through raw (faithful, useless).
Resolve. Timebox theme-color edge cases; fall back to raw refs for exotica.

## Why it's not binding tax

This completes an already-stated principle (Info/Patch symmetry) and improves
every binding + the core API, independent of any heavy Python wrapper. Sequence
it first: read-back is justified on its own, and a heavy wrapper built on a
write-only core only reproduces the write-only-property wart at scale.

## Test = spec

A round-trip corpus is both the guarantee and the resolver's spec:

    for patch in corpus: set(cell, patch); assert resolve(get(cell)) ⊇ patch

Symmetry enforced, not hoped for. Styles is ~90% of the work (xfId + color
mess); charts/tables/pivots already return near-symmetric `Info`.

## Backlog

Sequenced; each item is one ralph-agent task. Verify: `cargo test -p xlcore-api`
+ `cargo build --workspace`.

1. **Inverse converters + expose `CellInfo.style`.** Mirror the write path:
   `xf_to_style_patch(doc, xf) -> StylePatch` with `font_to_patch` / `fill_to_patch`
   / `border_to_patch` / alignment / protection / number_format, walking an interned
   `x::CellFormat` back to a flat `StylePatch`. Resolve the `xfId`/`format_id`
   named-style master from `cellStyleXfs` first, then layer the cell's own xf on top.
   Colors via item 1; numFmt via existing `num_fmt_code`. Add `style: Option<StylePatch>`
   to `CellInfo` (xlcore-types) and populate it in `get_cell` (style resolution needs
   `doc`, so resolve in `cells.rs::get_cell`, not the doc-less `cell_info_from_cell`).
   Regen TS schema (`scripts/regen-api-schema.sh`). Unit test: a styled cell returns a
   `StylePatch` matching what was set.

## Shipped

- Color resolver: `resolve_color_hex` (rgb/indexed/theme+tint → `RRGGBB`) in xlcore-api.
- Inverse converters (`xf_to_style_patch` + font/fill/border/alignment/protection) and
  `CellInfo.style: Option<StylePatch>` populated in `get_cell`.
- Round-trip corpus test (`tests/style_roundtrip.rs`) enforcing `resolve(get(cell)) ⊇
  set(cell, patch)` across font/fill/border/alignment/number_format/protection +
  named-style master. Colors normalized (`#`/`FF` alpha stripped, uppercased); solid
  `fill.color` read back as `pattern=Solid` + `foreground`; `border.all` read back as
  the four sides. Excluded as non-round-tripping (separate explicit tests instead):
  a `None` border side clears to no border (not a `None`-styled side), and
  `font.underline=None` / `vertAlign=Baseline` emit nothing.
