import type { Color, Dxf, Sheet, WorkbookLayout } from "./types.js";
import { activeThemeColor } from "./color.js";
import { findCell } from "./geometry.js";
import { HEADER_H, HEADER_W, colLabel } from "./grid.js";
import type { Grid } from "./grid.js";
import { buildMergeMaps, cellRect, mergedRect } from "./geometry.js";
import { frozenDims } from "./panes.js";
import {
  GUTTER_LINE,
  HEADER_BG,
  HEADER_BORDER,
  HEADER_FG,
  HEADER_HIGHLIGHT,
} from "./renderConstants.js";
import type { Pane, Viewport, Visible } from "./renderTypes.js";

// ---------- tables (`<table>` ListObjects) ----------
//
// Render strategy: tables get translated into per-cell dxf overlays
// (header band fill + bold white text, banded data-row tint) which are
// folded into the same `cfDxfs` map that conditional formatting uses,
// so the existing fill / text passes pick them up for free. The header
// row's filter-arrow glyphs paint in their own pass after text. We
// don't try to model Excel's full built-in table-style catalog — just
// enough chrome to make a table look like a table:
//
//  - header bg = the workbook's accent color picked by the trailing
//    integer in the style name (`TableStyleMedium2` → accent1, etc.)
//  - header fg = white, bold
//  - banded data rows (when `showRowStripes`) get a 12% tint of the
//    accent over white
//  - filter arrows paint when `<autoFilter>` was set on the table

function tableAccentHex(styleName: string | undefined): string {
  // Default Excel new-table style is `TableStyleMedium2` (accent1).
  let n = 2;
  if (styleName) {
    const m = styleName.match(/(\d+)$/);
    if (m) n = parseInt(m[1]!, 10);
  }
  // (n - 1) % 6 → 0..5 mapping `Medium2..Medium7` to accent1..accent6.
  // `Medium1` (and Light1 / Dark1) maps to accent1 in real Excel
  // (it's the "first style" in each row, all using accent1 with
  // varying intensities). `(n - 2 + 6) % 6` gives that.
  const idx = (((n - 2) % 6) + 6) % 6;
  return activeThemeColor(4 + idx, "#4472c4");
}

function mixHex(hex: string, other: string, t: number): string {
  // t = 0 → hex, 1 → other.
  const h = hex.startsWith("#") ? hex.slice(1) : hex;
  const o = other.startsWith("#") ? other.slice(1) : other;
  const r1 = parseInt(h.slice(0, 2), 16),
    g1 = parseInt(h.slice(2, 4), 16),
    b1 = parseInt(h.slice(4, 6), 16);
  const r2 = parseInt(o.slice(0, 2), 16),
    g2 = parseInt(o.slice(2, 4), 16),
    b2 = parseInt(o.slice(4, 6), 16);
  const r = Math.round(r1 + (r2 - r1) * t);
  const g = Math.round(g1 + (g2 - g1) * t);
  const b = Math.round(b1 + (b2 - b1) * t);
  const toHex = (v: number) => v.toString(16).padStart(2, "0");
  return "#" + toHex(r) + toHex(g) + toHex(b);
}

export function computeTableState(
  sheet: Sheet,
  vis?: Visible,
): {
  tableDxfs: Map<string, Dxf>;
  filterArrows: Set<string>;
} {
  const tableDxfs = new Map<string, Dxf>();
  const filterArrows = new Set<string>();
  const tables = sheet.tables ?? [];
  const pivots = sheet.pivots ?? [];
  if (tables.length === 0 && pivots.length === 0) {
    return { tableDxfs, filterArrows };
  }

  for (const t of tables) {
    const accent = tableAccentHex(t.style?.name);
    // Light tint = 12% accent over white. Roughly matches
    // `TableStyleMedium*` band rows in Excel.
    const bandHex = mixHex("#ffffff", accent, 0.12);
    const accentColor: Color = { rgb: accent.slice(1).toUpperCase() };
    const bandColor: Color = { rgb: bandHex.slice(1).toUpperCase() };
    const whiteColor: Color = { rgb: "FFFFFF" };

    const headerRows = t.headerRowCount;
    const totalsRows = t.totalsRowCount;
    const r1 = t.range.r1,
      r2 = t.range.r2;
    const c1 = t.range.c1,
      c2 = t.range.c2;
    const headerR = headerRows > 0 ? r1 : -1;
    const dataStart = r1 + headerRows;
    const dataEnd = r2 - totalsRows;

    // Header row: accent fill + bold white text; filter arrows on
    // each header cell when autoFilter is on.
    if (headerR >= 0) {
      const hc1 = Math.max(c1, vis?.firstCol ?? c1);
      const hc2 = Math.min(c2, vis?.lastCol ?? c2);
      for (let c = hc1; c <= hc2; c++) {
        const k = `${headerR}:${c}`;
        if (!vis || (headerR >= vis.firstRow && headerR <= vis.lastRow)) {
          tableDxfs.set(k, {
            fillColor: accentColor,
            fontColor: whiteColor,
            bold: true,
          });
        }
        if (t.hasAutoFilter) filterArrows.add(k);
      }
    }

    // Banded data rows: every other data row (1-indexed from data
    // start) gets the band tint. Skip when stripes are off.
    if (t.style?.showRowStripes !== false) {
      const rr1 = Math.max(dataStart, vis?.firstRow ?? dataStart);
      const rr2 = Math.min(dataEnd, vis?.lastRow ?? dataEnd);
      const cc1 = Math.max(c1, vis?.firstCol ?? c1);
      const cc2 = Math.min(c2, vis?.lastCol ?? c2);
      for (let r = rr1; r <= rr2; r++) {
        const isOdd = ((r - dataStart) & 1) === 1;
        if (!isOdd) continue;
        for (let c = cc1; c <= cc2; c++) {
          const k = `${r}:${c}`;
          if (tableDxfs.has(k)) continue;
          tableDxfs.set(k, { fillColor: bandColor });
        }
      }
    }

    // Totals row: bold + a 1-pixel-feeling top border via a darker
    // band. We don't have border-style overrides in dxf, so just
    // bold the text + give it the band tint.
    if (totalsRows > 0) {
      const totalsR = r2;
      if (vis && (totalsR < vis.firstRow || totalsR > vis.lastRow)) continue;
      const tc1 = Math.max(c1, vis?.firstCol ?? c1);
      const tc2 = Math.min(c2, vis?.lastCol ?? c2);
      for (let c = tc1; c <= tc2; c++) {
        const k = `${totalsR}:${c}`;
        if (tableDxfs.has(k)) continue;
        tableDxfs.set(k, { fillColor: bandColor, bold: true });
      }
    }
  }

  // Pivot-table filter chevrons. We treat pivots as cosmetic chrome
  // only — the cells themselves are already styled by Excel. The
  // extractor pre-computes `filterArrowCells` from `<location>` +
  // `<rowFields>` / `<colFields>` so we just register them here in
  // the same set the table chrome uses.
  for (const p of pivots) {
    for (const cell of p.filterArrowCells) {
      filterArrows.add(`${cell.r}:${cell.c}`);
    }
  }
  return { tableDxfs, filterArrows };
}

/// Paint a small downward-arrow glyph in a rounded box at the right
/// edge of each header cell with `<autoFilter>`. No interactivity.
export function drawFilterArrows(
  ctx: CanvasRenderingContext2D,
  sheet: Sheet,
  g: Grid,
  vis: Visible,
  filterArrows: Set<string>,
): void {
  if (filterArrows.size === 0) return;
  const BOX_W = 14,
    BOX_H = 14,
    INSET_X = 4;
  for (const k of filterArrows) {
    const [rs, cs] = k.split(":");
    const r = parseInt(rs!, 10),
      c = parseInt(cs!, 10);
    if (r < vis.firstRow || r > vis.lastRow) continue;
    if (c < vis.firstCol || c > vis.lastCol) continue;
    const rect = cellRect(g, r, c);
    const x = rect.x + rect.w - BOX_W - INSET_X;
    const y = rect.y + (rect.h - BOX_H) / 2;
    // Box: translucent white over accent header so it reads.
    ctx.fillStyle = "rgba(255, 255, 255, 0.85)";
    ctx.fillRect(x, y, BOX_W, BOX_H);
    ctx.strokeStyle = "rgba(0, 0, 0, 0.25)";
    ctx.lineWidth = 1;
    ctx.strokeRect(x + 0.5, y + 0.5, BOX_W - 1, BOX_H - 1);
    // Down-arrow triangle, centered.
    ctx.fillStyle = "#374151";
    ctx.beginPath();
    const ax = x + BOX_W / 2;
    const ay = y + BOX_H / 2 + 2;
    ctx.moveTo(ax - 4, ay - 2);
    ctx.lineTo(ax + 4, ay - 2);
    ctx.lineTo(ax, ay + 3);
    ctx.closePath();
    ctx.fill();
  }
}

export function drawHeaders(
  ctx: CanvasRenderingContext2D,
  sheet: Sheet,
  g: Grid,
  sel: { r1: number; c1: number; r2: number; c2: number } | null,
  vp: Viewport | null,
  canvasW: number,
  canvasH: number,
  panes: Pane[],
): void {
  const sx = vp ? vp.x : 0;
  const sy = vp ? vp.y : 0;
  const { splitX, splitY, pcw, prh } = frozenDims(sheet, g);

  // Column-label visible ranges. Pinned cols [1..splitX-1] always visible;
  // scrolling cols pulled from whichever pane covers them (TR if pinned
  // rows exist, otherwise BR).
  const scrollPane = panes.find((p) => p.kind === "br")!;
  const topPinPane = panes.find((p) => p.kind === "tr");
  const leftPinPane = panes.find((p) => p.kind === "bl");
  const colScrollVis = (topPinPane ?? scrollPane).vis;
  const rowScrollVis = (leftPinPane ?? scrollPane).vis;

  // Header strips run from the gutter band edge to the canvas edge;
  // the gutter strips (when present) get their own background.
  const headerLeft = g.rowGutterW; // left edge of row-number column
  const headerTop = g.colGutterH; // top edge of column-letter row
  const originX = g.originX; // = HEADER_W + rowGutterW; right edge of row headers
  const originY = g.originY; // = HEADER_H + colGutterH; bottom edge of col headers

  ctx.save();
  ctx.fillStyle = HEADER_BG;
  // Column-header band (everything in the top originY px) + row-header
  // band (everything in the left originX px). Painting both as one
  // L-shape would overlap the corner; doing it in two rects is cheaper
  // than masking.
  ctx.fillRect(0, 0, canvasW, originY);
  ctx.fillRect(0, 0, originX, canvasH);

  // Faint inter-tab rules. Pinned segments don't translate; scrolling
  // segments pan with the BR viewport.
  ctx.strokeStyle = HEADER_BORDER;
  ctx.lineWidth = 1;

  // --- column-header rules ---
  ctx.save();
  ctx.beginPath();
  ctx.rect(originX, headerTop, canvasW - originX, HEADER_H);
  ctx.clip();
  ctx.beginPath();
  // Pinned col rules.
  for (let c = 2; c < splitX; c++) {
    const x = Math.round(g.colX[c] ?? 0) + 0.5;
    ctx.moveTo(x, headerTop);
    ctx.lineTo(x, originY);
  }
  // Scrolling col rules.
  const firstScrollCol = Math.max(splitX, colScrollVis.firstCol);
  for (let c = Math.max(2, firstScrollCol); c <= colScrollVis.lastCol + 1; c++) {
    const x = Math.round((g.colX[c] ?? 0) - sx) + 0.5;
    if (x < originX + pcw) continue;
    ctx.moveTo(x, headerTop);
    ctx.lineTo(x, originY);
  }
  ctx.stroke();
  ctx.restore();

  // --- row-header rules ---
  ctx.save();
  ctx.beginPath();
  ctx.rect(headerLeft, originY, HEADER_W, canvasH - originY);
  ctx.clip();
  ctx.beginPath();
  for (let r = 2; r < splitY; r++) {
    const y = Math.round(g.rowY[r] ?? 0) + 0.5;
    ctx.moveTo(headerLeft, y);
    ctx.lineTo(originX, y);
  }
  const firstScrollRow = Math.max(splitY, rowScrollVis.firstRow);
  for (let r = Math.max(2, firstScrollRow); r <= rowScrollVis.lastRow + 1; r++) {
    const y = Math.round((g.rowY[r] ?? 0) - sy) + 0.5;
    if (y < originY + prh) continue;
    ctx.moveTo(headerLeft, y);
    ctx.lineTo(originX, y);
  }
  ctx.stroke();
  ctx.restore();

  // --- selection tint ---
  if (sel) {
    ctx.fillStyle = HEADER_HIGHLIGHT;
    // Column-header tint: split into pinned segment (cols < splitX) and
    // scrolling segment (cols >= splitX) so the tint stays glued to the
    // correct cells regardless of scroll. Tint covers only the label
    // band [headerTop..originY], not the gutter strip above it.
    const cAbsX1 = g.colX[sel.c1] ?? 0;
    const cAbsX2 = g.colX[sel.c2 + 1] ?? cAbsX1;
    if (cAbsX2 > cAbsX1) {
      if (sel.c1 < splitX) {
        const x1 = cAbsX1;
        const x2 = Math.min(cAbsX2, g.colX[splitX] ?? cAbsX2);
        const cx1 = Math.max(originX, x1);
        const cx2 = Math.min(originX + pcw, x2);
        if (cx2 > cx1) ctx.fillRect(cx1, headerTop, cx2 - cx1, HEADER_H);
      }
      if (sel.c2 >= splitX) {
        const x1 = Math.max(cAbsX1, g.colX[splitX] ?? cAbsX1) - sx;
        const x2 = cAbsX2 - sx;
        const cx1 = Math.max(originX + pcw, x1);
        const cx2 = Math.min(canvasW, x2);
        if (cx2 > cx1) ctx.fillRect(cx1, headerTop, cx2 - cx1, HEADER_H);
      }
    }

    const rAbsY1 = g.rowY[sel.r1] ?? 0;
    const rAbsY2 = g.rowY[sel.r2 + 1] ?? rAbsY1;
    if (rAbsY2 > rAbsY1) {
      if (sel.r1 < splitY) {
        const y1 = rAbsY1;
        const y2 = Math.min(rAbsY2, g.rowY[splitY] ?? rAbsY2);
        const cy1 = Math.max(originY, y1);
        const cy2 = Math.min(originY + prh, y2);
        if (cy2 > cy1) ctx.fillRect(headerLeft, cy1, HEADER_W, cy2 - cy1);
      }
      if (sel.r2 >= splitY) {
        const y1 = Math.max(rAbsY1, g.rowY[splitY] ?? rAbsY1) - sy;
        const y2 = rAbsY2 - sy;
        const cy1 = Math.max(originY + prh, y1);
        const cy2 = Math.min(canvasH, y2);
        if (cy2 > cy1) ctx.fillRect(headerLeft, cy1, HEADER_W, cy2 - cy1);
      }
    }
  }

  // Gutter line. Draws the bottom edge of the column-header strip and
  // the right edge of the row-header strip, both in the darker GUTTER_LINE.
  ctx.strokeStyle = GUTTER_LINE;
  ctx.lineWidth = 2;
  ctx.beginPath();
  ctx.moveTo(0, originY);
  ctx.lineTo(canvasW, originY);
  ctx.moveTo(originX, 0);
  ctx.lineTo(originX, canvasH);
  ctx.stroke();
  // Faint inner separators between gutter strip and header label strip
  // when a gutter is present.
  if (g.rowGutterW > 0 || g.colGutterH > 0) {
    ctx.strokeStyle = HEADER_BORDER;
    ctx.lineWidth = 1;
    ctx.beginPath();
    if (g.rowGutterW > 0) {
      const x = headerLeft + 0.5;
      ctx.moveTo(x, originY);
      ctx.lineTo(x, canvasH);
    }
    if (g.colGutterH > 0) {
      const y = headerTop + 0.5;
      ctx.moveTo(originX, y);
      ctx.lineTo(canvasW, y);
    }
    ctx.stroke();
  }

  ctx.fillStyle = HEADER_FG;
  ctx.font = '11px -apple-system, BlinkMacSystemFont, "Segoe UI", Arial, sans-serif';
  ctx.textBaseline = "middle";
  ctx.textAlign = "center";

  // --- column labels (pinned + scrolling) ---
  const colLabelMidY = headerTop + HEADER_H / 2;
  if (splitX > 1) {
    ctx.save();
    ctx.beginPath();
    ctx.rect(originX, headerTop, pcw, HEADER_H);
    ctx.clip();
    for (let c = 1; c < splitX; c++) {
      const w = g.colW[c] ?? 0;
      if (w <= 0) continue;
      const x = (g.colX[c] ?? 0) + w / 2;
      ctx.fillText(colLabel(c), x, colLabelMidY);
    }
    ctx.restore();
  }
  ctx.save();
  ctx.beginPath();
  ctx.rect(originX + pcw, headerTop, canvasW - originX - pcw, HEADER_H);
  ctx.clip();
  for (let c = Math.max(splitX, colScrollVis.firstCol); c <= colScrollVis.lastCol; c++) {
    const w = g.colW[c] ?? 0;
    if (w <= 0) continue;
    const x = (g.colX[c] ?? 0) + w / 2 - sx;
    ctx.fillText(colLabel(c), x, colLabelMidY);
  }
  ctx.restore();

  // --- row labels ---
  const rowLabelMidX = headerLeft + HEADER_W / 2;
  if (splitY > 1) {
    ctx.save();
    ctx.beginPath();
    ctx.rect(headerLeft, originY, HEADER_W, prh);
    ctx.clip();
    for (let r = 1; r < splitY; r++) {
      const h = g.rowH[r] ?? 0;
      if (h <= 0) continue;
      const y = (g.rowY[r] ?? 0) + h / 2;
      ctx.fillText(String(r), rowLabelMidX, y);
    }
    ctx.restore();
  }
  ctx.save();
  ctx.beginPath();
  ctx.rect(headerLeft, originY + prh, HEADER_W, canvasH - originY - prh);
  ctx.clip();
  for (let r = Math.max(splitY, rowScrollVis.firstRow); r <= rowScrollVis.lastRow; r++) {
    const h = g.rowH[r] ?? 0;
    if (h <= 0) continue;
    const y = (g.rowY[r] ?? 0) + h / 2 - sy;
    ctx.fillText(String(r), rowLabelMidX, y);
  }
  ctx.restore();

  // --- collapsed-group boundary ticks ---
  // When a contiguous run of rows (or columns) is hidden, Excel paints a
  // short green bar on the header of the *next visible* row/column to
  // signal "click here to expand the hidden range". We approximate that
  // with a 2px stroke on the leading edge (top edge for rows, left edge
  // for columns) of the first visible row/col after any hidden run.
  drawCollapsedRowTicks(ctx, g, sy, splitY, prh, canvasH, rowScrollVis);
  drawCollapsedColTicks(ctx, g, sx, splitX, pcw, canvasW, colScrollVis);

  // --- outline gutter strips ---
  // Excel paints group brackets in dedicated strips outside the row/col
  // header bands: a horizontal strip above the col letters for column
  // groupings, and a vertical strip left of the row numbers for row
  // groupings. The shared top-left corner shows level-numeral buttons
  // (1, 2, 3, ...) so you can collapse to a given depth.
  if (g.rowGutterW > 0 || g.colGutterH > 0) {
    drawOutlineCornerButtons(ctx, g);
  }
  if (g.rowGutterW > 0) {
    drawRowOutlineGutter(ctx, sheet, g, sy, splitY, prh, canvasH);
  }
  if (g.colGutterH > 0) {
    drawColOutlineGutter(ctx, sheet, g, sx, splitX, pcw, canvasW);
  }
  // Buttons paint last so they sit on top of any bracket strokes that
  // would otherwise occlude them. Single pass over both axes; collapsed
  // runs (zero bracket extent) still get their + glyph here.
  if (g.rowGutterW > 0 || g.colGutterH > 0) {
    drawOutlineButtons(ctx, sheet, g, {
      sx,
      sy,
      splitX,
      splitY,
      pcw,
      prh,
      canvasW,
      canvasH,
    });
  }

  ctx.textAlign = "start";
  ctx.textBaseline = "alphabetic";
  ctx.restore();
}

import {
  drawCollapsedColTicks,
  drawCollapsedRowTicks,
  drawColOutlineGutter,
  drawOutlineButtons,
  drawOutlineCornerButtons,
  drawRowOutlineGutter,
} from "./outlineGutter.js";

/// Hyperlinks: cells covered by any `<hyperlink>` range get a `Dxf`
/// overlay — `theme[10]` (hlink color, default Office blue `#0563C1`)
/// + `underline: true`. Same plumbing as table chrome, just emitted
/// from the sheet's `hyperlinks` array. We don't try to override an
/// already-present CF or table dxf — caller checks that.
///
/// **Yields to explicit cell formatting.** When the cell's resolved
/// xf points to a non-default fontId, that author chose a font
/// deliberately and Excel/hsx honors it (e.g. `e-007_input-3.xlsx`
/// has a stale `mailto:` rel pointing at a cell whose displayed text
/// was later edited to a plain phone number formatted in Arial 9
/// black — hsx renders that plain, not as a blue+underlined link).
/// We mirror the same rule: skip emitting the overlay when the
/// cell carries its own non-default fontId.
export function computeHyperlinkDxfs(
  sheet: Sheet,
  layout: WorkbookLayout,
): Map<string, Dxf> {
  const out = new Map<string, Dxf>();
  const hyperlinks = sheet.hyperlinks ?? [];
  if (hyperlinks.length === 0) return out;
  // Color { theme: 10 } resolves through `setActiveTheme` to whatever
  // the workbook's `<a:hlink>` slot points at; falls back to Office's
  // 0563C1 default when the theme is missing or the slot is unset.
  const hlinkColor: Color = { theme: 10 };
  for (const h of hyperlinks) {
    const { r1, c1, r2, c2 } = h.range;
    for (let r = r1; r <= r2; r++) {
      for (let c = c1; c <= c2; c++) {
        const k = `${r}:${c}`;
        if (out.has(k)) continue;
        // Check the cell's explicit xf.fontId. The default font is
        // index 0 in `styles.fonts`; anything else means the author
        // chose a specific font and the hyperlink overlay should
        // yield. We only check `cell.styleIndex` (not the row/col
        // fallback) because OOXML's hyperlink-style overlay applies
        // when the cell hasn't been re-styled away from the default.
        const cell = findCell(sheet, r, c);
        if (cell && cell.styleIndex !== undefined) {
          const xf = layout.styles.cellXfs[cell.styleIndex];
          if (xf && xf.fontId !== undefined && xf.fontId !== 0) continue;
        }
        out.set(k, { fontColor: hlinkColor, underline: true });
      }
    }
  }
  return out;
}

/// Comment markers: small red right-triangle clipped to the top-right
/// corner of each commented cell. Matches Excel's classic "this cell
/// has a comment" affordance. The marker draws over text since it sits
/// just inside the cell's border.
export function drawCommentMarkers(
  ctx: CanvasRenderingContext2D,
  sheet: Sheet,
  g: Grid,
  vis: Visible,
): void {
  const comments = sheet.comments ?? [];
  if (comments.length === 0) return;
  const { topLeftOf } = buildMergeMaps(sheet);
  // Triangle leg length in CSS pixels. Excel's marker is ~6px on a
  // 100% zoom row of the default 15pt height; we keep a constant
  // size so it stays legible at small zooms.
  const SIZE = 6;
  ctx.save();
  ctx.fillStyle = "#C81E1E";
  for (const cmt of comments) {
    if (cmt.r < vis.firstRow || cmt.r > vis.lastRow) continue;
    if (cmt.c < vis.firstCol || cmt.c > vis.lastCol) continue;
    const k = `${cmt.r}:${cmt.c}`;
    const rect = topLeftOf.has(k) ? mergedRect(g, topLeftOf.get(k)!) : cellRect(g, cmt.r, cmt.c);
    const x2 = rect.x + rect.w;
    const y1 = rect.y;
    ctx.beginPath();
    ctx.moveTo(x2 - SIZE, y1);
    ctx.lineTo(x2, y1);
    ctx.lineTo(x2, y1 + SIZE);
    ctx.closePath();
    ctx.fill();
  }
  ctx.restore();
}
