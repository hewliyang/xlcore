#!/usr/bin/env python3
"""Patch `text-autofit.xlsx` (built by hsx) to inject the autofit
choice inside each shape's `<a:bodyPr>`. SpreadJS only ever emits
`<a:noAutofit/>` (or no choice at all) — we need explicit
`<a:normAutofit fontScale lnSpcReduction>` / `<a:spAutoFit/>` to
exercise P1 #7 in `docs/parity-shapes.md`.

Mirrors `_patch_textbox.py`: byte-exact OOXML rewrite so the layout
JSON and pixel diffs are deterministic.
"""
import os
import re
import sys
import tempfile
import zipfile

PATH = sys.argv[1]
DRAWING = "xl/drawings/drawing1.xml"

# Maps shape@name → replacement inner-XML for the bodyPr autofit slot.
# fontScale is 1000..100000 (1%..100%); lnSpcReduction is 0..13200000.
PATCHES = {
    "fs100":    '<a:normAutofit fontScale="100000"/>',
    "fs75":     '<a:normAutofit fontScale="75000"/>',
    "fs50":     '<a:normAutofit fontScale="50000"/>',
    "fs50ln20": '<a:normAutofit fontScale="50000" lnSpcReduction="20000"/>',
    "fs25":     '<a:normAutofit fontScale="25000"/>',
    "spAuto":   '<a:spAutoFit/>',
}

with zipfile.ZipFile(PATH, "r") as zin:
    raw = zin.read(DRAWING).decode("utf-8")
    others = {n: zin.read(n) for n in zin.namelist() if n != DRAWING}


def patch_block(block: str, name: str) -> str:
    """Replace `<a:bodyPr …/>` (self-closing) or any existing autofit
    child inside `<a:bodyPr …> … </a:bodyPr>` with our marker."""
    inner = PATCHES.get(name)
    if not inner:
        return block

    # First strip any existing autofit child Spread might have written.
    block = re.sub(
        r'<a:(noAutofit|normAutofit|spAutoFit)\b[^/>]*/>',
        '',
        block,
    )

    # Convert a self-closing `<a:bodyPr …/>` into an opening+closing
    # pair so we can drop the autofit child inside it.
    def open_close(m: re.Match) -> str:
        return f"<a:bodyPr{m.group(1)}>{inner}</a:bodyPr>"

    new, n = re.subn(
        r'<a:bodyPr\b([^/>]*)/>',
        open_close,
        block,
        count=1,
    )
    if n == 1:
        return new

    # Otherwise inject as the first child of the existing pair.
    new, n = re.subn(
        r'(<a:bodyPr\b[^>]*>)',
        lambda m: m.group(1) + inner,
        block,
        count=1,
    )
    if n != 1:
        raise SystemExit(f"could not find <a:bodyPr> for shape {name!r}")
    return new


sp_iter_re = re.compile(r'<xdr:sp\b.*?</xdr:sp>', re.DOTALL)
name_re = re.compile(r'<xdr:cNvPr\b[^>]*\bname="([^"]+)"')


def walk(match: re.Match) -> str:
    block = match.group(0)
    nm = name_re.search(block)
    if not nm:
        return block
    return patch_block(block, nm.group(1))


raw = sp_iter_re.sub(walk, raw)

# Sanity: each requested fontScale must appear somewhere.
for name, snippet in PATCHES.items():
    if snippet not in raw:
        raise SystemExit(f"autofit patch for {name!r} missing in output")

fd, tmp = tempfile.mkstemp(suffix=".xlsx")
os.close(fd)
with zipfile.ZipFile(tmp, "w", zipfile.ZIP_DEFLATED) as zout:
    for n, data in others.items():
        zout.writestr(n, data)
    zout.writestr(DRAWING, raw)
os.replace(tmp, PATH)
print(f"patched {PATH}")
