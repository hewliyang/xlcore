#!/usr/bin/env bash
# Currency / accounting / locale-tagged formats.
# Catches: accounting underscores+asterisks ignored; non-$ currency
# symbols ([$€-407], [$£-809]) lost when the [..] stripper runs.
set -euo pipefail
F=${1:-$(dirname "$0")/currency-locale.xlsx}
rm -f "$F"
hsx create "$F"

hsx set "$F" "Sheet1!A1:B1" '[[
  {"value":"format","style":{"fontStyle":{"bold":true}}},
  {"value":"sample","style":{"fontStyle":{"bold":true}}}
]]'

hsx set "$F" "Sheet1!A2:B11" '[
  [{"value":"$#,##0"},                                    {"value":1234567,  "style":{"formatter":"$#,##0"}}],
  [{"value":"$#,##0.00"},                                 {"value":1234.5,   "style":{"formatter":"$#,##0.00"}}],
  [{"value":"$#,##0.00 (negative)"},                      {"value":-1234.5,  "style":{"formatter":"$#,##0.00;($#,##0.00)"}}],
  [{"value":"[$€-407] #,##0.00"},                         {"value":1234.5,   "style":{"formatter":"[$€-407] #,##0.00"}}],
  [{"value":"[$£-809] #,##0.00"},                         {"value":-1234.5,  "style":{"formatter":"[$£-809] #,##0.00;[Red][$£-809] (#,##0.00)"}}],
  [{"value":"[$¥-411] #,##0"},                            {"value":1234567,  "style":{"formatter":"[$¥-411] #,##0"}}],
  [{"value":"_(\"$\"* #,##0.00_);_(\"$\"* (#,##0.00);..."},{"value":1234.5,  "style":{"formatter":"_(\"$\"* #,##0.00_);_(\"$\"* (#,##0.00);_(\"$\"* \"-\"??_);_(@_)"}}],
  [{"value":"accounting (negative)"},                     {"value":-1234.5,  "style":{"formatter":"_(\"$\"* #,##0.00_);_(\"$\"* (#,##0.00);_(\"$\"* \"-\"??_);_(@_)"}}],
  [{"value":"accounting (zero)"},                         {"value":0,        "style":{"formatter":"_(\"$\"* #,##0.00_);_(\"$\"* (#,##0.00);_(\"$\"* \"-\"??_);_(@_)"}}],
  [{"value":"#,##0_);[Red](#,##0)"},                      {"value":-1234,    "style":{"formatter":"#,##0_);[Red](#,##0)"}}]
]'

hsx eval "$F" 'workbook.getSheet(0).setColumnWidth(0, 320); workbook.getSheet(0).setColumnWidth(1, 200);'

echo "Built $F"
