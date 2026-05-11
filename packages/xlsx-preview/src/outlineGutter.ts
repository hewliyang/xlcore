import type { Sheet } from "./types.js";
import { HEADER_H, HEADER_W, OUTLINE_GUTTER_PAD, OUTLINE_GUTTER_STEP } from "./grid.js";
import type { Grid } from "./grid.js";

// ---------- outline runs (axis-agnostic) ----------
//
// A `run` is a maximal contiguous range of rows / columns at a given
// outline level. The summary row/col sits one slot before or after the
// run depending on `outlinePr.summaryBelow` / `summaryRight`. We need
// runs in two places:
//   1. The painter — to draw `[`-shaped brackets and the +/- button.
//   2. The interaction layer — to hit-test the +/- button and toggle
//      the run's detail rows/cols.
// `computeOutlineRuns` is the shared source of truth.

export interface OutlineRun {
  axis: "row" | "col";
  level: number;
  /// First detail index (inclusive, 1-based).
  start: number;
  /// Last detail index (inclusive, 1-based).
  end: number;
  /// Summary row/col index (`end + 1` when summaryBelow/summaryRight,
  /// else `start - 1`).
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

/// Run is collapsed when every detail row/col has been hidden (zero
/// height/width). Used by the painter to pick `+`/`-` glyph and by
/// the interact layer to know which way to flip the toggle.
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
  /// Scroll offset on the data axis (px, pre-zoom).
  sx: number;
  sy: number;
  /// First scrolling row/col index (>= 1). Indices below the split are pinned.
  splitX: number;
  splitY: number;
  /// Pinned segment widths/heights (canvas-local px).
  pcw: number;
  prh: number;
  /// Total canvas extent (canvas-local px).
  canvasW: number;
  canvasH: number;
}

export interface OutlineButtonHit {
  run: OutlineRun;
  /// Canvas-local center of the button.
  cx: number;
  cy: number;
  collapsed: boolean;
}

/// Compute canvas-local positions of every visible +/- button. Painter
/// and interact share this so a click lands exactly on the glyph drawn.
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
  /// 1-based level: clicking N means "collapse everything at level >= N".
  /// The final track (level = depth + 1) is the "expand all" affordance.
  level: number;
  cx: number;
  cy: number;
}

/// Corner-numeral buttons (1, 2, …, depth+1) per axis.
///
/// * `axis: "col"` numerals ("collapse cols to depth N"): vertical
///   stack centered in the row-label section of the corner box
///   (x = midpoint between rowGutterW and originX), one numeral per
///   col-gutter track. Same y as the col-track centers so each label
///   visually points at "its" track.
/// * `axis: "row"` numerals ("collapse rows to depth N"): horizontal
///   row centered in the col-letter section of the corner box
///   (y = midpoint between colGutterH and originY), one numeral per
///   row-gutter track. Same x as the row-track centers.
export function outlineCornerHits(g: Grid): OutlineCornerHit[] {
  const out: OutlineCornerHit[] = [];
  if (g.colOutlineDepth > 0) {
    // Numerals in the row-label column section of the corner box.
    // When the row gutter is absent fall back to a small column
    // straddling the inner edge of the col-gutter.
    const cx = g.rowGutterW > 0 ? (g.rowGutterW + g.originX) / 2 : g.originX - HEADER_W / 2;
    for (let lvl = 1; lvl <= g.colOutlineDepth + 1; lvl++) {
      const cy = OUTLINE_GUTTER_PAD + (lvl - 1) * OUTLINE_GUTTER_STEP + OUTLINE_GUTTER_STEP / 2;
      out.push({ axis: "col", level: lvl, cx, cy });
    }
  }
  if (g.rowOutlineDepth > 0) {
    // Numerals in the col-letter row section of the corner box.
    const cy = g.colGutterH > 0 ? (g.colGutterH + g.originY) / 2 : g.originY - HEADER_H / 2;
    for (let lvl = 1; lvl <= g.rowOutlineDepth + 1; lvl++) {
      const cx = OUTLINE_GUTTER_PAD + (lvl - 1) * OUTLINE_GUTTER_STEP + OUTLINE_GUTTER_STEP / 2;
      out.push({ axis: "row", level: lvl, cx, cy });
    }
  }
  return out;
}

export const OUTLINE_BUTTON_HIT_RADIUS = 7;

/// Paint every visible +/- button in one pass. Picks `+` glyph for
/// runs whose detail rows/cols are all hidden, `-` otherwise. Same
/// `outlineButtonHits` source the interact layer uses for hit-tests,
/// so a click is guaranteed to land on the rendered glyph.
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

// ---------- outline gutter ----------
//
// Outline brackets live in dedicated gutter strips outside the row/col
// header label bands (Excel-style):
//
//   ┌──┬───────────┬──────────────────────────┐
//   │NN│ col-gutter│        column letters    │  (col-gutter row, height = colGutterH)
//   ├──┼───────────┼──────────────────────────┤
//   │ N│           │                          │
//   │ N│  row-     │          grid            │  (row-gutter column, width = rowGutterW)
//   │ N│  gutter   │                          │
//   └──┴───────────┴──────────────────────────┘
//
// Each level reserves one OUTLINE_GUTTER_STEP-wide track inside the
// gutter. Level 1 sits at the outer edge (closest to the canvas), level
// N at the inner edge (closest to the row labels / col letters); this
// matches Excel desktop. Each row/col group renders a `[`-shaped
// bracket in its level's track over the detail-cell extent, with a
// small +/- button at the summary side (per `outlinePr.summaryBelow`
// / `summaryRight`, both default true). No interactivity — the button
// is a cosmetic glyph today; clicks fall through to header-selection.

const OUTLINE_STROKE = "#9aa0a6"; // mid-gray, matches Excel's gutter color
const OUTLINE_BUTTON_SIZE = 10;
const OUTLINE_BUTTON_BG = "#ffffff";
const OUTLINE_BUTTON_BORDER = "#6b7280";
const OUTLINE_BUTTON_GLYPH = "#374151";

const COLLAPSED_TICK_STROKE = "#137333";
const COLLAPSED_TICK_WIDTH = 2;

/// Paint a green tick on the top edge of every visible row whose
/// immediate predecessor row is hidden (rowH == 0). The tick spans the
/// row-label band — width HEADER_W, positioned at x = rowGutterW —
/// so it reads as a clear divider on the row-number strip. Mirrors
/// Excel's "click here to expand" affordance for collapsed groups.
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

/// X-coordinate of the vertical bracket stroke for row-outline level
/// `lvl` (1-based). Tracks are laid out left-to-right with level 1
/// nearest the canvas edge and level N nearest the row-label band;
/// matches Excel desktop. Track count = depth + 1 (final track reserved
/// for the level-(N+1) corner numeral — see `drawOutlineCornerButtons`).
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
  // Horizontal stroke (always)
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

/// Level-numeral buttons in the gutter corner. Excel splits these by
/// axis:
///   * Col-axis numerals ("collapse all cols to depth N") sit in the
///     row-gutter column of the corner, vertically stacked, each
///     aligned with its col-gutter horizontal track.
///   * Row-axis numerals ("collapse all rows to depth N") sit in the
///     col-gutter row of the corner, horizontally laid out, each
///     aligned with its row-gutter vertical track.
/// When only one axis has groups we paint just that axis. Geometry
/// matches Excel desktop and the screenshot reference.
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
      /*rowFrom*/ 1,
      /*rowTo*/ Math.max(0, splitY - 1),
      /*offsetY*/ 0,
      /*clipY1*/ g.originY,
      /*clipY2*/ g.originY + prh,
    );
    paintRowRunsForLevel(
      ctx,
      lvlByRow,
      lvl,
      x,
      g,
      summaryBelow,
      /*rowFrom*/ Math.max(1, splitY),
      /*rowTo*/ g.maxRow,
      /*offsetY*/ -sy,
      /*clipY1*/ g.originY + prh,
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
      // Fully hidden run → skip; we'd just stack tick caps on top of
      // each other into a stray notch.
      if (y2 - y1 < 3) {
        runStart = -1;
        continue;
      }
      if (y2 > clipY1 && y1 < clipY2) {
        // Bracket: vertical line over the detail rows, with a short
        // horizontal hook at the non-summary end. Hook points RIGHT
        // (toward the row-label band) to match Excel desktop's `[`
        // shape. The +/- button is painted in a separate pass via
        // `drawOutlineButtons` so collapsed runs (zero bracket
        // extent) still get their +.
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
      /*colFrom*/ 1,
      /*colTo*/ Math.max(0, splitX - 1),
      /*offsetX*/ 0,
      /*clipX1*/ g.originX,
      /*clipX2*/ g.originX + pcw,
    );
    paintColRunsForLevel(
      ctx,
      lvlByCol,
      lvl,
      y,
      g,
      summaryRight,
      /*colFrom*/ Math.max(1, splitX),
      /*colTo*/ g.maxCol,
      /*offsetX*/ -sx,
      /*clipX1*/ g.originX + pcw,
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
        // Hook always points DOWN (toward the column-letter band) to
        // match the `┐` shape Excel desktop draws over column groups.
        // Buttons paint in a separate pass.
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
