#!/usr/bin/env bash
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
F="${1:-$HERE/style-refs-matrix.xlsx}"
SRC="$HERE/basic-autoshapes.xlsx"
if [ ! -f "$SRC" ]; then
  echo "missing source fixture $SRC — run build-basic-autoshapes.sh first" >&2
  exit 1
fi
rm -f "$F"
cp "$SRC" "$F"

python3 - "$F" <<'PY'
import re, sys, shutil, zipfile, tempfile, os

src = sys.argv[1]
tmp_dir = tempfile.mkdtemp()
out = os.path.join(tmp_dir, "patched.xlsx")

with zipfile.ZipFile(src, "r") as zin, zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as zout:
    for item in zin.infolist():
        data = zin.read(item.filename)
        if item.filename.startswith("xl/drawings/drawing") and item.filename.endswith(".xml"):
            xml = data.decode("utf-8")
            xml = re.sub(r"<a:solidFill>.*?</a:solidFill>", "", xml, flags=re.S)
            xml = re.sub(r"<a:ln(\s[^>]*)?>.*?</a:ln>", "", xml, flags=re.S)
            xml = re.sub(r"<a:ln(\s[^>]*)?/>", "", xml)

            accents = ["accent1", "accent2", "accent3", "accent4", "accent5", "accent6"]
            tuples = [(1, 0), (2, 1), (3, 2)]
            counter = [0]
            def rewrite(m):
                i = counter[0]
                row = i // 5
                acc = accents[i % len(accents)]
                fill_idx, effect_idx = tuples[row % len(tuples)]
                counter[0] = i + 1
                return (
                    f'<xdr:style>'
                    f'<a:lnRef idx="2"><a:schemeClr val="{acc}"/></a:lnRef>'
                    f'<a:fillRef idx="{fill_idx}"><a:schemeClr val="{acc}"/></a:fillRef>'
                    f'<a:effectRef idx="{effect_idx}"><a:schemeClr val="{acc}"/></a:effectRef>'
                    f'<a:fontRef idx="minor"><a:schemeClr val="dk1"/></a:fontRef>'
                    f'</xdr:style>'
                )
            xml = re.sub(r"<xdr:style>.*?</xdr:style>", rewrite, xml, flags=re.S)
            data = xml.encode("utf-8")
        zout.writestr(item, data)

shutil.move(out, src)
shutil.rmtree(tmp_dir)
print(f"patched {src}")
PY

echo "wrote $F"
