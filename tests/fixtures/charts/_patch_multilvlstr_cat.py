#!/usr/bin/env python3
import re
import shutil
import sys
import zipfile
from pathlib import Path

CHART_PATH = "xl/charts/chart1.xml"

NEW_CAT = """<c:cat>
            <c:multiLvlStrRef>
              <c:f>Sheet1!$B$1:$M$2</c:f>
              <c:multiLvlStrCache>
                <c:ptCount val="1"/>
                <c:lvl><c:pt idx="0"><c:v>Jan</c:v></c:pt></c:lvl>
                <c:lvl><c:pt idx="0"><c:v>Feb</c:v></c:pt></c:lvl>
                <c:lvl><c:pt idx="0"><c:v>Mar</c:v></c:pt></c:lvl>
                <c:lvl><c:pt idx="0"><c:v>Apr</c:v></c:pt></c:lvl>
                <c:lvl><c:pt idx="0"><c:v>May</c:v></c:pt></c:lvl>
                <c:lvl><c:pt idx="0"><c:v>Jun</c:v></c:pt></c:lvl>
                <c:lvl><c:pt idx="0"><c:v>Jul</c:v></c:pt></c:lvl>
                <c:lvl><c:pt idx="0"><c:v>Aug</c:v></c:pt></c:lvl>
                <c:lvl><c:pt idx="0"><c:v>Sep</c:v></c:pt></c:lvl>
                <c:lvl><c:pt idx="0"><c:v>Oct</c:v></c:pt></c:lvl>
                <c:lvl><c:pt idx="0"><c:v>Nov</c:v></c:pt></c:lvl>
                <c:lvl><c:pt idx="0"><c:v>Dec</c:v></c:pt></c:lvl>
              </c:multiLvlStrCache>
            </c:multiLvlStrRef>
          </c:cat>"""


def patch(xlsx_path: Path) -> None:
    tmp = xlsx_path.with_suffix(".xlsx.tmp")
    with zipfile.ZipFile(xlsx_path, "r") as zin, zipfile.ZipFile(
        tmp, "w", zipfile.ZIP_DEFLATED
    ) as zout:
        for item in zin.infolist():
            data = zin.read(item.filename)
            if item.filename == CHART_PATH:
                text = data.decode("utf-8")
                new, n = re.subn(
                    r"<c:cat>\s*<c:strRef>.*?</c:strRef>\s*</c:cat>",
                    NEW_CAT,
                    text,
                    count=1,
                    flags=re.DOTALL,
                )
                if n != 1:
                    raise SystemExit(
                        f"could not locate <c:cat><c:strRef> in {CHART_PATH}"
                    )
                data = new.encode("utf-8")
            zout.writestr(item, data)
    shutil.move(tmp, xlsx_path)


if __name__ == "__main__":
    patch(Path(sys.argv[1]))
