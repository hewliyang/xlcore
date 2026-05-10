#!/usr/bin/env bash
# Fixture: row + column outline levels (Excel "Group" feature).
#
# Layout (rows 1-indexed):
#   Row 1: header
#   Row 2: Region A: Q1
#   Row 3: Region A: Q2     <-- outlineLevel 1
#   Row 4: Region A: Q3     <-- outlineLevel 1
#   Row 5: Region A total   <-- outlineLevel 0 (summary, summaryBelow=true)
#   Row 6: Region B: Q1
#   Row 7: Region B: Q2     <-- outlineLevel 1
#   Row 8: Region B: Q3     <-- outlineLevel 1
#   Row 9: Region B total   <-- outlineLevel 0
#
# Columns A..F:
#   A: label
#   B: Q1   <-- outlineLevel 1
#   C: Q2   <-- outlineLevel 1
#   D: Q3   <-- outlineLevel 1
#   E: Total
#   F: Notes
#
# SpreadJS's xlsx writer drops outlineLevel on rows/cols, so we build
# a plain workbook with hsx and then post-patch the OOXML via Python's
# zipfile + a tiny XML rewrite. Same approach as borders/diagonal and
# fills/patterns fixtures.

set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
F="${1:-$HERE/outline-groups.xlsx}"
rm -f "$F"

hsx create "$F"
hsx set "$F" "Sheet1!A1:F9" '[
  [{"value":"Region/Quarter"},{"value":"Q1"},{"value":"Q2"},{"value":"Q3"},{"value":"Total"},{"value":"Notes"}],
  [{"value":"Region A: Q1"},{"value":100},{"value":110},{"value":120},{"value":330},{"value":""}],
  [{"value":"  detail row 1"},{"value":40},{"value":45},{"value":50},{"value":135},{"value":"breakdown"}],
  [{"value":"  detail row 2"},{"value":60},{"value":65},{"value":70},{"value":195},{"value":"breakdown"}],
  [{"value":"Region A total"},{"value":200},{"value":220},{"value":240},{"value":660},{"value":""}],
  [{"value":"Region B: Q1"},{"value":80},{"value":85},{"value":90},{"value":255},{"value":""}],
  [{"value":"  detail row 1"},{"value":30},{"value":32},{"value":35},{"value":97},{"value":""}],
  [{"value":"  detail row 2"},{"value":50},{"value":53},{"value":55},{"value":158},{"value":""}],
  [{"value":"Region B total"},{"value":160},{"value":170},{"value":180},{"value":510},{"value":""}]
]'

hsx eval "$F" '
  const s = workbook.getSheet(0);
  s.setColumnWidth(0, 130);
  for (let c = 1; c <= 5; c++) s.setColumnWidth(c, 70);
  for (let r = 0; r < 9; r++) s.setRowHeight(r, 22);
  // Bold the header + total rows.
  s.getCell(0, 0).fontStyle({bold:true});
  s.getRange(0, 0, 1, 6).fontStyle({bold:true});
  s.getRange(4, 0, 1, 6).fontStyle({bold:true});
  s.getRange(8, 0, 1, 6).fontStyle({bold:true});
'

# Make sure all daemon-buffered writes have hit disk before we crack open
# the .xlsx zip. Without this, the post-patch sees an empty <sheetData/>.
hsx daemon flush >/dev/null 2>&1 || true

# Post-patch the worksheet XML to add outlineLevel attributes that
# SpreadJS does not expose through its public API.
python3 - "$F" <<'PY'
import sys, zipfile, re, shutil, os

src = sys.argv[1]
tmp = src + ".tmp"

# Per-row outlineLevel. row index is 1-based here (matches XML r="N").
ROW_OL = {3: 1, 4: 1, 7: 1, 8: 1}
# Per-col outlineLevel. min/max are 1-based; B/C/D = 2..4.
COL_OL = {2: 1, 3: 1, 4: 1}

with zipfile.ZipFile(src) as zin:
    with zipfile.ZipFile(tmp, "w", zipfile.ZIP_DEFLATED) as zout:
        for name in zin.namelist():
            data = zin.read(name)
            if name == "xl/worksheets/sheet1.xml":
                xml = data.decode("utf-8")

                # 1) inject outlineLevel="N" on each <row r="N" ...>.
                def patch_row(m):
                    attrs = m.group(0)
                    # Pull the row index.
                    rm = re.search(r'\br="(\d+)"', attrs)
                    if not rm: return attrs
                    r = int(rm.group(1))
                    if r not in ROW_OL: return attrs
                    if "outlineLevel=" in attrs: return attrs
                    return attrs[:-1] + f' outlineLevel="{ROW_OL[r]}">'
                xml = re.sub(r'<row\b[^>]*>', patch_row, xml)

                # 2) ensure a <cols> block with outlineLevel attrs.
                # SpreadJS coalesces equal-width columns into a single
                # <col min="X" max="Y">; we may need to split that
                # block so only B/C/D get outlineLevel="1" and E/F
                # stay at 0.
                def patch_col(m):
                    attrs = m.group(0)
                    minm = re.search(r'\bmin="(\d+)"', attrs)
                    maxm = re.search(r'\bmax="(\d+)"', attrs)
                    if not minm or not maxm: return attrs
                    cmin = int(minm.group(1)); cmax = int(maxm.group(1))
                    # Group consecutive cols that share the same level.
                    runs = []  # list of (lo, hi, level)
                    cur_lo = cmin
                    cur_lvl = COL_OL.get(cmin, 0)
                    for c in range(cmin + 1, cmax + 1):
                        lvl = COL_OL.get(c, 0)
                        if lvl != cur_lvl:
                            runs.append((cur_lo, c - 1, cur_lvl))
                            cur_lo = c
                            cur_lvl = lvl
                    runs.append((cur_lo, cmax, cur_lvl))
                    if all(lvl == 0 for _, _, lvl in runs):
                        return attrs
                    # Split the original <col min=A max=B ...> into one
                    # <col> per run, preserving every other attribute.
                    # Strip the surrounding `<col ... />` wrapper and the
                    # min/max attrs; what remains is the other attribute
                    # text (width, style, customWidth, etc.) we want to
                    # carry through to every split.
                    inner = attrs[1:-2] if attrs.endswith("/>") else attrs[1:-1]
                    inner = re.sub(r'^col\s*', '', inner)
                    inner = re.sub(r'\bmin="\d+"\s*', '', inner, count=1)
                    inner = re.sub(r'\bmax="\d+"\s*', '', inner, count=1)
                    inner = inner.strip()
                    out = []
                    for lo, hi, lvl in runs:
                        attr_str = f'col min="{lo}" max="{hi}"'
                        if inner:
                            attr_str += " " + inner
                        if lvl > 0:
                            attr_str += f' outlineLevel="{lvl}"'
                        out.append(f'<{attr_str}/>')
                    return "".join(out)
                xml = re.sub(r'<col\b[^/>]*/?>', patch_col, xml)

                # 3) sheetPr/outlinePr — leave defaults (summaryBelow=1,
                # summaryRight=1) so summary rows sit at row 5 / row 9.

                data = xml.encode("utf-8")
            zout.writestr(name, data)

os.replace(tmp, src)
print(f"patched {src}")
PY

echo "Built $F"
ls -la "$F"
