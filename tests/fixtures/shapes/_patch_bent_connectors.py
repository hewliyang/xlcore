#!/usr/bin/env python3
import os
import re
import sys
import tempfile
import zipfile

PATH = sys.argv[1]
DRAWING = "xl/drawings/drawing1.xml"

TARGETS = {
    "b2_conn": ("bentConnector2", []),
    "b3_conn": ("bentConnector3", [("adj1", 50000)]),
    "b4_conn": ("bentConnector4", [("adj1", 50000), ("adj2", 50000)]),
    "b5_conn": (
        "bentConnector5",
        [("adj1", 33000), ("adj2", 50000), ("adj3", 67000)],
    ),
}

with zipfile.ZipFile(PATH, "r") as zin:
    raw = zin.read(DRAWING).decode("utf-8")
    others = {n: zin.read(n) for n in zin.namelist() if n != DRAWING}

cxn_re = re.compile(r"<xdr:cxnSp>.*?</xdr:cxnSp>", re.DOTALL)
name_re = re.compile(r'<xdr:cNvPr\b[^>]*\bname="([^"]+)"')
prst_re = re.compile(r"<a:prstGeom[^>]*>.*?</a:prstGeom>", re.DOTALL)


def build_prstgeom(prst, adjs):
    if not adjs:
        return f'<a:prstGeom prst="{prst}"><a:avLst/></a:prstGeom>'
    gds = "".join(f'<a:gd name="{n}" fmla="val {v}"/>' for n, v in adjs)
    return f'<a:prstGeom prst="{prst}"><a:avLst>{gds}</a:avLst></a:prstGeom>'


out = []
last = 0
patched = set()
for m in cxn_re.finditer(raw):
    out.append(raw[last : m.start()])
    block = m.group(0)
    nm = name_re.search(block)
    if nm and nm.group(1) in TARGETS:
        prst, adjs = TARGETS[nm.group(1)]
        new_geom = build_prstgeom(prst, adjs)
        new_block, n = prst_re.subn(new_geom, block, count=1)
        if n != 1:
            raise SystemExit(f"connector {nm.group(1)}: missing <a:prstGeom>")
        block = new_block
        patched.add(nm.group(1))
    out.append(block)
    last = m.end()
out.append(raw[last:])
raw = "".join(out)

missing = set(TARGETS) - patched
if missing:
    raise SystemExit(f"did not patch all connectors; missing: {sorted(missing)}")

fd, tmp = tempfile.mkstemp(suffix=".xlsx")
os.close(fd)
with zipfile.ZipFile(tmp, "w", zipfile.ZIP_DEFLATED) as zout:
    for n, data in others.items():
        zout.writestr(n, data)
    zout.writestr(DRAWING, raw)
os.replace(tmp, PATH)
print(f"patched {PATH}: {sorted(patched)}")
