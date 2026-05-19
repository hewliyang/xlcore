import type { Sheet } from "./types.js";
import { HEADER_H, HEADER_W, OUTLINE_GUTTER_PAD, OUTLINE_GUTTER_STEP } from "./grid.js";
import type { Grid } from "./grid.js";

export interface OutlineRun {
  axis: "row" | "col";
  level: number;

  start: number;

  end: number;

  summary: number;
}

export function computeOutlineRuns(sheet: Sheet, g: Grid): OutlineRun[] {
  const runs: OutlineRun[] = [];

  if (g.rowOutlineDepth > 0) {
    const meta = sheet.decodedRowMeta;
    const lvlByRow = new Map<number, number>();
    if (meta && meta.outlineLevel.length > 0) {
      for (let i = 0; i < meta.count; i++) {
        const v = meta.outlineLevel[i] ?? 0;
        if (v > 0) lvlByRow.set(meta.index[i] ?? 0, v);
      }
    }
    const summaryBelow = sheet.outlinePr?.summaryBelow ?? true;
    for (let lvl = 1; lvl <= g.rowOutlineDepth; lvl++) {
      let runStart = -1;
      for (let r = 1; r <= g.maxRow + 1; r++) {
        const inRun = r <= g.maxRow && (lvlByRow.get(r) ?? 0) >= lvl;
        if (inRun && runStart < 0) runStart = r;
        if (!inRun && runStart >= 0) {
          const runEnd = r - 1;
          const summary = summaryBelow ? runEnd + 1 : runStart - 1;
          runs.push({ axis: "row", level: lvl, start: runStart, end: runEnd, summary });
          runStart = -1;
        }
      }
    }
  }

  if (g.colOutlineDepth > 0) {
    const lvlByCol = new Map<number, number>();
    for (const c of sheet.cols) {
      const lvl = c.outlineLevel ?? 0;
      if (lvl === 0) continue;
      for (let i = c.min; i <= c.max; i++) lvlByCol.set(i, lvl);
    }
    const summaryRight = sheet.outlinePr?.summaryRight ?? true;
    for (let lvl = 1; lvl <= g.colOutlineDepth; lvl++) {
      let runStart = -1;
      for (let c = 1; c <= g.maxCol + 1; c++) {
        const inRun = c <= g.maxCol && (lvlByCol.get(c) ?? 0) >= lvl;
        if (inRun && runStart < 0) runStart = c;
        if (!inRun && runStart >= 0) {
          const runEnd = c - 1;
          const summary = summaryRight ? runEnd + 1 : runStart - 1;
          runs.push({ axis: "col", level: lvl, start: runStart, end: runEnd, summary });
          runStart = -1;
        }
      }
    }
  }

  return runs;
}

export function isOutlineRunCollapsed(run: OutlineRun, g: Grid): boolean {
  if (run.axis === "row") {
    for (let r = run.start; r <= run.end; r++) {
      if ((g.rowH[r] ?? 0) > 0) return false;
    }
  } else {
    for (let c = run.start; c <= run.end; c++) {
      if ((g.colW[c] ?? 0) > 0) return false;
    }
  }
  return true;
}

export interface OutlineGutterView {
  sx: number;
  sy: number;

  splitX: number;
  splitY: number;

  pcw: number;
  prh: number;

  canvasW: number;
  canvasH: number;
}

export interface OutlineButtonHit {
  run: OutlineRun;

  cx: number;
  cy: number;
  collapsed: boolean;
}

export function outlineButtonHits(
  sheet: Sheet,
  g: Grid,
  view: OutlineGutterView,
): OutlineButtonHit[] {
  const out: OutlineButtonHit[] = [];
  if (g.rowGutterW === 0 && g.colGutterH === 0) return out;
  const runs = computeOutlineRuns(sheet, g);
  for (const run of runs) {
    if (run.axis === "row") {
      const sumY = g.rowY[run.summary] ?? -1;
      const sumH = g.rowH[run.summary] ?? 0;
      if (sumH <= 0) continue;
      const isPinned = run.summary < view.splitY;
      const cy = isPinned ? sumY + sumH / 2 : sumY + sumH / 2 - view.sy;
      if (isPinned) {
        if (cy < g.originY || cy > g.originY + view.prh) continue;
      } else {
        if (cy < g.originY + view.prh || cy > view.canvasH) continue;
      }
      const cx = rowGutterTrackX(g, run.level);
      out.push({ run, cx, cy, collapsed: isOutlineRunCollapsed(run, g) });
    } else {
      const sumX = g.colX[run.summary] ?? -1;
      const sumW = g.colW[run.summary] ?? 0;
      if (sumW <= 0) continue;
      const isPinned = run.summary < view.splitX;
      const cx = isPinned ? sumX + sumW / 2 : sumX + sumW / 2 - view.sx;
      if (isPinned) {
        if (cx < g.originX || cx > g.originX + view.pcw) continue;
      } else {
        if (cx < g.originX + view.pcw || cx > view.canvasW) continue;
      }
      const cy = colGutterTrackY(g, run.level);
      out.push({ run, cx, cy, collapsed: isOutlineRunCollapsed(run, g) });
    }
  }
  return out;
}

export interface OutlineCornerHit {
  axis: "row" | "col";

  level: number;
  cx: number;
  cy: number;
}

export function outlineCornerHits(g: Grid): OutlineCornerHit[] {
  const out: OutlineCornerHit[] = [];
  if (g.colOutlineDepth > 0) {
    const cx = g.rowGutterW > 0 ? (g.rowGutterW + g.originX) / 2 : g.originX - HEADER_W / 2;
    for (let lvl = 1; lvl <= g.colOutlineDepth + 1; lvl++) {
      const cy = OUTLINE_GUTTER_PAD + (lvl - 1) * OUTLINE_GUTTER_STEP + OUTLINE_GUTTER_STEP / 2;
      out.push({ axis: "col", level: lvl, cx, cy });
    }
  }
  if (g.rowOutlineDepth > 0) {
    const cy = g.colGutterH > 0 ? (g.colGutterH + g.originY) / 2 : g.originY - HEADER_H / 2;
    for (let lvl = 1; lvl <= g.rowOutlineDepth + 1; lvl++) {
      const cx = OUTLINE_GUTTER_PAD + (lvl - 1) * OUTLINE_GUTTER_STEP + OUTLINE_GUTTER_STEP / 2;
      out.push({ axis: "row", level: lvl, cx, cy });
    }
  }
  return out;
}

export const OUTLINE_BUTTON_HIT_RADIUS = 7;

export function drawOutlineButtons(
  ctx: CanvasRenderingContext2D,
  sheet: Sheet,
  g: Grid,
  view: OutlineGutterView,
): void {
  const hits = outlineButtonHits(sheet, g, view);
  if (hits.length === 0) return;
  ctx.save();
  for (const h of hits) {
    drawOutlineButton(
      ctx,
      h.run.axis === "row" ? h.cx - 0.5 : h.cx,
      h.run.axis === "row" ? h.cy : h.cy - 0.5,
      h.collapsed ? "+" : "-",
    );
  }
  ctx.restore();
}

const OUTLINE_STROKE = "#9aa0a6";
const OUTLINE_BUTTON_SIZE = 10;
const OUTLINE_BUTTON_BG = "#ffffff";
const OUTLINE_BUTTON_BORDER = "#6b7280";
const OUTLINE_BUTTON_GLYPH = "#374151";

const COLLAPSED_TICK_STROKE = "#137333";
const COLLAPSED_TICK_WIDTH = 2;

export function drawCollapsedRowTicks(
  ctx: CanvasRenderingContext2D,
  g: Grid,
  sy: number,
  splitY: number,
  prh: number,
  canvasH: number,
  rowScrollVis: { firstRow: number; lastRow: number },
): void {
  const xLeft = g.rowGutterW;
  const xRight = g.originX;
  ctx.save();
  ctx.strokeStyle = COLLAPSED_TICK_STROKE;
  ctx.lineWidth = COLLAPSED_TICK_WIDTH;

  const paintTick = (yTop: number, clipY1: number, clipY2: number) => {
    if (yTop < clipY1 || yTop > clipY2) return;
    const y = yTop + COLLAPSED_TICK_WIDTH / 2;
    ctx.beginPath();
    ctx.moveTo(xLeft, y);
    ctx.lineTo(xRight, y);
    ctx.stroke();
  };

  if (splitY > 1) {
    ctx.save();
    ctx.beginPath();
    ctx.rect(xLeft, g.originY, HEADER_W, prh);
    ctx.clip();
    for (let r = 2; r < splitY; r++) {
      if ((g.rowH[r] ?? 0) <= 0) continue;
      if ((g.rowH[r - 1] ?? 0) > 0) continue;
      paintTick(g.rowY[r] ?? 0, g.originY, g.originY + prh);
    }
    ctx.restore();
  }

  ctx.save();
  ctx.beginPath();
  ctx.rect(xLeft, g.originY + prh, HEADER_W, canvasH - g.originY - prh);
  ctx.clip();
  const first = Math.max(splitY, rowScrollVis.firstRow);
  for (let r = Math.max(2, first); r <= rowScrollVis.lastRow; r++) {
    if ((g.rowH[r] ?? 0) <= 0) continue;
    if ((g.rowH[r - 1] ?? 0) > 0) continue;
    paintTick((g.rowY[r] ?? 0) - sy, g.originY + prh, canvasH);
  }
  ctx.restore();

  ctx.restore();
}

export function drawCollapsedColTicks(
  ctx: CanvasRenderingContext2D,
  g: Grid,
  sx: number,
  splitX: number,
  pcw: number,
  canvasW: number,
  colScrollVis: { firstCol: number; lastCol: number },
): void {
  const yTop = g.colGutterH;
  const yBot = g.originY;
  ctx.save();
  ctx.strokeStyle = COLLAPSED_TICK_STROKE;
  ctx.lineWidth = COLLAPSED_TICK_WIDTH;

  const paintTick = (xLeft: number, clipX1: number, clipX2: number) => {
    if (xLeft < clipX1 || xLeft > clipX2) return;
    const x = xLeft + COLLAPSED_TICK_WIDTH / 2;
    ctx.beginPath();
    ctx.moveTo(x, yTop);
    ctx.lineTo(x, yBot);
    ctx.stroke();
  };

  if (splitX > 1) {
    ctx.save();
    ctx.beginPath();
    ctx.rect(g.originX, yTop, pcw, HEADER_H);
    ctx.clip();
    for (let c = 2; c < splitX; c++) {
      if ((g.colW[c] ?? 0) <= 0) continue;
      if ((g.colW[c - 1] ?? 0) > 0) continue;
      paintTick(g.colX[c] ?? 0, g.originX, g.originX + pcw);
    }
    ctx.restore();
  }

  ctx.save();
  ctx.beginPath();
  ctx.rect(g.originX + pcw, yTop, canvasW - g.originX - pcw, HEADER_H);
  ctx.clip();
  const first = Math.max(splitX, colScrollVis.firstCol);
  for (let c = Math.max(2, first); c <= colScrollVis.lastCol; c++) {
    if ((g.colW[c] ?? 0) <= 0) continue;
    if ((g.colW[c - 1] ?? 0) > 0) continue;
    paintTick((g.colX[c] ?? 0) - sx, g.originX + pcw, canvasW);
  }
  ctx.restore();

  ctx.restore();
}

function rowGutterTrackX(g: Grid, lvl: number): number {
  return OUTLINE_GUTTER_PAD + (lvl - 1) * OUTLINE_GUTTER_STEP + OUTLINE_GUTTER_STEP / 2;
}

function colGutterTrackY(g: Grid, lvl: number): number {
  return OUTLINE_GUTTER_PAD + (lvl - 1) * OUTLINE_GUTTER_STEP + OUTLINE_GUTTER_STEP / 2;
}

function drawOutlineButton(
  ctx: CanvasRenderingContext2D,
  cx: number,
  cy: number,
  glyph: "+" | "-",
): void {
  const s = OUTLINE_BUTTON_SIZE;
  const x = Math.round(cx - s / 2) + 0.5;
  const y = Math.round(cy - s / 2) + 0.5;
  ctx.fillStyle = OUTLINE_BUTTON_BG;
  ctx.fillRect(x, y, s - 1, s - 1);
  ctx.strokeStyle = OUTLINE_BUTTON_BORDER;
  ctx.lineWidth = 1;
  ctx.strokeRect(x, y, s - 1, s - 1);
  ctx.strokeStyle = OUTLINE_BUTTON_GLYPH;
  ctx.beginPath();

  const mx1 = x + 2;
  const mx2 = x + s - 3;
  const my = y + (s - 1) / 2;
  ctx.moveTo(mx1, my);
  ctx.lineTo(mx2, my);
  if (glyph === "+") {
    const mvy1 = y + 2;
    const mvy2 = y + s - 3;
    const mvx = x + (s - 1) / 2;
    ctx.moveTo(mvx, mvy1);
    ctx.lineTo(mvx, mvy2);
  }
  ctx.stroke();
}

export function drawOutlineCornerButtons(ctx: CanvasRenderingContext2D, g: Grid): void {
  ctx.save();
  ctx.font = '10px -apple-system, BlinkMacSystemFont, "Segoe UI", Arial, sans-serif';
  ctx.textBaseline = "middle";
  ctx.textAlign = "center";
  const paintNumeral = (cx: number, cy: number, n: number) => {
    const s = OUTLINE_BUTTON_SIZE;
    const x = Math.round(cx - s / 2) + 0.5;
    const y = Math.round(cy - s / 2) + 0.5;
    ctx.fillStyle = OUTLINE_BUTTON_BG;
    ctx.fillRect(x, y, s - 1, s - 1);
    ctx.strokeStyle = OUTLINE_BUTTON_BORDER;
    ctx.lineWidth = 1;
    ctx.strokeRect(x, y, s - 1, s - 1);
    ctx.fillStyle = OUTLINE_BUTTON_GLYPH;
    ctx.fillText(String(n), cx, cy + 0.5);
  };
  for (const h of outlineCornerHits(g)) paintNumeral(h.cx, h.cy, h.level);
  ctx.restore();
}

export function drawRowOutlineGutter(
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
  const lvlByRow = new Map<number, number>();
  for (let i = 0; i < meta.count; i++) {
    const v = meta.outlineLevel[i] ?? 0;
    if (v > 0) lvlByRow.set(meta.index[i] ?? 0, v);
  }
  const summaryBelow = sheet.outlinePr?.summaryBelow ?? true;

  ctx.save();
  ctx.strokeStyle = OUTLINE_STROKE;
  ctx.lineWidth = 1;

  for (let lvl = 1; lvl <= g.rowOutlineDepth; lvl++) {
    const x = rowGutterTrackX(g, lvl) + 0.5;
    paintRowRunsForLevel(
      ctx,
      lvlByRow,
      lvl,
      x,
      g,
      summaryBelow,
      1,
      Math.max(0, splitY - 1),
      0,
      g.originY,
      g.originY + prh,
    );
    paintRowRunsForLevel(
      ctx,
      lvlByRow,
      lvl,
      x,
      g,
      summaryBelow,
      Math.max(1, splitY),
      g.maxRow,
      -sy,
      g.originY + prh,
      canvasH,
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
  summaryBelow: boolean,
  rowFrom: number,
  rowTo: number,
  offsetY: number,
  clipY1: number,
  clipY2: number,
): void {
  if (rowTo < rowFrom) return;
  ctx.save();
  ctx.beginPath();
  ctx.rect(0, clipY1, g.rowGutterW, clipY2 - clipY1);
  ctx.clip();

  let runStart = -1;
  for (let r = rowFrom; r <= rowTo + 1; r++) {
    const inRun = r <= rowTo && (lvlByRow.get(r) ?? 0) >= lvl;
    if (inRun && runStart < 0) runStart = r;
    if (!inRun && runStart >= 0) {
      const runEnd = r - 1;
      const y1 = (g.rowY[runStart] ?? g.originY) + offsetY;
      const y2 = (g.rowY[runEnd + 1] ?? g.originY) + offsetY;

      if (y2 - y1 < 3) {
        runStart = -1;
        continue;
      }
      if (y2 > clipY1 && y1 < clipY2) {
        const hookY = summaryBelow ? y1 : y2;
        ctx.beginPath();
        ctx.moveTo(xLine, y1);
        ctx.lineTo(xLine, y2);
        ctx.moveTo(xLine, hookY);
        ctx.lineTo(xLine + 3, hookY);
        ctx.stroke();
      }
      runStart = -1;
    }
  }
  ctx.restore();
}

export function drawColOutlineGutter(
  ctx: CanvasRenderingContext2D,
  sheet: Sheet,
  g: Grid,
  sx: number,
  splitX: number,
  pcw: number,
  canvasW: number,
): void {
  if (g.colOutlineDepth === 0) return;
  const lvlByCol = new Map<number, number>();
  for (const c of sheet.cols) {
    const lvl = c.outlineLevel ?? 0;
    if (lvl === 0) continue;
    for (let i = c.min; i <= c.max; i++) lvlByCol.set(i, lvl);
  }
  const summaryRight = sheet.outlinePr?.summaryRight ?? true;

  ctx.save();
  ctx.strokeStyle = OUTLINE_STROKE;
  ctx.lineWidth = 1;

  for (let lvl = 1; lvl <= g.colOutlineDepth; lvl++) {
    const y = colGutterTrackY(g, lvl) + 0.5;
    paintColRunsForLevel(
      ctx,
      lvlByCol,
      lvl,
      y,
      g,
      summaryRight,
      1,
      Math.max(0, splitX - 1),
      0,
      g.originX,
      g.originX + pcw,
    );
    paintColRunsForLevel(
      ctx,
      lvlByCol,
      lvl,
      y,
      g,
      summaryRight,
      Math.max(1, splitX),
      g.maxCol,
      -sx,
      g.originX + pcw,
      canvasW,
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
  summaryRight: boolean,
  colFrom: number,
  colTo: number,
  offsetX: number,
  clipX1: number,
  clipX2: number,
): void {
  if (colTo < colFrom) return;
  ctx.save();
  ctx.beginPath();
  ctx.rect(clipX1, 0, clipX2 - clipX1, g.colGutterH);
  ctx.clip();

  let runStart = -1;
  for (let c = colFrom; c <= colTo + 1; c++) {
    const inRun = c <= colTo && (lvlByCol.get(c) ?? 0) >= lvl;
    if (inRun && runStart < 0) runStart = c;
    if (!inRun && runStart >= 0) {
      const runEnd = c - 1;
      const x1 = (g.colX[runStart] ?? g.originX) + offsetX;
      const x2 = (g.colX[runEnd + 1] ?? g.originX) + offsetX;
      if (x2 - x1 < 3) {
        runStart = -1;
        continue;
      }
      if (x2 > clipX1 && x1 < clipX2) {
        const hookX = summaryRight ? x1 : x2;
        ctx.beginPath();
        ctx.moveTo(x1, yLine);
        ctx.lineTo(x2, yLine);
        ctx.moveTo(hookX, yLine);
        ctx.lineTo(hookX, yLine + 3);
        ctx.stroke();
      }
      runStart = -1;
    }
  }
  ctx.restore();
}
