#!/usr/bin/env bash
# Date / time built-in + custom number formats.
# Catches: formatNumber() in packages/xlsx-preview has no date branch — these
# all currently render as the raw serial number.
set -euo pipefail
F=${1:-$(dirname "$0")/date-time-formats.xlsx}
rm -f "$F"
hsx create "$F"

# 45292 = 2024-01-01, 45292.5 = noon, 45292.75 = 18:00.
# 0.5 = 12:00:00 (date-less time).
hsx set "$F" "Sheet1!A1:B1" '[[
  {"value":"format","style":{"fontStyle":{"bold":true}}},
  {"value":"sample","style":{"fontStyle":{"bold":true}}}
]]'

hsx set "$F" "Sheet1!A2:B14" '[
  [{"value":"m/d/yyyy"},                 {"value":45292,    "style":{"formatter":"m/d/yyyy"}}],
  [{"value":"d-mmm-yy"},                 {"value":45292,    "style":{"formatter":"d-mmm-yy"}}],
  [{"value":"d-mmm"},                    {"value":45292,    "style":{"formatter":"d-mmm"}}],
  [{"value":"mmm-yy"},                   {"value":45292,    "style":{"formatter":"mmm-yy"}}],
  [{"value":"h:mm AM/PM"},               {"value":45292.5,  "style":{"formatter":"h:mm AM/PM"}}],
  [{"value":"h:mm:ss AM/PM"},            {"value":45292.75, "style":{"formatter":"h:mm:ss AM/PM"}}],
  [{"value":"h:mm"},                     {"value":0.5,      "style":{"formatter":"h:mm"}}],
  [{"value":"h:mm:ss"},                  {"value":0.75,     "style":{"formatter":"h:mm:ss"}}],
  [{"value":"m/d/yyyy h:mm"},            {"value":45292.5,  "style":{"formatter":"m/d/yyyy h:mm"}}],
  [{"value":"dddd, mmmm d, yyyy"},       {"value":45292,    "style":{"formatter":"dddd, mmmm d, yyyy"}}],
  [{"value":"yyyy-mm-dd"},               {"value":45292,    "style":{"formatter":"yyyy-mm-dd"}}],
  [{"value":"[h]:mm:ss"},                {"value":2.5,      "style":{"formatter":"[h]:mm:ss"}}],
  [{"value":"mm:ss.0"},                  {"value":0.000694, "style":{"formatter":"mm:ss.0"}}]
]'

hsx eval "$F" 'workbook.getSheet(0).setColumnWidth(0, 220); workbook.getSheet(0).setColumnWidth(1, 200);'

echo "Built $F"
