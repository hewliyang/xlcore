#!/usr/bin/env bash
# Derives pivot-empty-records.xlsx from pivot-simple.xlsx by blanking the
# pivotCacheRecords part (count="0", no <r> rows), simulating a workbook saved
# with the cache stripped (refreshOnLoad). Exercises the engine's
# worksheetSource fallback that rebuilds records from the source range.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
SRC="$HERE/pivot-simple.xlsx"
OUT="$HERE/pivot-empty-records.xlsx"
TMP="$(mktemp -d)"
unzip -q "$SRC" -d "$TMP"
printf '%s' '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<pivotCacheRecords xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" count="0"/>' \
  > "$TMP/xl/pivotCache/pivotCacheRecords1.xml"
rm -f "$OUT"
(cd "$TMP" && zip -qr -X "$OUT" .)
rm -rf "$TMP"
echo "wrote $OUT"
