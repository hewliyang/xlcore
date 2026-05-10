import type { Border, BorderLine, Fill, Sheet, Styles } from "./types.js";
import { colorToCss } from "./color.js";
import type { Grid } from "./grid.js";
import { buildMergeMaps, cellRect, findCell, mergedRect } from "./geometry.js";
import { iterCellsInRange } from "./columnar.js";
import type { CellRect } from "./geometry.js";
import type { Visible } from "./renderTypes.js";

// 8x8 binary tiles for OOXML hatch patterns. Each row is an 8-bit mask;
// LSB = leftmost pixel, MSB = rightmost. Sources: OOXML §18.18.55 +
// the GDI+ HatchStyle reference Excel uses to render these.
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

// Cache CanvasPatterns by (type|fg|bg). Built lazily; cleared per render via
// `patternCache.clear()` so the underlying ctx isn't kept across frames.
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
  // Use a small offscreen canvas. Don't scale to DPR — letting the
  // browser nearest-neighbor a 1:1 tile keeps the look crisp and matches
  // Excel's pixel-quantized hatch.
  const off = document.createElement("canvas");
  off.width = 8;
  off.height = 8;
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
  const pat = ctx.createPattern(off, "repeat");
  patternCache.set(key, pat);
  return pat;
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
    const stops = fill.gradientStops ?? [];
    const c1 = stops[0] ?? fill.fgColor;
    const c2 = stops[stops.length - 1] ?? fill.bgColor ?? c1;
    if (!c1 || !c2) return;
    const grad = ctx.createLinearGradient(rect.x, rect.y, rect.x + rect.w, rect.y);
    grad.addColorStop(0, colorToCss(c1, "#ffffff"));
    grad.addColorStop(1, colorToCss(c2, "#ffffff"));
    ctx.fillStyle = grad;
    ctx.fillRect(rect.x, rect.y, rect.w, rect.h);
    return;
  }
  if (PATTERN_TILES_8X8[pt]) {
    // Defaults match Excel: missing fg=black, missing bg=transparent.
    const fgCss = colorToCss(fill.fgColor, "#000000");
    const bgCss = fill.bgColor ? colorToCss(fill.bgColor, "#ffffff") : null;
    if (bgCss) {
      ctx.fillStyle = bgCss;
      ctx.fillRect(rect.x, rect.y, rect.w, rect.h);
    }
    const pat = buildPattern(ctx, pt, fgCss, bgCss);
    if (!pat) return;
    // Align tile origin to the cell so adjacent same-pattern cells line up.
    ctx.save();
    ctx.translate(rect.x, rect.y);
    ctx.fillStyle = pat;
    ctx.fillRect(0, 0, rect.w, rect.h);
    ctx.restore();
  }
}

export function drawCellBackgrounds(
  ctx: CanvasRenderingContext2D,
  sheet: Sheet,
  styles: Styles,
  g: Grid,
  vis: Visible,
): void {
  const { covered, topLeftOf } = buildMergeMaps(sheet);
  // Pass 1: paint merges whose extent overlaps `vis`. This handles merges
  // crossing a freeze split, where the merge's top-left cell may live in a
  // different pane than the rest of the merge. Pane clipping ensures each
  // pane only paints the slice of the merge it owns.
  for (const m of sheet.merges) {
    if (m.r2 < vis.firstRow || m.r1 > vis.lastRow) continue;
    if (m.c2 < vis.firstCol || m.c1 > vis.lastCol) continue;
    const tl = findCell(sheet, m.r1, m.c1);
    if (!tl) continue;
    const xf = tl.styleIndex !== undefined ? styles.cellXfs[tl.styleIndex] : undefined;
    if (!xf) continue;
    const fill = xf.fillId !== undefined ? styles.fills[xf.fillId] : undefined;
    if (!fill) continue;
    paintFill(ctx, mergedRect(g, m), fill);
  }
  // Pass 2: regular (non-merge) cells.
  iterCellsInRange(sheet, vis.firstRow, vis.lastRow, vis.firstCol, vis.lastCol, (cell) => {
    const k = `${cell.r}:${cell.c}`;
    if (covered.has(k)) return;
    if (topLeftOf.has(k)) return; // handled by pass 1
    const xf = cell.styleIndex !== undefined ? styles.cellXfs[cell.styleIndex] : undefined;
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
      return 1; // simplified; double-render handled below
    default:
      return 1;
  }
}

// Per-style dash pattern (in pixels). `null` means solid.
// Patterns chosen to roughly match Excel's visual cadence.
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

/// Draw the diagonal segment(s) for a cell rect. OOXML `diagonalDown`
/// = top-left → bottom-right slash; `diagonalUp` = bottom-left →
/// top-right slash. Both share one `<diagonal>` style+color. Clipped
/// to the cell rect so the diagonal never bleeds into neighbors. For
/// a merged region pass the merged rect; the diagonal spans the full
/// merge.
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
  // Clip strictly to the cell so wide stroke widths don't escape.
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
  styles: Styles,
  g: Grid,
  vis: Visible,
): void {
  const { covered, topLeftOf } = buildMergeMaps(sheet);
  // Pass 1: paint merged-rect borders for any merge whose extent overlaps
  // `vis`. This handles merges that cross a freeze split — the top-left
  // cell may sit in a different pane than the rest of the merge, and its
  // pane's clip would otherwise cut off the right/bottom edges. Pane
  // clipping ensures each pane only paints its own slice of the long edges.
  // Perimeter `covered` cells in pass 2 may double-paint their segments;
  // for solid lines this is a no-op and matches Excel's perimeter-borders
  // model where each perimeter cell carries its own border defs.
  for (const m of sheet.merges) {
    if (m.r2 < vis.firstRow || m.r1 > vis.lastRow) continue;
    if (m.c2 < vis.firstCol || m.c1 > vis.lastCol) continue;
    const tl = findCell(sheet, m.r1, m.c1);
    if (!tl) continue;
    const xf = tl.styleIndex !== undefined ? styles.cellXfs[tl.styleIndex] : undefined;
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
  // Pass 2: per-cell borders.
  iterCellsInRange(sheet, vis.firstRow, vis.lastRow, vis.firstCol, vis.lastCol, (cell) => {
    const k = `${cell.r}:${cell.c}`;
    const xf = cell.styleIndex !== undefined ? styles.cellXfs[cell.styleIndex] : undefined;
    if (!xf || xf.borderId === undefined) return;
    const b = styles.borders[xf.borderId];
    if (!b) return;

    // Excel/SpreadJS quirk: when a range has a border applied around it,
    // the right/bottom edges of a *merged* region are stored on the cells
    // along the merge perimeter, not on the merge's top-left cell. Those
    // perimeter cells are "covered" by the merge and thus normally hidden
    // from the renderer — but their border definitions still need to
    // paint, otherwise the merged box looks open on the right/bottom.
    const merge = topLeftOf.get(k);
    const isCovered = covered.has(k);
    if (isCovered && merge) {
      // Draw only the side(s) that lie on the merge boundary, using this
      // cell's *own* (small) rect so each cell paints just its segment of
      // the long edge. Adjacent cells stitch into a continuous line.
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

    // Regular (non-merged) cell. Merge top-lefts are handled by pass 1.
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
