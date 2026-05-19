#!/usr/bin/env python3
import os
import re
import sys
import tempfile
import zipfile

PATH = sys.argv[1]
DRAWING = "xl/drawings/drawing1.xml"

PATCHES = {
    "vOverflow": {"vertOverflow": "overflow", "horzOverflow": "overflow", "wrap": "square"},
    "vClip":     {"vertOverflow": "clip",     "horzOverflow": "overflow", "wrap": "square"},
    "vEllipsis": {"vertOverflow": "ellipsis", "horzOverflow": "overflow", "wrap": "square"},
    "hOverflow": {"vertOverflow": "overflow", "horzOverflow": "overflow", "wrap": "none"},
    "hClip":     {"vertOverflow": "overflow", "horzOverflow": "clip",     "wrap": "none"},
    "hEllipsis": {"vertOverflow": "ellipsis", "horzOverflow": "clip",     "wrap": "none"},
}

with zipfile.ZipFile(PATH, "r") as zin:
    raw = zin.read(DRAWING).decode("utf-8")
    others = {n: zin.read(n) for n in zin.namelist() if n != DRAWING}


def set_attrs(open_tag: str, attrs: dict) -> str:
    out = open_tag
    for k, v in attrs.items():
        pat = re.compile(rf'\s{re.escape(k)}="[^"]*"')
        out = pat.sub("", out)
    inject = "".join(f' {k}="{v}"' for k, v in attrs.items())
    if out.endswith("/>"):
        return out[:-2] + inject + "/>"
    return out[:-1] + inject + ">"


def patch_block(block: str, name: str) -> str:
    attrs = PATCHES.get(name)
    if not attrs:
        return block
    m = re.search(r"<a:bodyPr\b[^>]*?/?>", block)
    if not m:
        raise SystemExit(f"could not find <a:bodyPr> for shape {name!r}")
    new_open = set_attrs(m.group(0), attrs)
    return block[: m.start()] + new_open + block[m.end():]


sp_iter_re = re.compile(r"<xdr:sp\b.*?</xdr:sp>", re.DOTALL)
name_re = re.compile(r'<xdr:cNvPr\b[^>]*\bname="([^"]+)"')


def walk(match: re.Match) -> str:
    block = match.group(0)
    nm = name_re.search(block)
    if not nm:
        return block
    return patch_block(block, nm.group(1))


raw = sp_iter_re.sub(walk, raw)

for name, attrs in PATCHES.items():
    for k, v in attrs.items():
        if f'{k}="{v}"' not in raw:
            raise SystemExit(f"overflow patch for {name!r}: {k}={v!r} missing in output")

fd, tmp = tempfile.mkstemp(suffix=".xlsx")
os.close(fd)
with zipfile.ZipFile(tmp, "w", zipfile.ZIP_DEFLATED) as zout:
    for n, data in others.items():
        zout.writestr(n, data)
    zout.writestr(DRAWING, raw)
os.replace(tmp, PATH)
print(f"patched {PATH}")
