#!/usr/bin/env bash
# Fractions and scientific notation. PARITY.md marks both as ❌ today.
set -euo pipefail
F=${1:-$(dirname "$0")/fraction-and-scientific.xlsx}
rm -f "$F"
hsx create "$F"

hsx set "$F" "Sheet1!A1:B1" '[[
  {"value":"format","style":{"fontStyle":{"bold":true}}},
  {"value":"sample","style":{"fontStyle":{"bold":true}}}
]]'

hsx set "$F" "Sheet1!A2:B11" '[
  [{"value":"# ?/?"},        {"value":0.625,        "style":{"formatter":"# ?/?"}}],
  [{"value":"# ??/??"},      {"value":3.14159,      "style":{"formatter":"# ??/??"}}],
  [{"value":"# ???/???"},    {"value":0.142857,     "style":{"formatter":"# ???/???"}}],
  [{"value":"# ?/8"},        {"value":2.375,        "style":{"formatter":"# ?/8"}}],
  [{"value":"# ?/16"},       {"value":1.0625,       "style":{"formatter":"# ?/16"}}],
  [{"value":"0E+00"},        {"value":12345678,     "style":{"formatter":"0E+00"}}],
  [{"value":"0.00E+00"},     {"value":12345678,     "style":{"formatter":"0.00E+00"}}],
  [{"value":"##0.0E+0"},     {"value":12345678,     "style":{"formatter":"##0.0E+0"}}],
  [{"value":"0.000E+00"},    {"value":0.00001234,   "style":{"formatter":"0.000E+00"}}],
  [{"value":"0.00E+00 (neg)"},{"value":-987654.321, "style":{"formatter":"0.00E+00"}}]
]'

hsx eval "$F" 'workbook.getSheet(0).setColumnWidth(0, 200); workbook.getSheet(0).setColumnWidth(1, 200);'

echo "Built $F"
