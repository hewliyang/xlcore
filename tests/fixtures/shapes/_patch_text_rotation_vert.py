#!/usr/bin/env python3
"""Patch `text-rotation-vert.xlsx` (built by hsx) to inject
`<a:bodyPr rot>` / `<a:bodyPr vert>` attributes SpreadJS won't emit
through its public API.

Mirrors `_patch_text_autofit.py`: byte-exact OOXML rewrite so the
layout JSON and pixel diffs are deterministic.

`rot` is in 1/60000 of a degree (same units as `<a:xfrm rot>`).
"""
import os
import re
import sys
import tempfile
import zipfile

PATH = sys.argv[1]
DRAWING = "xl/drawings/drawing1.xml"

DEG = 60000

# shape@name → (rot_in_60000ths_of_deg or None, vert_token or None)
PATCHES = {
    "rot0":    (0,             None),
    "rotP45":  (45  * DEG,     None),
    "rotP90":  (90  * DEG,     None),
    "rotP180": (180 * DEG,     None),
    # OOXML rot is unsigned 0..21600000 (0..360°). Negative angles
    # round-trip as 360 - angle.
    "rotN90":  (270 * DEG,     None),
    "rotN45":  (315 * DEG,     None),
    "vert":    (None,          "vert"),
    "vert270": (None,          "vert270"),
    "eaVert":  (None,          "eaVert"),
}

with zipfile.ZipFile(PATH, "r") as zin:
    raw = zin.read(DRAWING).decode("utf-8")
    others = {n: zin.read(n) for n in zin.namelist() if n != DRAWING}


def patch_block(block: str, name: str) -> str:
    cfg = PATCHES.get(name)
    if not cfg:
        return block
    rot, vert = cfg

    extra = ""
    if rot is not None:
        extra += f' rot="{rot}"'
    if vert is not None:
        extra += f' vert="{vert}"'
    if not extra:
        return block

    # Strip any pre-existing rot / vert attrs from `<a:bodyPr …>`
    # before injecting ours, so re-runs don't duplicate.
    def scrub(m: re.Match) -> str:
        attrs = m.group(2)
        attrs = re.sub(r'\s+rot="[^"]*"', "", attrs)
        attrs = re.sub(r'\s+vert="[^"]*"', "", attrs)
        return m.group(1) + attrs + extra + m.group(3)

    new, n = re.subn(
        r'(<a:bodyPr\b)([^/>]*)(/?>)',
        scrub,
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

# Sanity: each requested rot / vert value must appear in the output.
for name, (rot, vert) in PATCHES.items():
    if rot is not None and f'rot="{rot}"' not in raw:
        raise SystemExit(f"rot patch for {name!r} missing in output")
    if vert is not None and f'vert="{vert}"' not in raw:
        raise SystemExit(f"vert patch for {name!r} missing in output")

fd, tmp = tempfile.mkstemp(suffix=".xlsx")
os.close(fd)
with zipfile.ZipFile(tmp, "w", zipfile.ZIP_DEFLATED) as zout:
    for n, data in others.items():
        zout.writestr(n, data)
    zout.writestr(DRAWING, raw)
os.replace(tmp, PATH)
print(f"patched {PATH}")
