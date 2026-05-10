#!/usr/bin/env bash
# Builds a workbook exercising hyperlinks + cell comments.
#
# Hyperlinks: 4 cells in column B, three external (https://, mailto:,
# UNC-style file://) and one in-workbook (`#Sheet1!A1`-style location).
# Verifies that:
#   - the renderer paints theme-hlink color + underline on the cell,
#   - the extractor resolves the `r:id` rel target for external links,
#   - the in-workbook `location` attribute survives the round-trip.
#
# Comments: 3 cells in column D with rich-text bodies and distinct
# authors. Verifies the red-triangle marker placement and that author
# resolution works against the part's `<authors>` table.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
F="${1:-$HERE/hyperlinks-comments.xlsx}"
rm -f "$F"
hsx create "$F" >/dev/null

hsx eval "$F" - <<'JS'
const Sheets = GC.Spread.Sheets;

// --- Hyperlinks ---------------------------------------------------------
sheet.getCell(0, 0).value("link kind");
sheet.getCell(0, 1).value("cell");
sheet.getCell(0, 0).font("bold 11pt Calibri");
sheet.getCell(0, 1).font("bold 11pt Calibri");

const links = [
  { row: 1, label: "external https",  text: "OpenAI",     url: "https://openai.com/" },
  { row: 2, label: "external mailto", text: "email",      url: "mailto:hi@example.com" },
  { row: 3, label: "in-workbook",     text: "to D7",      url: null, location: "Sheet1!D7" },
  { row: 4, label: "external file",   text: "share",      url: "file://server/share/doc.xlsx" },
];
for (const l of links) {
  sheet.getCell(l.row, 0).value(l.label);
  sheet.getCell(l.row, 1).value(l.text);
  // setHyperlink takes (row, col, url, [tooltip], [type])
  if (l.url) {
    sheet.setHyperlink(l.row, 1, { url: l.url, tooltipText: l.label });
  } else if (l.location) {
    // location-only hyperlink: SpreadJS treats this as a "location" via
    // the same setHyperlink with a `#`-prefixed url.
    sheet.setHyperlink(l.row, 1, { url: "#" + l.location, tooltipText: l.label });
  }
}

// --- Comments -----------------------------------------------------------
sheet.getCell(0, 3).value("commented");
sheet.getCell(0, 3).font("bold 11pt Calibri");
const comments = [
  { row: 1, val: 42,    author: "Alice", body: "needs review" },
  { row: 2, val: "WIP", author: "Bob",   body: "blocked on data" },
  { row: 3, val: 3.14,  author: "Carol", body: "rounding ok" },
];
for (const ct of comments) {
  sheet.getCell(ct.row, 3).value(ct.val);
  sheet.comments.add(ct.row, 3, ct.body);
  // The most-recently-added comment is the last in `.all()`. Set its
  // author so the extractor's `<authors>` lookup has something to find.
  const allCmts = sheet.comments.all();
  const cmt = allCmts[allCmts.length - 1];
  if (cmt && typeof cmt.userName === "function") cmt.userName(ct.author);
}

// Layout: widen the link column + give the comment column some air.
sheet.setColumnWidth(0, 130);
sheet.setColumnWidth(1, 130);
sheet.setColumnWidth(3, 110);
JS

hsx daemon flush >/dev/null 2>&1 || true
echo "wrote $F"
