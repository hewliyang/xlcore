#!/usr/bin/env bash
set -euo pipefail

DIR=$(cd "$(dirname "$0")" && pwd)
F=${1:-"$DIR/shared-formulas.xlsx"}
rm -f "$F"
mkdir -p "$(dirname "$F")"

python3 - "$F" <<'PY'
import sys
from openpyxl import Workbook

path = sys.argv[1]
wb = Workbook()
ws = wb.active
ws.title = "Sheet1"

for r, (a, b) in enumerate([(10, 1), (20, 2), (30, 3)], start=1):
    ws.cell(r, 1, a)
    ws.cell(r, 2, b)
    ws.cell(r, 3, f"=A{r}+B{r}")
    ws.cell(r, 4, f"=SUM(A{r}:B{r})")
    ws.cell(r, 5, f"=$A{r}+B$1")

wb.save(path)
PY

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
unzip -q "$F" -d "$TMP"

python3 - "$TMP/xl/worksheets/sheet1.xml" <<'PY'
import sys
import xml.etree.ElementTree as ET

path = sys.argv[1]
ns = "http://schemas.openxmlformats.org/spreadsheetml/2006/main"
q = f"{{{ns}}}"
ET.register_namespace("", ns)

tree = ET.parse(path)
root = tree.getroot()
cells = {c.attrib["r"]: c for c in root.findall(f".//{q}c") if "r" in c.attrib}

def set_shared(ref, si, formula=None, shared_ref=None, cached="-999"):
    c = cells[ref]
    for child in list(c):
        if child.tag in {f"{q}f", f"{q}v"}:
            c.remove(child)
    f = ET.Element(f"{q}f", {"t": "shared", "si": str(si)})
    if shared_ref:
        f.set("ref", shared_ref)
    if formula is not None:
        f.text = formula
    v = ET.Element(f"{q}v")
    v.text = cached
    c.append(f)
    c.append(v)

for row in range(1, 4):
    set_shared(f"C{row}", 0, "A1+B1" if row == 1 else None, "C1:C3" if row == 1 else None)
    set_shared(f"D{row}", 1, "SUM(A1:B1)" if row == 1 else None, "D1:D3" if row == 1 else None)
    set_shared(f"E{row}", 2, "$A1+B$1" if row == 1 else None, "E1:E3" if row == 1 else None)

tree.write(path, encoding="UTF-8", xml_declaration=True)
PY

rm -f "$F"
(cd "$TMP" && zip -q -r "$F" .)

echo "Built $F"
