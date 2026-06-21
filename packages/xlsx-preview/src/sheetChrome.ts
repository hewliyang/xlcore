import type { Color, CustomTableStyle, Dxf, Sheet, WorkbookLayout } from "./types.js";
import type { PivotFilterArrow } from "./schema/PivotFilterArrow.js";
import type { TableFilterArrow } from "./schema/TableFilterArrow.js";
import { activeThemeColor } from "./color.js";
import type { CellRect } from "./geometry.js";
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

function tableAccentHex(styleName: string | undefined): string {
  let n = 2;
  if (styleName) {
    const m = styleName.match(/(\d+)$/);
    if (m) n = parseInt(m[1]!, 10);
  }

  const idx = (((n - 2) % 6) + 6) % 6;
  return activeThemeColor(4 + idx, "#4472c4");
}

function findCustomTableStyle(
  layout: WorkbookLayout | undefined,
  name: string | undefined,
): CustomTableStyle | undefined {
  if (!name || !layout?.tableStyles?.length) return undefined;
  return layout.tableStyles.find((s) => s.name === name);
}

function resolveDxf(layout: WorkbookLayout | undefined, id: number | undefined): Dxf | undefined {
  if (id === undefined) return undefined;
  const dxf = layout?.dxfs?.[id];
  return dxf;
}

function mergeDxf(base: Dxf | undefined, over: Dxf | undefined): Dxf | undefined {
  if (!base) return over;
  if (!over) return base;
  return {
    fontColor: over.fontColor ?? base.fontColor,
    bold: over.bold ?? base.bold,
    italic: over.italic ?? base.italic,
    strike: over.strike ?? base.strike,
    underline: over.underline ?? base.underline,
    underlineStyle: over.underlineStyle ?? base.underlineStyle,
    fillColor: over.fillColor ?? base.fillColor,
    numFmt: over.numFmt ?? base.numFmt,
    vertAlign: over.vertAlign ?? base.vertAlign,
  };
}

function mixHex(hex: string, other: string, t: number): string {
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
  layout: WorkbookLayout | undefined,
  vis?: Visible,
): {
  tableDxfs: Map<string, Dxf>;
  filterArrows: Set<string>;
} {
  const tableDxfs = new Map<string, Dxf>();
  const filterArrows = new Set<string>();
  const tables = sheet.tables ?? [];
  const pivots = sheet.pivots ?? [];
  const autoFilterRange = sheet.autoFilterRange;
  if (tables.length === 0 && pivots.length === 0 && !autoFilterRange) {
    return { tableDxfs, filterArrows };
  }

  if (autoFilterRange) {
    const r = autoFilterRange.r1;
    if (!vis || (r >= vis.firstRow && r <= vis.lastRow)) {
      const c1 = Math.max(autoFilterRange.c1, vis?.firstCol ?? autoFilterRange.c1);
      const c2 = Math.min(autoFilterRange.c2, vis?.lastCol ?? autoFilterRange.c2);
      for (let c = c1; c <= c2; c++) filterArrows.add(`${r}:${c}`);
    }
  }

  for (const t of tables) {
    const custom = findCustomTableStyle(layout, t.style?.name);
    const wholeTableDxf = resolveDxf(layout, custom?.wholeTable);

    const accent = tableAccentHex(t.style?.name);
    const bandHex = mixHex("#ffffff", accent, 0.12);
    const accentColor: Color = { rgb: accent.slice(1).toUpperCase() };
    const bandColor: Color = { rgb: bandHex.slice(1).toUpperCase() };
    const whiteColor: Color = { rgb: "FFFFFF" };
    const headerDxf: Dxf = mergeDxf(wholeTableDxf, resolveDxf(layout, custom?.headerRow)) ?? {
      fillColor: accentColor,
      fontColor: whiteColor,
      bold: true,
    };
    const stripeDxf: Dxf = mergeDxf(wholeTableDxf, resolveDxf(layout, custom?.firstRowStripe)) ?? {
      fillColor: bandColor,
    };
    const totalDxf: Dxf | undefined = mergeDxf(wholeTableDxf, resolveDxf(layout, custom?.totalRow));

    const headerRows = t.headerRowCount;
    const totalsRows = t.totalsRowCount;
    const r1 = t.range.r1,
      r2 = t.range.r2;
    const c1 = t.range.c1,
      c2 = t.range.c2;
    const headerR = headerRows > 0 ? r1 : -1;
    const dataStart = r1 + headerRows;
    const dataEnd = r2 - totalsRows;

    if (headerR >= 0) {
      const hc1 = Math.max(c1, vis?.firstCol ?? c1);
      const hc2 = Math.min(c2, vis?.lastCol ?? c2);
      for (let c = hc1; c <= hc2; c++) {
        const k = `${headerR}:${c}`;
        if (!vis || (headerR >= vis.firstRow && headerR <= vis.lastRow)) {
          tableDxfs.set(k, headerDxf);
        }
        if (t.hasAutoFilter) filterArrows.add(k);
      }
    }

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
          tableDxfs.set(k, stripeDxf);
        }
      }
    }

    if (totalsRows > 0) {
      const totalsR = r2;
      if (vis && (totalsR < vis.firstRow || totalsR > vis.lastRow)) continue;
      const tc1 = Math.max(c1, vis?.firstCol ?? c1);
      const tc2 = Math.min(c2, vis?.lastCol ?? c2);
      for (let c = tc1; c <= tc2; c++) {
        const k = `${totalsR}:${c}`;
        if (tableDxfs.has(k)) continue;
        tableDxfs.set(k, totalDxf ?? { fillColor: bandColor, bold: true });
      }
    }
  }

  for (const p of pivots) {
    for (const cell of p.filterArrowCells) {
      filterArrows.add(`${cell.r}:${cell.c}`);
    }
  }
  return { tableDxfs, filterArrows };
}

export const FILTER_ARROW_BOX_W = 14;
export const FILTER_ARROW_BOX_H = 14;
export const FILTER_ARROW_INSET_X = 4;

export function filterArrowRect(rect: CellRect): CellRect {
  return {
    x: rect.x + rect.w - FILTER_ARROW_BOX_W - FILTER_ARROW_INSET_X,
    y: rect.y + (rect.h - FILTER_ARROW_BOX_H) / 2,
    w: FILTER_ARROW_BOX_W,
    h: FILTER_ARROW_BOX_H,
  };
}

export const VALIDATION_ARROW_BOX = 16;

export function validationArrowRect(rect: CellRect): CellRect {
  const h = Math.min(VALIDATION_ARROW_BOX, rect.h);
  return {
    x: rect.x + rect.w,
    y: rect.y + (rect.h - h) / 2,
    w: VALIDATION_ARROW_BOX,
    h,
  };
}

function drawArrowButton(ctx: CanvasRenderingContext2D, box: CellRect): void {
  const { x, y, w, h } = box;
  ctx.fillStyle = "rgba(255, 255, 255, 0.95)";
  ctx.fillRect(x, y, w, h);
  ctx.strokeStyle = "rgba(0, 0, 0, 0.35)";
  ctx.lineWidth = 1;
  ctx.strokeRect(x + 0.5, y + 0.5, w - 1, h - 1);

  ctx.fillStyle = "#374151";
  ctx.beginPath();
  const ax = x + w / 2;
  const ay = y + h / 2 + 1;
  ctx.moveTo(ax - 4, ay - 2);
  ctx.lineTo(ax + 4, ay - 2);
  ctx.lineTo(ax, ay + 3);
  ctx.closePath();
  ctx.fill();
}

export interface PivotArrowHit extends PivotFilterArrow {
  pivot: string;
}

export function pivotFilterArrows(sheet: Sheet): PivotArrowHit[] {
  const out: PivotArrowHit[] = [];
  for (const p of sheet.pivots ?? []) {
    for (const a of p.filterArrowCells) out.push({ ...a, pivot: p.name });
  }
  return out;
}

export function tableFilterArrows(sheet: Sheet): TableFilterArrow[] {
  return sheet.tableFilterArrows ?? [];
}

export function drawFilterArrows(
  ctx: CanvasRenderingContext2D,
  sheet: Sheet,
  g: Grid,
  vis: Visible,
  filterArrows: Set<string>,
): void {
  if (filterArrows.size === 0) return;
  const BOX_W = FILTER_ARROW_BOX_W,
    BOX_H = FILTER_ARROW_BOX_H;
  for (const k of filterArrows) {
    const [rs, cs] = k.split(":");
    const r = parseInt(rs!, 10),
      c = parseInt(cs!, 10);
    if (r < vis.firstRow || r > vis.lastRow) continue;
    if (c < vis.firstCol || c > vis.lastCol) continue;
    const box = filterArrowRect(cellRect(g, r, c));
    const x = box.x;
    const y = box.y;

    ctx.fillStyle = "rgba(255, 255, 255, 0.85)";
    ctx.fillRect(x, y, BOX_W, BOX_H);
    ctx.strokeStyle = "rgba(0, 0, 0, 0.25)";
    ctx.lineWidth = 1;
    ctx.strokeRect(x + 0.5, y + 0.5, BOX_W - 1, BOX_H - 1);

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

export function drawValidationArrows(
  ctx: CanvasRenderingContext2D,
  sheet: Sheet,
  g: Grid,
  vis: Visible,
  active: { r: number; c: number } | null,
): void {
  const dropdowns = sheet.validationDropdowns ?? [];
  if (dropdowns.length === 0) return;
  const list = active ? dropdowns.filter((d) => d.r === active.r && d.c === active.c) : dropdowns;
  const { topLeftOf } = buildMergeMaps(sheet);
  for (const d of list) {
    if (d.r < vis.firstRow || d.r > vis.lastRow) continue;
    if (d.c < vis.firstCol || d.c > vis.lastCol) continue;
    const m = topLeftOf.get(`${d.r}:${d.c}`);
    const rect = m ? mergedRect(g, m) : cellRect(g, d.r, d.c);
    drawArrowButton(ctx, validationArrowRect(rect));
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

  const scrollPane = panes.find((p) => p.kind === "br")!;
  const topPinPane = panes.find((p) => p.kind === "tr");
  const leftPinPane = panes.find((p) => p.kind === "bl");
  const colScrollVis = (topPinPane ?? scrollPane).vis;
  const rowScrollVis = (leftPinPane ?? scrollPane).vis;

  const headerLeft = g.rowGutterW;
  const headerTop = g.colGutterH;
  const originX = g.originX;
  const originY = g.originY;

  ctx.save();
  ctx.fillStyle = HEADER_BG;

  ctx.fillRect(0, 0, canvasW, originY);
  ctx.fillRect(0, 0, originX, canvasH);

  ctx.strokeStyle = HEADER_BORDER;
  ctx.lineWidth = 1;

  ctx.save();
  ctx.beginPath();
  ctx.rect(originX, headerTop, canvasW - originX, HEADER_H);
  ctx.clip();
  ctx.beginPath();

  for (let c = 2; c < splitX; c++) {
    const x = Math.round(g.colX[c] ?? 0) + 0.5;
    ctx.moveTo(x, headerTop);
    ctx.lineTo(x, originY);
  }

  const firstScrollCol = Math.max(splitX, colScrollVis.firstCol);
  for (let c = Math.max(2, firstScrollCol); c <= colScrollVis.lastCol + 1; c++) {
    const x = Math.round((g.colX[c] ?? 0) - sx) + 0.5;
    if (x < originX + pcw) continue;
    ctx.moveTo(x, headerTop);
    ctx.lineTo(x, originY);
  }
  ctx.stroke();
  ctx.restore();

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

  if (sel) {
    ctx.fillStyle = HEADER_HIGHLIGHT;

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

  ctx.strokeStyle = GUTTER_LINE;
  ctx.lineWidth = 2;
  ctx.beginPath();
  ctx.moveTo(0, originY);
  ctx.lineTo(canvasW, originY);
  ctx.moveTo(originX, 0);
  ctx.lineTo(originX, canvasH);
  ctx.stroke();

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

  drawCollapsedRowTicks(ctx, g, sy, splitY, prh, canvasH, rowScrollVis);
  drawCollapsedColTicks(ctx, g, sx, splitX, pcw, canvasW, colScrollVis);

  if (g.rowGutterW > 0 || g.colGutterH > 0) {
    drawOutlineCornerButtons(ctx, g);
  }
  if (g.rowGutterW > 0) {
    drawRowOutlineGutter(ctx, sheet, g, sy, splitY, prh, canvasH);
  }
  if (g.colGutterH > 0) {
    drawColOutlineGutter(ctx, sheet, g, sx, splitX, pcw, canvasW);
  }

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

export function computeHyperlinkDxfs(sheet: Sheet, layout: WorkbookLayout): Map<string, Dxf> {
  const out = new Map<string, Dxf>();
  const hyperlinks = sheet.hyperlinks ?? [];
  if (hyperlinks.length === 0) return out;

  const hlinkColor: Color = { theme: 10 };
  for (const h of hyperlinks) {
    const { r1, c1, r2, c2 } = h.range;
    for (let r = r1; r <= r2; r++) {
      for (let c = c1; c <= c2; c++) {
        const k = `${r}:${c}`;
        if (out.has(k)) continue;

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

export function drawCommentMarkers(
  ctx: CanvasRenderingContext2D,
  sheet: Sheet,
  g: Grid,
  vis: Visible,
): void {
  const comments = sheet.comments ?? [];
  if (comments.length === 0) return;
  const { topLeftOf } = buildMergeMaps(sheet);

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
