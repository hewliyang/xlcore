#!/usr/bin/env python3
"""Add cNvPr/hlinkClick + drawing rels for shape hyperlink fixture."""
import os
import re
import sys
import tempfile
import zipfile

PATH = sys.argv[1]
DRAWING = "xl/drawings/drawing1.xml"
RELS = "xl/drawings/_rels/drawing1.xml.rels"

HYPERLINKS = {
    "externalLink": {
        "rid": "rIdHyper1",
        "target": "https://example.com/shape-external",
        "tooltip": "External shape link",
    },
    "internalLink": {
        "rid": "rIdHyper2",
        "target": "",
        "location": "Sheet1!B5",
        "tooltip": "Jump to B5",
    },
}

with zipfile.ZipFile(PATH, "r") as zin:
    drawing = zin.read(DRAWING).decode("utf-8")
    rels = zin.read(RELS).decode("utf-8")
    others = {
        n: zin.read(n)
        for n in zin.namelist()
        if n not in (DRAWING, RELS)
    }

cnvpr_re = re.compile(r"<xdr:cNvPr\b[^>]*/>")
name_re = re.compile(r'\bname="([^"]+)"')

for shape_name, spec in HYPERLINKS.items():
    rid = spec["rid"]
    tooltip = spec.get("tooltip", "")
    tooltip_attr = f' tooltip="{tooltip}"' if tooltip else ""
    hlink = f'<a:hlinkClick r:id="{rid}"{tooltip_attr}/>'
    hit = [False]

    def inject(m: re.Match) -> str:
        tag = m.group(0)
        nm = name_re.search(tag)
        if not nm or nm.group(1) != shape_name:
            return tag
        hit[0] = True
        return tag[:-2] + f">{hlink}</xdr:cNvPr>"

    drawing = cnvpr_re.sub(inject, drawing)
    if not hit[0]:
        raise SystemExit(f"failed to inject hlinkClick for {shape_name!r}")

insert = ""
for spec in HYPERLINKS.values():
    rid = spec["rid"]
    target = spec["target"]
    if target:
        insert += (
            f'<Relationship Id="{rid}" '
            'Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" '
            f'Target="{target}" TargetMode="External"/>'
        )
    else:
        insert += (
            f'<Relationship Id="{rid}" '
            'Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" '
            f'Target="{spec["location"]}" TargetMode="External"/>'
        )

rels_body = rels
if "</Relationships>" in rels_body:
    rels_body = rels_body.replace("</Relationships>", insert + "</Relationships>")
elif rels_body.rstrip().endswith("/>"):
    rels_body = rels_body.rstrip()[:-2] + ">" + insert + "</Relationships>"
else:
    raise SystemExit("unexpected drawing rels format")

fd, tmp = tempfile.mkstemp(suffix=".xlsx")
os.close(fd)
with zipfile.ZipFile(tmp, "w", zipfile.ZIP_DEFLATED) as zout:
    for n, data in others.items():
        zout.writestr(n, data)
    zout.writestr(DRAWING, drawing)
    zout.writestr(RELS, rels_body)
os.replace(tmp, PATH)
print(f"patched {PATH}")
