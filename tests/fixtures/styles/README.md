# styles fixtures

## `named-inheritance.xlsx`

Exercises `cellStyleXf` inheritance via the `apply*="0"` flags
(ECMA-376 §18.8.45). Four cells in column A; each cell xf carries
default ids (`fontId="0"` / `fillId="0"` / `borderId="0"`) plus an
`xfId="N"` pointing at a `cellStyleXfs[N]` entry that holds the real
formatting. The relevant `apply*` flag is set to `"0"` so the
renderer must walk back to the parent style:

| Cell | xfId | `apply*="0"` | parent style       | expected render |
|------|------|--------------|--------------------|-----------------|
| A2   | 1    | `applyFont`  | font 2 (Calibri 18 bold #1F4E79)        | "Title" — large bold dark-blue |
| A3   | 2    | `applyFont`, `applyBorder` | font 3 (Calibri 14 italic #2E75B6) + thick blue bottom border | "Heading 1" — italic blue + bottom rule |
| A4   | 3    | `applyFill`  | fill 2 (yellow #FFE699 solid)           | yellow-filled cell |
| A5   | 4    | `applyAlignment` | alignment center/center             | "Centered" centered horizontally + vertically |

### Build

```bash
tests/fixtures/styles/build-named-inheritance.sh
```

`hsx` (and most Excel writers) flatten `apply*` inheritance at
write-time — they copy the parent's `fontId` / `fillId` / etc. into
the cell xf and emit `applyFont="1"`. That makes the unflattened path
(the only one that exercises the new code) impossible to produce
through SpreadJS's public API, so the build script post-patches
`xl/styles.xml` and the worksheet directly via Python zip-edit.

### hsx divergence

`hsx` (SpreadJS) ignores `apply*="0"` and renders all four cells as
plain Calibri 11 left-aligned with no fill or border. Excel desktop
honors the inheritance and shows the styled cells. We picked the
Excel side per `PARITY.md`'s `hsx-vs-Excel` rule.
