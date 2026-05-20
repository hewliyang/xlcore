#!/usr/bin/env python3
"""Rewrite the `absolutePos` shape from twoCellAnchor to absoluteAnchor."""
import os
import re
import sys
import tempfile
import zipfile

PATH = sys.argv[1]
DRAWING = "xl/drawings/drawing1.xml"

# EMU offsets from sheet origin (A1 top-left): x=320px, y=160px at 96dpi → ~304800, 152400 EMU
# Extent: 160×90 px → ~1524000 × 857250 EMU
POS_X = 304800
POS_Y = 152400
EXT_CX = 1524000
EXT_CY = 857250

with zipfile.ZipFile(PATH, "r") as zin:
    raw = zin.read(DRAWING).decode("utf-8")
    others = {n: zin.read(n) for n in zin.namelist() if n != DRAWING}

name_re = re.compile(r'<xdr:cNvPr\b[^>]*\bname="([^"]+)"')
anchor_re = re.compile(
    r"<xdr:twoCellAnchor>.*?</xdr:twoCellAnchor>", re.DOTALL
)


def is_absolute_shape(block: str) -> bool:
    m = name_re.search(block)
    return bool(m and m.group(1) == "absolutePos")


def to_absolute(block: str) -> str:
    sp_m = re.search(r"(<xdr:sp\b.*?</xdr:sp>)", block, re.DOTALL)
    if not sp_m:
        raise SystemExit("absolutePos anchor: missing <xdr:sp>")
    sp = sp_m.group(1)
    client = "<xdr:clientData/>"
    return (
        "<xdr:absoluteAnchor>"
        f'<xdr:pos x="{POS_X}" y="{POS_Y}"/>'
        f'<xdr:ext cx="{EXT_CX}" cy="{EXT_CY}"/>'
        f"{sp}{client}"
        "</xdr:absoluteAnchor>"
    )


out = []
last = 0
patched = False
for m in anchor_re.finditer(raw):
    out.append(raw[last : m.start()])
    block = m.group(0)
    if is_absolute_shape(block):
        out.append(to_absolute(block))
        patched = True
    else:
        out.append(block)
    last = m.end()
out.append(raw[last:])
raw = "".join(out)

if not patched:
    raise SystemExit("did not find absolutePos twoCellAnchor to rewrite")
if "<xdr:absoluteAnchor>" not in raw:
    raise SystemExit("absoluteAnchor missing after patch")

fd, tmp = tempfile.mkstemp(suffix=".xlsx")
os.close(fd)
with zipfile.ZipFile(tmp, "w", zipfile.ZIP_DEFLATED) as zout:
    for n, data in others.items():
        zout.writestr(n, data)
    zout.writestr(DRAWING, raw)
os.replace(tmp, PATH)
print(f"patched {PATH}")
