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

// Build the linear/path gradient for a `<gradientFill>` element. Spec:
// ECMA-376 §18.8.24. Two flavors:
//   linear: gradient runs along an axis rotated `degree` degrees clockwise
//           from the cell's left→right vector. Stops at fractional positions
//           along that axis (clamped to the cell). 0° = L→R, 90° = T→B.
//   path:   stops radiate from a rectangular `inner` region (defined by
//           `left`/`right`/`top`/`bottom` insets, each a fraction of the
//           cell's width/height) outward to the cell rect. Position 0 paints
//           inside the inner rect; position 1 paints at the cell edge.
function collectStops(fill: Fill): Array<{ pos: number; css: string }> {
  const stops = (fill.gradientStops ?? []).map((s) => ({
    pos: Math.max(0, Math.min(1, s.position ?? 0)),
    css: colorToCss(s.color, "#ffffff"),
  }));
  if (stops.length >= 2) return stops;
  // Pre-schema gradients only carried fg/bg; preserve that fallback.
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

function paintGradientFill(
  ctx: CanvasRenderingContext2D,
  rect: CellRect,
  fill: Fill,
): void {
  const stops = collectStops(fill);
  if (stops.length === 0) return;
  const type = fill.gradientType ?? "linear";
  if (type === "path") {
    // Inner convergence rect (clamped + ordered).
    const li = Math.max(0, Math.min(1, fill.gradientLeft ?? 0));
    const ri = Math.max(0, Math.min(1, fill.gradientRight ?? 0));
    const ti = Math.max(0, Math.min(1, fill.gradientTop ?? 0));
    const bi = Math.max(0, Math.min(1, fill.gradientBottom ?? 0));
    const ix = rect.x + li * rect.w;
    const iy = rect.y + ti * rect.h;
    const iw = Math.max(0, rect.w * Math.max(0, 1 - li - ri));
    const ih = Math.max(0, rect.h * Math.max(0, 1 - ti - bi));
    // Excel paints the innermost color uniformly across the inner rect,
    // then radiates outward. Canvas only gives us an elliptical radial
    // gradient so we approximate: fill the cell with the innermost stop
    // first, then overlay the radial transition from the inner rect's
    // bounding circle out to a circle that covers the farthest cell
    // corner. This matches Excel for non-degenerate insets and degrades
    // gracefully when one or more sides are 0 (collapses to the matching
    // cell corner / edge midpoint).
    ctx.fillStyle = stops[0]!.css;
    ctx.fillRect(rect.x, rect.y, rect.w, rect.h);
    const cx = ix + iw / 2;
    const cy = iy + ih / 2;
    // Inner radius: half of the inner rect's diagonal (so the inner
    // color reaches every corner of the inner rect).
    const r0 = Math.hypot(iw, ih) / 2;
    // Outer radius: distance from inner-rect center to the farthest
    // cell corner.
    const corners = [
      [rect.x, rect.y],
      [rect.x + rect.w, rect.y],
      [rect.x, rect.y + rect.h],
      [rect.x + rect.w, rect.y + rect.h],
    ] as const;
    const r1 = Math.max(...corners.map(([x, y]) => Math.hypot(x - cx, y - cy)));
    if (r1 <= r0 + 0.5) return; // degenerate; inner fill is enough
    const grad = ctx.createRadialGradient(cx, cy, r0, cx, cy, r1);
    for (const s of stops) grad.addColorStop(s.pos, s.css);
    ctx.fillStyle = grad;
    ctx.fillRect(rect.x, rect.y, rect.w, rect.h);
    return;
  }
  // Linear. `degree` rotates the L→R axis clockwise (in screen space).
  // We compute the projection of the cell rect onto the rotated axis,
  // then place start/end at the rect's projected extents so that
  // position 0 = first "hit" pixel and position 1 = last. This matches
  // CSS `linear-gradient(<degree>+90deg, ...)` style intuition while
  // staying in canvas's two-point gradient API.
  const deg = fill.gradientDegree ?? 0;
  const theta = (deg * Math.PI) / 180;
  const dx = Math.cos(theta);
  const dy = Math.sin(theta);
  // Project the four corners onto the unit axis (relative to rect origin).
  const projs = [
    0,
    rect.w * dx,
    rect.h * dy,
    rect.w * dx + rect.h * dy,
  ];
  const pmin = Math.min(...projs);
  const pmax = Math.max(...projs);
  // Starting point is the corner whose projection equals pmin; end is the
  // pmax corner. Computed by stepping from rect origin along (dx, dy).
  const x0 = rect.x + pmin * dx;
  const y0 = rect.y + pmin * dy;
  const x1 = rect.x + pmax * dx;
  const y1 = rect.y + pmax * dy;
  const grad =
    Math.hypot(x1 - x0, y1 - y0) < 0.5
      ? null
      : ctx.createLinearGradient(x0, y0, x1, y1);
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

/// Per-sheet column-style lookup (1-based, like `Col.min`/`Col.max`).
/// Mirror of the map in `cellText.ts` but keyed 1-based so the paint
/// loops below can use Excel-style col indices directly. Cached on
/// the sheet so we build it once.
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

/// Paint row-level and column-level default fills across the visible
/// viewport, *before* per-cell backgrounds. OOXML §18.3.1.4 says a
/// cell without its own xf inherits `row.s → col.style → xf 0`; the
/// rest of the renderer applies this fallback for text/border via
/// `resolveCellXf`, but the fill path used to silently skip cells
/// that simply didn't exist in `sheetData` — leaving empty rows like
/// Cover!7 (which carries a solid-blue `<row s=N>` with no children)
/// completely unpainted.
///
/// Strategy:
///   * Column fills first (lowest priority of the two), per column
///     `c` in `[1, maxCol]` ∩ visible, painted across all visible rows.
///   * Row fills second (higher priority), per styled `rowMeta` row
///     `r` in vis, painted across cols `[1, sheet.maxCol]`.
///
/// Per-cell xf fills paint on top in `drawCellBackgrounds`, so this
/// only fills the truly-empty cells. The horizontal extent of row
/// fills is clipped to `sheet.maxCol` so the gray "sheet-area" of a
/// cover page stops at the last styled column instead of bleeding
/// out to infinity (matches hsx/Excel).
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

  // Column fills (lowest priority of the two).
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

  // Row fills (higher priority — paint over col fills).
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
  // Pass 1: paint merges whose extent overlaps `vis`. This handles merges
  // crossing a freeze split, where the merge's top-left cell may live in a
  // different pane than the rest of the merge. Pane clipping ensures each
  // pane only paints the slice of the merge it owns.
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
  // Pass 2: regular (non-merge) cells.
  iterCellsInRange(sheet, vis.firstRow, vis.lastRow, vis.firstCol, vis.lastCol, (cell) => {
    const k = `${cell.r}:${cell.c}`;
    if (covered.has(k)) return;
    if (topLeftOf.has(k)) return; // handled by pass 1
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
  layout: WorkbookLayout,
  g: Grid,
  vis: Visible,
): void {
  const styles = layout.styles;
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
  // Pass 2: per-cell borders.
  iterCellsInRange(sheet, vis.firstRow, vis.lastRow, vis.firstCol, vis.lastCol, (cell) => {
    const k = `${cell.r}:${cell.c}`;
    const xf = resolveCellXf(cell, sheet, layout);
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
