#!/usr/bin/env bash
# Fixture: OOXML font-family detection — <scheme val="major|minor"/>
# theme references and <family val="N"/> numeric family hints.
#
# Covers:
#   - Cell font with <scheme val="major"/> and a stale <name val="WRONG"/>
#     cache; renderer must re-resolve to the theme's majorFont (Georgia).
#   - Same for <scheme val="minor"/> (resolves to Verdana).
#   - <family val="1|3|4|5"/> with an uninstalled named typeface; the
#     CSS fallback chain should drop into serif / monospace / cursive /
#     fantasy respectively.
#
# Built via Python zip-patch because hsx's public JS API doesn't expose
# <scheme> or <family> on the font block.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
F="${1:-$HERE/fontfamily.xlsx}"
rm -f "$F"
hsx create "$F" >/dev/null
python3 "$HERE/_patch_fontfamily.py" "$F"
echo "wrote $F"
