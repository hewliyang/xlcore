#!/usr/bin/env bash
# Multi-section formats: positive;negative;zero;text and [color][cond] gates.
# Catches: formatNumber() always uses section[0]; sign + condition logic missing.
set -euo pipefail
F=${1:-$(dirname "$0")/custom-section-conditions.xlsx}
rm -f "$F"
hsx create "$F"

hsx set "$F" "Sheet1!A1:B1" '[[
  {"value":"format","style":{"fontStyle":{"bold":true}}},
  {"value":"sample","style":{"fontStyle":{"bold":true}}}
]]'

# Same format, three signs, so the renderer must pick the right section.
F1='[Red][>0]+#,##0;[Blue][<0]-#,##0;[Green]"zero"'
F2='#,##0.00;(#,##0.00);"-"'
F3='[Red][>100]"high";[Blue][<10]"low";"mid"'
F4='0.0,"K";0.0,,"M";0'   # scaling thousands/millions via trailing comma

payload=$(jq -nc --arg f1 "$F1" --arg f2 "$F2" --arg f3 "$F3" --arg f4 "$F4" '[
  [{value:$f1},                       {value:42,     style:{formatter:$f1}}],
  [{value:($f1+" (neg)")},            {value:-42,    style:{formatter:$f1}}],
  [{value:($f1+" (zero)")},           {value:0,      style:{formatter:$f1}}],
  [{value:$f2},                       {value:1234.5, style:{formatter:$f2}}],
  [{value:($f2+" (neg)")},            {value:-1234.5,style:{formatter:$f2}}],
  [{value:($f2+" (zero)")},           {value:0,      style:{formatter:$f2}}],
  [{value:($f3+" @ 200")},            {value:200,    style:{formatter:$f3}}],
  [{value:($f3+" @ 5")},              {value:5,      style:{formatter:$f3}}],
  [{value:($f3+" @ 50")},             {value:50,     style:{formatter:$f3}}],
  [{value:($f4+" @ 1500")},           {value:1500,   style:{formatter:$f4}}],
  [{value:($f4+" @ 2500000")},        {value:2500000,style:{formatter:$f4}}],
  [{value:($f4+" @ 5")},              {value:5,      style:{formatter:$f4}}]
]')
hsx set "$F" "Sheet1!A2:B13" "$payload"

hsx eval "$F" 'workbook.getSheet(0).setColumnWidth(0, 380); workbook.getSheet(0).setColumnWidth(1, 200);'

echo "Built $F"
