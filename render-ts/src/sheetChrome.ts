import type { Color, Dxf, Sheet } from "./types.js";
import { activeThemeColor } from "./color.js";
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

export function computeTableState(sheet: Sheet): {
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
      for (let c = c1; c <= c2; c++) {
        const k = `${headerR}:${c}`;
        tableDxfs.set(k, {
          fillColor: accentColor,
          fontColor: whiteColor,
          bold: true,
        });
        if (t.hasAutoFilter) filterArrows.add(k);
      }
    }

    // Banded data rows: every other data row (1-indexed from data
    // start) gets the band tint. Skip when stripes are off.
    if (t.style?.showRowStripes !== false) {
      for (let r = dataStart; r <= dataEnd; r++) {
        const isOdd = ((r - dataStart) & 1) === 1;
        if (!isOdd) continue;
        for (let c = c1; c <= c2; c++) {
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
      for (let c = c1; c <= c2; c++) {
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

  ctx.save();
  ctx.fillStyle = HEADER_BG;
  ctx.fillRect(0, 0, canvasW, HEADER_H);
  ctx.fillRect(0, 0, HEADER_W, canvasH);

  // Faint inter-tab rules. Pinned segments don't translate; scrolling
  // segments pan with the BR viewport.
  ctx.strokeStyle = HEADER_BORDER;
  ctx.lineWidth = 1;

  // --- column-header rules ---
  ctx.save();
  ctx.beginPath();
  ctx.rect(HEADER_W, 0, canvasW - HEADER_W, HEADER_H);
  ctx.clip();
  ctx.beginPath();
  // Pinned col rules.
  for (let c = 2; c < splitX; c++) {
    const x = Math.round(g.colX[c] ?? 0) + 0.5;
    ctx.moveTo(x, 0);
    ctx.lineTo(x, HEADER_H);
  }
  // Scrolling col rules.
  const firstScrollCol = Math.max(splitX, colScrollVis.firstCol);
  for (let c = Math.max(2, firstScrollCol); c <= colScrollVis.lastCol + 1; c++) {
    const x = Math.round((g.colX[c] ?? 0) - sx) + 0.5;
    if (x < HEADER_W + pcw) continue;
    ctx.moveTo(x, 0);
    ctx.lineTo(x, HEADER_H);
  }
  ctx.stroke();
  ctx.restore();

  // --- row-header rules ---
  ctx.save();
  ctx.beginPath();
  ctx.rect(0, HEADER_H, HEADER_W, canvasH - HEADER_H);
  ctx.clip();
  ctx.beginPath();
  for (let r = 2; r < splitY; r++) {
    const y = Math.round(g.rowY[r] ?? 0) + 0.5;
    ctx.moveTo(0, y);
    ctx.lineTo(HEADER_W, y);
  }
  const firstScrollRow = Math.max(splitY, rowScrollVis.firstRow);
  for (let r = Math.max(2, firstScrollRow); r <= rowScrollVis.lastRow + 1; r++) {
    const y = Math.round((g.rowY[r] ?? 0) - sy) + 0.5;
    if (y < HEADER_H + prh) continue;
    ctx.moveTo(0, y);
    ctx.lineTo(HEADER_W, y);
  }
  ctx.stroke();
  ctx.restore();

  // --- selection tint ---
  if (sel) {
    ctx.fillStyle = HEADER_HIGHLIGHT;
    // Column-header tint: split into pinned segment (cols < splitX) and
    // scrolling segment (cols >= splitX) so the tint stays glued to the
    // correct cells regardless of scroll.
    const cAbsX1 = g.colX[sel.c1] ?? 0;
    const cAbsX2 = g.colX[sel.c2 + 1] ?? cAbsX1;
    if (cAbsX2 > cAbsX1) {
      // Pinned slice [c1..min(c2, splitX-1)] -> canvas x = colX[c]
      if (sel.c1 < splitX) {
        const x1 = cAbsX1;
        const x2 = Math.min(cAbsX2, g.colX[splitX] ?? cAbsX2);
        const cx1 = Math.max(HEADER_W, x1);
        const cx2 = Math.min(HEADER_W + pcw, x2);
        if (cx2 > cx1) ctx.fillRect(cx1, 0, cx2 - cx1, HEADER_H);
      }
      // Scrolling slice [max(c1, splitX)..c2] -> canvas x = colX[c] - sx
      if (sel.c2 >= splitX) {
        const x1 = Math.max(cAbsX1, g.colX[splitX] ?? cAbsX1) - sx;
        const x2 = cAbsX2 - sx;
        const cx1 = Math.max(HEADER_W + pcw, x1);
        const cx2 = Math.min(canvasW, x2);
        if (cx2 > cx1) ctx.fillRect(cx1, 0, cx2 - cx1, HEADER_H);
      }
    }

    const rAbsY1 = g.rowY[sel.r1] ?? 0;
    const rAbsY2 = g.rowY[sel.r2 + 1] ?? rAbsY1;
    if (rAbsY2 > rAbsY1) {
      if (sel.r1 < splitY) {
        const y1 = rAbsY1;
        const y2 = Math.min(rAbsY2, g.rowY[splitY] ?? rAbsY2);
        const cy1 = Math.max(HEADER_H, y1);
        const cy2 = Math.min(HEADER_H + prh, y2);
        if (cy2 > cy1) ctx.fillRect(0, cy1, HEADER_W, cy2 - cy1);
      }
      if (sel.r2 >= splitY) {
        const y1 = Math.max(rAbsY1, g.rowY[splitY] ?? rAbsY1) - sy;
        const y2 = rAbsY2 - sy;
        const cy1 = Math.max(HEADER_H + prh, y1);
        const cy2 = Math.min(canvasH, y2);
        if (cy2 > cy1) ctx.fillRect(0, cy1, HEADER_W, cy2 - cy1);
      }
    }
  }

  // Gutter line.
  ctx.strokeStyle = GUTTER_LINE;
  ctx.lineWidth = 2;
  ctx.beginPath();
  ctx.moveTo(0, HEADER_H);
  ctx.lineTo(canvasW, HEADER_H);
  ctx.moveTo(HEADER_W, 0);
  ctx.lineTo(HEADER_W, canvasH);
  ctx.stroke();

  ctx.fillStyle = HEADER_FG;
  ctx.font = '11px -apple-system, BlinkMacSystemFont, "Segoe UI", Arial, sans-serif';
  ctx.textBaseline = "middle";
  ctx.textAlign = "center";

  // --- column labels (pinned + scrolling) ---
  if (splitX > 1) {
    ctx.save();
    ctx.beginPath();
    ctx.rect(HEADER_W, 0, pcw, HEADER_H);
    ctx.clip();
    for (let c = 1; c < splitX; c++) {
      const x = (g.colX[c] ?? 0) + (g.colW[c] ?? 0) / 2;
      ctx.fillText(colLabel(c), x, HEADER_H / 2);
    }
    ctx.restore();
  }
  ctx.save();
  ctx.beginPath();
  ctx.rect(HEADER_W + pcw, 0, canvasW - HEADER_W - pcw, HEADER_H);
  ctx.clip();
  for (let c = Math.max(splitX, colScrollVis.firstCol); c <= colScrollVis.lastCol; c++) {
    const x = (g.colX[c] ?? 0) + (g.colW[c] ?? 0) / 2 - sx;
    ctx.fillText(colLabel(c), x, HEADER_H / 2);
  }
  ctx.restore();

  // --- row labels ---
  if (splitY > 1) {
    ctx.save();
    ctx.beginPath();
    ctx.rect(0, HEADER_H, HEADER_W, prh);
    ctx.clip();
    for (let r = 1; r < splitY; r++) {
      const y = (g.rowY[r] ?? 0) + (g.rowH[r] ?? 0) / 2;
      ctx.fillText(String(r), HEADER_W / 2, y);
    }
    ctx.restore();
  }
  ctx.save();
  ctx.beginPath();
  ctx.rect(0, HEADER_H + prh, HEADER_W, canvasH - HEADER_H - prh);
  ctx.clip();
  for (let r = Math.max(splitY, rowScrollVis.firstRow); r <= rowScrollVis.lastRow; r++) {
    const y = (g.rowY[r] ?? 0) + (g.rowH[r] ?? 0) / 2 - sy;
    ctx.fillText(String(r), HEADER_W / 2, y);
  }
  ctx.restore();

  // --- outline brackets (row + column groupings) ---
  // Painted last (over labels) so the bracket strokes are not clipped
  // by the label clip-rects. Indent per outline level is tight enough
  // (~7px) to fit inside HEADER_W=44 / HEADER_H=22 without restructuring
  // the grid coordinate system. See PARITY.md “Outline / group levels”
  // row for the planned follow-up that adds a separate gutter strip.
  drawRowOutlineBrackets(ctx, sheet, g, sy, splitY, prh, canvasH);
  drawColOutlineBrackets(ctx, sheet, g, sx, splitX, pcw, canvasW);

  ctx.textAlign = "start";
  ctx.textBaseline = "alphabetic";
  ctx.restore();
}

// Pixels of indent per outline level. Excel uses ~12px in its own
// outline gutter strip; we squeeze into HEADER_W=44 next to the row
// label, so we use a smaller step. With max OOXML depth 7 this still
// leaves ~26px for the label text (which usually shows at most 4
// digits anyway).
const OUTLINE_STEP_PX = 7;
// Inset from the start of HEADER so the bracket doesn't graze the
// canvas edge. Brackets stack inward from there.
const OUTLINE_INSET_PX = 4;
const OUTLINE_STROKE = "#9aa0a6"; // mid-gray, matches Excel’s gutter color

/// Paint vertical outline brackets in the row header strip for every
/// contiguous run of rows whose `outlineLevel >= L` (for L = 1..maxLvl).
/// Brackets stack from canvas-x = OUTLINE_INSET_PX outward, one column
/// per level. No expand/collapse buttons, no summary-row glyphs.
function drawRowOutlineBrackets(
  ctx: CanvasRenderingContext2D,
  sheet: Sheet,
  g: Grid,
  sy: number,
  splitY: number,
  prh: number,
  canvasH: number,
): void {
  const meta = sheet.decodedRowMeta;
  if (meta.outlineLevel.length === 0) return;
  let maxLvl = 0;
  for (let i = 0; i < meta.outlineLevel.length; i++) {
    const v = meta.outlineLevel[i] ?? 0;
    if (v > maxLvl) maxLvl = v;
  }
  if (maxLvl === 0) return;

  // index → outlineLevel for fast row lookup.
  const lvlByRow = new Map<number, number>();
  for (let i = 0; i < meta.count; i++) {
    const v = meta.outlineLevel[i] ?? 0;
    if (v > 0) lvlByRow.set(meta.index[i] ?? 0, v);
  }

  ctx.save();
  ctx.strokeStyle = OUTLINE_STROKE;
  ctx.lineWidth = 1;

  // Walk all visible rows in [1..g.maxRow] split into pinned + scrolling
  // segments. We compute runs separately for each level so a level-2
  // group (rows 5..10) sits inside its level-1 parent (rows 4..11).
  for (let lvl = 1; lvl <= maxLvl; lvl++) {
    const xLine = OUTLINE_INSET_PX + (lvl - 1) * OUTLINE_STEP_PX + 0.5;
    paintRowRunsForLevel(
      ctx,
      lvlByRow,
      lvl,
      xLine,
      g,
      /*rowFrom*/ 1,
      /*rowTo*/ Math.max(0, splitY - 1),
      /*offsetY*/ 0,
      /*clipY1*/ HEADER_H,
      /*clipY2*/ HEADER_H + prh,
    );
    paintRowRunsForLevel(
      ctx,
      lvlByRow,
      lvl,
      xLine,
      g,
      /*rowFrom*/ Math.max(1, splitY),
      /*rowTo*/ g.maxRow,
      /*offsetY*/ -sy,
      /*clipY1*/ HEADER_H + prh,
      /*clipY2*/ canvasH,
    );
  }
  ctx.restore();
}

function paintRowRunsForLevel(
  ctx: CanvasRenderingContext2D,
  lvlByRow: Map<number, number>,
  lvl: number,
  xLine: number,
  g: Grid,
  rowFrom: number,
  rowTo: number,
  offsetY: number,
  clipY1: number,
  clipY2: number,
): void {
  if (rowTo < rowFrom) return;
  ctx.save();
  ctx.beginPath();
  ctx.rect(0, clipY1, HEADER_W, clipY2 - clipY1);
  ctx.clip();

  // Find every contiguous run of rows in [rowFrom..rowTo] with level >= lvl.
  let runStart = -1;
  for (let r = rowFrom; r <= rowTo + 1; r++) {
    const inRun = r <= rowTo && (lvlByRow.get(r) ?? 0) >= lvl;
    if (inRun && runStart < 0) runStart = r;
    if (!inRun && runStart >= 0) {
      const runEnd = r - 1;
      const y1 = (g.rowY[runStart] ?? HEADER_H) + offsetY;
      const y2 = (g.rowY[runEnd + 1] ?? HEADER_H) + offsetY;
      if (y2 > clipY1 && y1 < clipY2) {
        // Vertical line + 4px tick caps on top + bottom (Excel-style).
        ctx.beginPath();
        ctx.moveTo(xLine, y1);
        ctx.lineTo(xLine, y2);
        ctx.moveTo(xLine, y1 + 0.5);
        ctx.lineTo(xLine + 4, y1 + 0.5);
        ctx.moveTo(xLine, y2 - 0.5);
        ctx.lineTo(xLine + 4, y2 - 0.5);
        ctx.stroke();
      }
      runStart = -1;
    }
  }
  ctx.restore();
}

/// Same shape as the row-bracket painter but for column groupings.
function drawColOutlineBrackets(
  ctx: CanvasRenderingContext2D,
  sheet: Sheet,
  g: Grid,
  sx: number,
  splitX: number,
  pcw: number,
  canvasW: number,
): void {
  let maxLvl = 0;
  for (const c of sheet.cols) {
    if ((c.outlineLevel ?? 0) > maxLvl) maxLvl = c.outlineLevel ?? 0;
  }
  if (maxLvl === 0) return;

  // Build a column → outlineLevel lookup. The wire stores Col entries
  // as (min..max) ranges; expand them so the per-col paint can do an
  // O(1) test.
  const lvlByCol = new Map<number, number>();
  for (const c of sheet.cols) {
    const lvl = c.outlineLevel ?? 0;
    if (lvl === 0) continue;
    for (let i = c.min; i <= c.max; i++) lvlByCol.set(i, lvl);
  }

  ctx.save();
  ctx.strokeStyle = OUTLINE_STROKE;
  ctx.lineWidth = 1;

  for (let lvl = 1; lvl <= maxLvl; lvl++) {
    const yLine = OUTLINE_INSET_PX + (lvl - 1) * OUTLINE_STEP_PX + 0.5;
    paintColRunsForLevel(
      ctx,
      lvlByCol,
      lvl,
      yLine,
      g,
      /*colFrom*/ 1,
      /*colTo*/ Math.max(0, splitX - 1),
      /*offsetX*/ 0,
      /*clipX1*/ HEADER_W,
      /*clipX2*/ HEADER_W + pcw,
    );
    paintColRunsForLevel(
      ctx,
      lvlByCol,
      lvl,
      yLine,
      g,
      /*colFrom*/ Math.max(1, splitX),
      /*colTo*/ g.maxCol,
      /*offsetX*/ -sx,
      /*clipX1*/ HEADER_W + pcw,
      /*clipX2*/ canvasW,
    );
  }
  ctx.restore();
}

function paintColRunsForLevel(
  ctx: CanvasRenderingContext2D,
  lvlByCol: Map<number, number>,
  lvl: number,
  yLine: number,
  g: Grid,
  colFrom: number,
  colTo: number,
  offsetX: number,
  clipX1: number,
  clipX2: number,
): void {
  if (colTo < colFrom) return;
  ctx.save();
  ctx.beginPath();
  ctx.rect(clipX1, 0, clipX2 - clipX1, HEADER_H);
  ctx.clip();

  let runStart = -1;
  for (let c = colFrom; c <= colTo + 1; c++) {
    const inRun = c <= colTo && (lvlByCol.get(c) ?? 0) >= lvl;
    if (inRun && runStart < 0) runStart = c;
    if (!inRun && runStart >= 0) {
      const runEnd = c - 1;
      const x1 = (g.colX[runStart] ?? HEADER_W) + offsetX;
      const x2 = (g.colX[runEnd + 1] ?? HEADER_W) + offsetX;
      if (x2 > clipX1 && x1 < clipX2) {
        ctx.beginPath();
        ctx.moveTo(x1, yLine);
        ctx.lineTo(x2, yLine);
        ctx.moveTo(x1 + 0.5, yLine);
        ctx.lineTo(x1 + 0.5, yLine + 4);
        ctx.moveTo(x2 - 0.5, yLine);
        ctx.lineTo(x2 - 0.5, yLine + 4);
        ctx.stroke();
      }
      runStart = -1;
    }
  }
  ctx.restore();
}

/// Hyperlinks: cells covered by any `<hyperlink>` range get a `Dxf`
/// overlay — `theme[10]` (hlink color, default Office blue `#0563C1`)
/// + `underline: true`. Same plumbing as table chrome, just emitted
/// from the sheet's `hyperlinks` array. We don't try to override an
/// already-present CF or table dxf — caller checks that.
export function computeHyperlinkDxfs(sheet: Sheet): Map<string, Dxf> {
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
