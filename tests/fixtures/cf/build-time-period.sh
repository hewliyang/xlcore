#!/usr/bin/env bash
# Builds a workbook exercising every OOXML `timePeriod` CF rule the
# extractor recognizes: today, yesterday, tomorrow, last7Days, thisWeek,
# lastWeek, nextWeek, thisMonth, lastMonth, nextMonth.
#
# Layout: one row per period. Column A names the period; columns B..H
# hold dates offset around "now" (today − 30 / − 7 / − 1 / 0 / + 1 / + 7
# / + 30 days). Each row carries one `addDateOccurringRule` painting
# matches red on white bold. Because time-period CF is evaluated against
# the *current* date, this fixture is regenerated against today every
# time the script runs; commit the resulting .xlsx + .hsx.png together.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
F="${1:-$HERE/time-period.xlsx}"
rm -f "$F"
hsx create "$F" >/dev/null

hsx eval "$F" - <<'JS'
const Sheets = GC.Spread.Sheets;
const Range = Sheets.Range;
const T = Sheets.ConditionalFormatting.DateOccurringType;

// 7 date offsets around "today", in days.
const offsets = [-30, -7, -1, 0, 1, 7, 30];
const today = new Date(); today.setHours(0,0,0,0);
const dateFor = (d) => {
  const x = new Date(today); x.setDate(today.getDate() + d); return x;
};

// Header row.
const headers = ["timePeriod", ...offsets.map(d => d === 0 ? "today" : (d > 0 ? `+${d}d` : `${d}d`))];
for (let c = 0; c < headers.length; c++) {
  sheet.getCell(0, c).value(headers[c]);
  sheet.getCell(0, c).font("bold 11pt Calibri");
}

const cases = [
  { label: "today",      type: T.today      },
  { label: "yesterday",  type: T.yesterday  },
  { label: "tomorrow",   type: T.tomorrow   },
  { label: "last7Days",  type: T.last7Days  },
  { label: "thisWeek",   type: T.thisWeek   },
  { label: "lastWeek",   type: T.lastWeek   },
  { label: "nextWeek",   type: T.nextWeek   },
  { label: "thisMonth",  type: T.thisMonth  },
  { label: "lastMonth",  type: T.lastMonth  },
  { label: "nextMonth",  type: T.nextMonth  },
];

const dxfRed = { backColor: "#ff4444", foreColor: "#ffffff", fontWeight: "bold" };

for (let i = 0; i < cases.length; i++) {
  const r = i + 1;
  sheet.getCell(r, 0).value(cases[i].label);
  for (let c = 0; c < offsets.length; c++) {
    const cell = sheet.getCell(r, c + 1);
    cell.value(dateFor(offsets[c]));
    cell.formatter("yyyy-mm-dd");
  }
  const rng = new Range(r, 1, 1, offsets.length);
  sheet.conditionalFormats.addDateOccurringRule(cases[i].type, dxfRed, [rng]);
}

sheet.setColumnWidth(0, 110);
for (let c = 1; c <= offsets.length; c++) sheet.setColumnWidth(c, 90);
JS

# hsx daemon caches writes; flush before any subsequent read of the .xlsx.
hsx daemon flush >/dev/null 2>&1 || true
echo "wrote $F"
