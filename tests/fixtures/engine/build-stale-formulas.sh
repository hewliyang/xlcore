#!/usr/bin/env bash
set -euo pipefail

DIR=$(cd "$(dirname "$0")" && pwd)
F=${1:-"$DIR/stale-formulas.xlsx"}
rm -f "$F"
mkdir -p "$(dirname "$F")"

python3 - "$F" <<'PY'
import sys
from openpyxl import Workbook

path = sys.argv[1]
wb = Workbook()
ws = wb.active
ws.title = "Sheet1"

ws["A1"] = 10
ws["B1"] = 2
ws["C1"] = "=SUM(A1:A2)"
ws["A2"] = 20
ws["B2"] = 3
ws["C2"] = "=SUMPRODUCT(A1:A2,B1:B2)"

wb.save(path)
PY

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
unzip -q "$F" -d "$TMP"
perl -0pi -e 's#(<c r="C1"[^>]*><f>SUM\(A1:A2\)</f>)(?:<v>[^<]*</v>)?#$1<v>-1</v>#g; s#(<c r="C2"[^>]*><f>SUMPRODUCT\(A1:A2,B1:B2\)</f>)(?:<v>[^<]*</v>)?#$1<v>-2</v>#g' "$TMP/xl/worksheets/sheet1.xml"
rm -f "$F"
(cd "$TMP" && zip -q -r "$F" .)

echo "Built $F"
