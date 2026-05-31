#!/usr/bin/env bash
set -euo pipefail

DIR=$(cd "$(dirname "$0")" && pwd)
F=${1:-"$DIR/unsupported-formulas.xlsx"}
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
ws["B1"] = "=NO_SUCH_FUNCTION(A1)"
ws["C1"] = "=SUM(A1,5)"

wb.save(path)
PY

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
unzip -q "$F" -d "$TMP"
perl -0pi -e 's#(<c r="B1"[^>]*><f>NO_SUCH_FUNCTION\(A1\)</f>)(?:<v>[^<]*</v>)?#$1<v>123</v>#g; s#(<c r="C1"[^>]*><f>SUM\(A1,5\)</f>)(?:<v>[^<]*</v>)?#$1<v>-1</v>#g' "$TMP/xl/worksheets/sheet1.xml"
rm -f "$F"
(cd "$TMP" && zip -q -r "$F" .)

echo "Built $F"
