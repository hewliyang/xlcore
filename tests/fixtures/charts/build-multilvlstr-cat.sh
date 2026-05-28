#!/usr/bin/env bash
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
F="${1:-$HERE/multilvlstr-cat.xlsx}"
rm -f "$F"
hsx create "$F" >/dev/null
hsx set "$F" 'Sheet1!A1:M2' '[
[{"value":""},{"value":"Jan"},{"value":"Feb"},{"value":"Mar"},{"value":"Apr"},{"value":"May"},{"value":"Jun"},{"value":"Jul"},{"value":"Aug"},{"value":"Sep"},{"value":"Oct"},{"value":"Nov"},{"value":"Dec"}],
[{"value":"Sales"},{"value":10},{"value":20},{"value":15},{"value":25},{"value":30},{"value":22},{"value":18},{"value":28},{"value":35},{"value":40},{"value":33},{"value":45}]
]' >/dev/null
hsx eval "$F" '
const sht = workbook.getSheet(0);
const T = GC.Spread.Sheets.Charts.ChartType;
const c = sht.charts.add("c", T.columnClustered, 0, 80, 600, 300, "A1:M2");
c.title({text:"multiLvlStrRef category axis"});
' >/dev/null
hsx daemon flush >/dev/null
python3 "$HERE/_patch_multilvlstr_cat.py" "$F"
echo "wrote $F"
