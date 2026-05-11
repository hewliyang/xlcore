#!/usr/bin/env bash
# Builds a fixture that exercises CF rule priority + cross-kind
# stopIfTrue masking. SpreadJS doesn't expose stopIfTrue on its public
# xlsx-emit path, so we patch the OOXML directly via Python zip-edit.
#
# See `_patch_stop_if_true.py` for the full layout + expected behavior.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
F="${1:-$HERE/stop-if-true.xlsx}"
rm -f "$F"
hsx create "$F" >/dev/null
hsx daemon flush >/dev/null 2>&1 || true
python3 "$HERE/_patch_stop_if_true.py" "$F"
echo "wrote $F"
