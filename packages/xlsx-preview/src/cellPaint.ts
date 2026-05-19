import type { Border, BorderLine, Fill, WorkbookLayout } from "./types.js";
import type { Sheet } from "./types.js";
import { colorToCss } from "./color.js";
import type { Grid } from "./grid.js";
import { buildMergeMaps, cellRect, findCell, mergedRect } from "./geometry.js";
import { iterCellsInRange } from "./columnar.js";
import { resolveCellXf } from "./cellText.js";
import { makeOffscreenCanvas } from "./canvasFactory.js";
import type { CellRect } from "./geometry.js";
import type { Visible } from "./renderTypes.js";

const PATTERN_TILES_8X8: Record<string, number[]> = {
  gray125: [0x88, 0x00, 0x22, 0x00, 0x88, 0x00, 0x22, 0x00],
  gray0625: [0x88, 0x00, 0x00, 0x00, 0x22, 0x00, 0x00, 0x00],
  lightGray: [0x88, 0x22, 0x88, 0x22, 0x88, 0x22, 0x88, 0x22],
  mediumGray: [0xaa, 0x55, 0xaa, 0x55, 0xaa, 0x55, 0xaa, 0x55],
  darkGray: [0x77, 0xdd, 0x77, 0xdd, 0x77, 0xdd, 0x77, 0xdd],
  lightHorizontal: [0xff, 0x00, 0x00, 0x00, 0xff, 0x00, 0x00, 0x00],
  darkHorizontal: [0xff, 0xff, 0x00, 0x00, 0xff, 0xff, 0x00, 0x00],
  lightVertical: [0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11],
  darkVertical: [0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33],
  lightDown: [0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80],
  darkDown: [0x03, 0x06, 0x0c, 0x18, 0x30, 0x60, 0xc0, 0x81],
  lightUp: [0x80, 0x40, 0x20, 0x10, 0x08, 0x04, 0x02, 0x01],
  darkUp: [0xc0, 0x60, 0x30, 0x18, 0x0c, 0x06, 0x03, 0x81],
  lightGrid: [0xff, 0x11, 0x11, 0x11, 0xff, 0x11, 0x11, 0x11],
  darkGrid: [0xff, 0xff, 0x33, 0x33, 0xff, 0xff, 0x33, 0x33],
  lightTrellis: [0x81, 0x42, 0x24, 0x18, 0x18, 0x24, 0x42, 0x81],
  darkTrellis: [0xc3, 0x66, 0x3c, 0x18, 0x18, 0x3c, 0x66, 0xc3],
};

const patternCache = new Map<string, CanvasPattern | null>();

function buildPattern(
  ctx: CanvasRenderingContext2D,
  patternType: string,
  fgCss: string,
  bgCss: string | null,
): CanvasPattern | null {
  const key = `${patternType}|${fgCss}|${bgCss ?? ""}`;
  const hit = patternCache.get(key);
  if (hit !== undefined) return hit;
  const tile = PATTERN_TILES_8X8[patternType];
  if (!tile) {
    patternCache.set(key, null);
    return null;
  }

  const off = makeOffscreenCanvas(8, 8);
  const octx = off.getContext("2d")!;
  if (bgCss) {
    octx.fillStyle = bgCss;
    octx.fillRect(0, 0, 8, 8);
  }
  octx.fillStyle = fgCss;
  for (let y = 0; y < 8; y++) {
    const row = tile[y] ?? 0;
    for (let x = 0; x < 8; x++) {
      if (row & (1 << x)) octx.fillRect(x, y, 1, 1);
    }
  }
  const pat = ctx.createPattern(off as unknown as CanvasImageSource, "repeat");
  patternCache.set(key, pat);
  return pat;
}

function collectStops(fill: Fill): Array<{ pos: number; css: string }> {
  const stops = (fill.gradientStops ?? []).map((s) => ({
    pos: Math.max(0, Math.min(1, s.position ?? 0)),
    css: colorToCss(s.color, "#ffffff"),
  }));
  if (stops.length >= 2) return stops;

  const c1 = fill.fgColor ? colorToCss(fill.fgColor, "#ffffff") : null;
  const c2 = fill.bgColor ? colorToCss(fill.bgColor, "#ffffff") : c1;
  if (!c1 || !c2) return [];
  if (stops.length === 1) {
    const s = stops[0]!;
    return s.pos < 0.5 ? [s, { pos: 1, css: c2 }] : [{ pos: 0, css: c1 }, s];
  }
  return [
    { pos: 0, css: c1 },
    { pos: 1, css: c2 },
  ];
}

function paintGradientFill(ctx: CanvasRenderingContext2D, rect: CellRect, fill: Fill): void {
  const stops = collectStops(fill);
  if (stops.length === 0) return;
  const type = fill.gradientType ?? "linear";
  if (type === "path") {
    const li = Math.max(0, Math.min(1, fill.gradientLeft ?? 0));
    const ri = Math.max(0, Math.min(1, fill.gradientRight ?? 0));
    const ti = Math.max(0, Math.min(1, fill.gradientTop ?? 0));
    const bi = Math.max(0, Math.min(1, fill.gradientBottom ?? 0));
    const ix = rect.x + li * rect.w;
    const iy = rect.y + ti * rect.h;
    const iw = Math.max(0, rect.w * Math.max(0, 1 - li - ri));
    const ih = Math.max(0, rect.h * Math.max(0, 1 - ti - bi));

    ctx.fillStyle = stops[0]!.css;
    ctx.fillRect(rect.x, rect.y, rect.w, rect.h);
    const cx = ix + iw / 2;
    const cy = iy + ih / 2;

    const r0 = Math.hypot(iw, ih) / 2;

    const corners = [
      [rect.x, rect.y],
      [rect.x + rect.w, rect.y],
      [rect.x, rect.y + rect.h],
      [rect.x + rect.w, rect.y + rect.h],
    ] as const;
    const r1 = Math.max(...corners.map(([x, y]) => Math.hypot(x - cx, y - cy)));
    if (r1 <= r0 + 0.5) return;
    const grad = ctx.createRadialGradient(cx, cy, r0, cx, cy, r1);
    for (const s of stops) grad.addColorStop(s.pos, s.css);
    ctx.fillStyle = grad;
    ctx.fillRect(rect.x, rect.y, rect.w, rect.h);
    return;
  }

  const deg = fill.gradientDegree ?? 0;
  const theta = (deg * Math.PI) / 180;
  const dx = Math.cos(theta);
  const dy = Math.sin(theta);

  const projs = [0, rect.w * dx, rect.h * dy, rect.w * dx + rect.h * dy];
  const pmin = Math.min(...projs);
  const pmax = Math.max(...projs);

  const x0 = rect.x + pmin * dx;
  const y0 = rect.y + pmin * dy;
  const x1 = rect.x + pmax * dx;
  const y1 = rect.y + pmax * dy;
  const grad = Math.hypot(x1 - x0, y1 - y0) < 0.5 ? null : ctx.createLinearGradient(x0, y0, x1, y1);
  if (grad) {
    for (const s of stops) grad.addColorStop(s.pos, s.css);
    ctx.fillStyle = grad;
  } else {
    ctx.fillStyle = stops[0]!.css;
  }
  ctx.fillRect(rect.x, rect.y, rect.w, rect.h);
}

export function paintFill(ctx: CanvasRenderingContext2D, rect: CellRect, fill: Fill): void {
  const pt = fill.patternType;
  if (!pt || pt === "none") return;
  if (pt === "solid" && fill.fgColor) {
    ctx.fillStyle = colorToCss(fill.fgColor, "#ffffff");
    ctx.fillRect(rect.x, rect.y, rect.w, rect.h);
    return;
  }
  if (pt === "gradient") {
    paintGradientFill(ctx, rect, fill);
    return;
  }
  if (PATTERN_TILES_8X8[pt]) {
    const fgCss = colorToCss(fill.fgColor, "#000000");
    const bgCss = fill.bgColor ? colorToCss(fill.bgColor, "#ffffff") : null;
    if (bgCss) {
      ctx.fillStyle = bgCss;
      ctx.fillRect(rect.x, rect.y, rect.w, rect.h);
    }
    const pat = buildPattern(ctx, pt, fgCss, bgCss);
    if (!pat) return;

    ctx.save();
    ctx.translate(rect.x, rect.y);
    ctx.fillStyle = pat;
    ctx.fillRect(0, 0, rect.w, rect.h);
    ctx.restore();
  }
}

const COL_STYLE_1BASED = new WeakMap<Sheet, Map<number, number>>();
function colStyleMap1Based(sheet: Sheet): Map<number, number> {
  let m = COL_STYLE_1BASED.get(sheet);
  if (m) return m;
  m = new Map<number, number>();
  for (const col of sheet.cols) {
    if (col.styleIndex === undefined) continue;
    for (let i = col.min; i <= col.max; i++) m.set(i, col.styleIndex);
  }
  COL_STYLE_1BASED.set(sheet, m);
  return m;
}

export function drawDefaultFills(
  ctx: CanvasRenderingContext2D,
  sheet: Sheet,
  layout: WorkbookLayout,
  g: Grid,
  vis: Visible,
): void {
  const styles = layout.styles;
  const xfs = styles.cellXfs;
  const fillFor = (xfId: number) => {
    const xf = xfs[xfId];
    if (!xf) return undefined;
    return xf.fillId !== undefined ? styles.fills[xf.fillId] : undefined;
  };

  const colMap = colStyleMap1Based(sheet);
  if (colMap.size > 0) {
    const colFirst = Math.max(1, vis.firstCol);
    const colLast = Math.min(sheet.maxCol, vis.lastCol);
    const rowFirst = Math.max(1, vis.firstRow);
    const rowLast = Math.min(sheet.maxRow, vis.lastRow);
    if (colFirst <= colLast && rowFirst <= rowLast) {
      const yTop = g.rowY[rowFirst] ?? 0;
      const yBot = g.rowY[rowLast + 1] ?? yTop;
      const h = yBot - yTop;
      if (h > 0) {
        for (let c = colFirst; c <= colLast; c++) {
          const xfId = colMap.get(c);
          if (xfId === undefined) continue;
          const fill = fillFor(xfId);
          if (!fill) continue;
          const x = g.colX[c] ?? 0;
          const w = (g.colX[c + 1] ?? x) - x;
          if (w <= 0) continue;
          paintFill(ctx, { x, y: yTop, w, h }, fill);
        }
      }
    }
  }

  const meta = sheet.decodedRowMeta;
  if (meta.count > 0 && sheet.maxCol >= 1) {
    const colFirst = Math.max(1, vis.firstCol);
    const colLast = Math.min(sheet.maxCol, vis.lastCol);
    if (colFirst <= colLast) {
      const xLeft = g.colX[colFirst] ?? 0;
      const xRight = g.colX[colLast + 1] ?? xLeft;
      const w = xRight - xLeft;
      if (w > 0) {
        for (let i = 0; i < meta.count; i++) {
          const r = meta.index[i] ?? 0;
          if (r < vis.firstRow || r > vis.lastRow) continue;
          const sIdx = meta.styleIdx[i] ?? -1;
          if (sIdx < 0) continue;
          const fill = fillFor(sIdx);
          if (!fill) continue;
          const y = g.rowY[r] ?? 0;
          const h = (g.rowY[r + 1] ?? y) - y;
          if (h <= 0) continue;
          paintFill(ctx, { x: xLeft, y, w, h }, fill);
        }
      }
    }
  }
}

export function drawCellBackgrounds(
  ctx: CanvasRenderingContext2D,
  sheet: Sheet,
  layout: WorkbookLayout,
  g: Grid,
  vis: Visible,
): void {
  const styles = layout.styles;
  const { covered, topLeftOf } = buildMergeMaps(sheet);

  for (const m of sheet.merges) {
    if (m.r2 < vis.firstRow || m.r1 > vis.lastRow) continue;
    if (m.c2 < vis.firstCol || m.c1 > vis.lastCol) continue;
    const tl = findCell(sheet, m.r1, m.c1);
    if (!tl) continue;
    const xf = resolveCellXf(tl, sheet, layout);
    if (!xf) continue;
    const fill = xf.fillId !== undefined ? styles.fills[xf.fillId] : undefined;
    if (!fill) continue;
    paintFill(ctx, mergedRect(g, m), fill);
  }

  iterCellsInRange(sheet, vis.firstRow, vis.lastRow, vis.firstCol, vis.lastCol, (cell) => {
    const k = `${cell.r}:${cell.c}`;
    if (covered.has(k)) return;
    if (topLeftOf.has(k)) return;
    const xf = resolveCellXf(cell, sheet, layout);
    if (!xf) return;
    const fill = xf.fillId !== undefined ? styles.fills[xf.fillId] : undefined;
    if (!fill) return;
    paintFill(ctx, cellRect(g, cell.r, cell.c), fill);
  });
}

function borderWidth(line: BorderLine): number {
  switch (line.style) {
    case "thin":
    case "hair":
    case "dotted":
    case "dashed":
    case "dashDot":
    case "dashDotDot":
      return 1;
    case "medium":
    case "mediumDashed":
    case "mediumDashDot":
    case "mediumDashDotDot":
    case "slantDashDot":
      return 2;
    case "thick":
      return 3;
    case "double":
      return 1;
    default:
      return 1;
  }
}

function borderDash(style: string): number[] | null {
  switch (style) {
    case "dotted":
      return [1, 1];
    case "hair":
      return [1, 2];
    case "dashed":
    case "mediumDashed":
      return [3, 2];
    case "dashDot":
    case "mediumDashDot":
    case "slantDashDot":
      return [3, 1, 1, 1];
    case "dashDotDot":
    case "mediumDashDotDot":
      return [3, 1, 1, 1, 1, 1];
    default:
      return null;
  }
}

function drawBorderLine(
  ctx: CanvasRenderingContext2D,
  x1: number,
  y1: number,
  x2: number,
  y2: number,
  line: BorderLine,
): void {
  const w = borderWidth(line);
  ctx.strokeStyle = colorToCss(line.color, "#000000");
  ctx.lineWidth = w;
  const dash = borderDash(line.style);
  ctx.setLineDash(dash ?? []);
  ctx.beginPath();
  ctx.moveTo(x1, y1);
  ctx.lineTo(x2, y2);
  ctx.stroke();
  if (line.style === "double") {
    ctx.beginPath();
    const horizontal = y1 === y2;
    if (horizontal) {
      ctx.moveTo(x1, y1 + 2);
      ctx.lineTo(x2, y2 + 2);
    } else {
      ctx.moveTo(x1 + 2, y1);
      ctx.lineTo(x2 + 2, y2);
    }
    ctx.stroke();
  }
  ctx.setLineDash([]);
}

function drawDiagonalBorders(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  w: number,
  h: number,
  b: Border,
): void {
  if (!b.diagonal) return;
  if (!b.diagonalUp && !b.diagonalDown) return;
  ctx.save();

  ctx.beginPath();
  ctx.rect(x, y, w, h);
  ctx.clip();
  if (b.diagonalDown) drawBorderLine(ctx, x, y, x + w, y + h, b.diagonal);
  if (b.diagonalUp) drawBorderLine(ctx, x, y + h, x + w, y, b.diagonal);
  ctx.restore();
}

export function drawCellBorders(
  ctx: CanvasRenderingContext2D,
  sheet: Sheet,
  layout: WorkbookLayout,
  g: Grid,
  vis: Visible,
): void {
  const styles = layout.styles;
  const { covered, topLeftOf } = buildMergeMaps(sheet);

  for (const m of sheet.merges) {
    if (m.r2 < vis.firstRow || m.r1 > vis.lastRow) continue;
    if (m.c2 < vis.firstCol || m.c1 > vis.lastCol) continue;
    const tl = findCell(sheet, m.r1, m.c1);
    if (!tl) continue;
    const xf = resolveCellXf(tl, sheet, layout);
    if (!xf || xf.borderId === undefined) continue;
    const b = styles.borders[xf.borderId];
    if (!b) continue;
    const { x, y, w, h } = mergedRect(g, m);
    if (b.top) drawBorderLine(ctx, x, y, x + w, y, b.top);
    if (b.bottom) drawBorderLine(ctx, x, y + h, x + w, y + h, b.bottom);
    if (b.left) drawBorderLine(ctx, x, y, x, y + h, b.left);
    if (b.right) drawBorderLine(ctx, x + w, y, x + w, y + h, b.right);
    drawDiagonalBorders(ctx, x, y, w, h, b);
  }

  iterCellsInRange(sheet, vis.firstRow, vis.lastRow, vis.firstCol, vis.lastCol, (cell) => {
    const k = `${cell.r}:${cell.c}`;
    const xf = resolveCellXf(cell, sheet, layout);
    if (!xf || xf.borderId === undefined) return;
    const b = styles.borders[xf.borderId];
    if (!b) return;

    const merge = topLeftOf.get(k);
    const isCovered = covered.has(k);
    if (isCovered && merge) {
      const cr = cellRect(g, cell.r, cell.c);
      const { x, y, w, h } = cr;
      const onTop = cell.r === merge.r1;
      const onBottom = cell.r === merge.r2;
      const onLeft = cell.c === merge.c1;
      const onRight = cell.c === merge.c2;
      if (onTop && b.top) drawBorderLine(ctx, x, y, x + w, y, b.top);
      if (onBottom && b.bottom) drawBorderLine(ctx, x, y + h, x + w, y + h, b.bottom);
      if (onLeft && b.left) drawBorderLine(ctx, x, y, x, y + h, b.left);
      if (onRight && b.right) drawBorderLine(ctx, x + w, y, x + w, y + h, b.right);
      return;
    }

    if (merge) return;
    const rect = cellRect(g, cell.r, cell.c);
    const { x, y, w, h } = rect;
    if (b.top) drawBorderLine(ctx, x, y, x + w, y, b.top);
    if (b.bottom) drawBorderLine(ctx, x, y + h, x + w, y + h, b.bottom);
    if (b.left) drawBorderLine(ctx, x, y, x, y + h, b.left);
    if (b.right) drawBorderLine(ctx, x + w, y, x + w, y + h, b.right);
    drawDiagonalBorders(ctx, x, y, w, h, b);
  });
}
