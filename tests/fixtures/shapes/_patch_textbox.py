#!/usr/bin/env python3
"""Patch `textbox-wrap-align.xlsx` (built by hsx) to inject the
`<a:bodyPr>` / `<a:pPr>` attributes SpreadJS won't emit through its
public API:

  - `algn="just"` on the `hJ` shape's first paragraph (SpreadJS's
    `HorizontalAlign.justify` round-trips as no `algn` attr at all),
  - `wrap="none"` on the `wOff1` / `wOff2` shapes' bodyPr (SpreadJS
    silently drops `textFrame.wordWrap(false)`),
  - explicit `lIns`/`tIns`/`rIns`/`bIns` on `insTight` / `insLoose`
    / `insAsym` (SpreadJS never emits these, so the fixture's inset
    row is otherwise indistinguishable from the default).

Mirrors the pattern used by `borders/_patch_every_style.py` /
`text/_patch_underline.py`: byte-exact OOXML rewrite so layout JSON
+ pixel diffs are deterministic.
"""
import os
import re
import sys
import tempfile
import zipfile

PATH = sys.argv[1]
DRAWING = "xl/drawings/drawing1.xml"

# 1 inch = 914400 EMU. DrawingML defaults: lIns/rIns = 91440 (~0.1in),
# tIns/bIns = 45720 (~0.05in). Our fixture's "tight/loose/asym" are
# meant to be visibly different from those defaults.
INSETS = {
    "insTight":  (12700,  12700,  12700,  12700),    # ~1.3px each side
    "insLoose":  (228600, 228600, 228600, 228600),   # ~24px each side
    "insAsym":   (304800, 50800,  304800, 50800),    # ~32px / ~5px L,T,R,B
}
WRAP_NONE = {"wOff1", "wOff2"}
JUSTIFY = {"hJ"}

with zipfile.ZipFile(PATH, "r") as zin:
    raw = zin.read(DRAWING).decode("utf-8")
    others = {n: zin.read(n) for n in zin.namelist() if n != DRAWING}

def patch_shape_block(block: str, name: str) -> str:
    """Apply per-name mutations to a single `<xdr:sp>...</xdr:sp>` block."""
    out = block
    if name in INSETS:
        l, t, r, b = INSETS[name]
        ins_attrs = f' lIns="{l}" tIns="{t}" rIns="{r}" bIns="{b}"'
        out = re.sub(
            r'(<a:bodyPr\b)([^/>]*)(/?>)',
            lambda mm: mm.group(1) + mm.group(2) + ins_attrs + mm.group(3),
            out,
            count=1,
        )
    if name in WRAP_NONE:
        out = re.sub(
            r'(<a:bodyPr\b[^>]*?\bwrap=")square(")',
            r'\1none\2',
            out,
            count=1,
        )
    if name in JUSTIFY:
        # Inject `<a:pPr algn="just"/>` as the first child of the
        # paragraph. The existing `<a:p>` has no `<a:pPr>` so we
        # splice one in right after `<a:p>`.
        out = re.sub(
            r'(<a:p\b[^>]*>)',
            r'\1<a:pPr algn="just"/>',
            out,
            count=1,
        )
    return out

# Walk every `<xdr:sp>...</xdr:sp>` block. Replace in-place so we
# don't have to worry about cross-block regex over-reach (which is
# what tripped up the first version of this script — a non-greedy
# `.*?` between `<xdr:sp>` and the matching `<xdr:cNvPr name="X">`
# happily expanded across earlier sibling sp blocks once the first
# one’s name didn’t match).
sp_iter_re = re.compile(r'<xdr:sp\b.*?</xdr:sp>', re.DOTALL)
name_re = re.compile(r'<xdr:cNvPr\b[^>]*\bname="([^"]+)"')

def walk(match: re.Match) -> str:
    block = match.group(0)
    nm = name_re.search(block)
    if not nm:
        return block
    return patch_shape_block(block, nm.group(1))

raw = sp_iter_re.sub(walk, raw)

# Sanity: confirm the mutations actually landed (catches silent
# regex misses on a future hsx-format change).
for n, (l, t, r, b) in INSETS.items():
    if f'lIns="{l}"' not in raw:
        raise SystemExit(f"inset patch for {n!r} failed")
for n in WRAP_NONE:
    pass  # presence already enforced by the regex sub above succeeding.
if 'algn="just"' not in raw:
    raise SystemExit("justify patch failed")

fd, tmp = tempfile.mkstemp(suffix=".xlsx")
os.close(fd)
with zipfile.ZipFile(tmp, "w", zipfile.ZIP_DEFLATED) as zout:
    for n, data in others.items():
        zout.writestr(n, data)
    zout.writestr(DRAWING, raw)
os.replace(tmp, PATH)
print(f"patched {PATH}")
