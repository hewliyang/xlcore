#!/usr/bin/env bash
set -euo pipefail

F=${1:-$(dirname "$0")/basic-formulas.xlsx}
rm -f "$F"
mkdir -p "$(dirname "$F")"

hsx create "$F" >/dev/null
hsx set "$F" "Sheet1!A1:C2" '[
  [{"value":10},{"value":2},{"formula":"=SUM(A1:A2)"}],
  [{"value":20},{"value":3},{"formula":"=SUMPRODUCT(A1:A2,B1:B2)"}]
]'

echo "Built $F"
