import type { Sheet, WorkbookLayout } from "./types.js";
import { setActiveTheme } from "./color.js";
export { applyTint } from "./color.js";
import { buildGrid } from "./grid.js";
import type { Grid } from "./grid.js";
export { HEADER_H, HEADER_W, buildGrid } from "./grid.js";
import { drawGridLines } from "./geometry.js";
import { splitPanes } from "./panes.js";
export { paneAtPoint, frozenDims } from "./panes.js";
import { resolveSelection, drawSelection } from "./selection.js";
import { drawHighlights } from "./highlights.js";
import { drawDrawings } from "./drawings.js";
import { drawCellBackgrounds, drawCellBorders, drawDefaultFills } from "./cellPaint.js";
import { drawFreezeIndicators } from "./freezeIndicators.js";
import { computeOverflowSuppressedSides, drawCellText } from "./textRenderer.js";
import {
  computeCfDxfMap,
  computeCfIconState,
  computeCfStopLocks,
  computeCfTextSuppress,
  drawConditionalFormats,
} from "./conditionalFormatting.js";
import type { Dxf } from "./types.js";
import { drawCfIcons } from "./cfIcons.js";
import { drawSparklines } from "./sparklines.js";
import {
  computeHyperlinkDxfs,
  computeTableState,
  drawCommentMarkers,
  drawFilterArrows,
  drawHeaders,
} from "./sheetChrome.js";
import type { Pane, RenderOptions, Visible } from "./renderTypes.js";
export type { RenderOptions, Viewport } from "./renderTypes.js";

export function render(
  canvas:
    | HTMLCanvasElement
    | { width: number; height: number; getContext(t: "2d"): CanvasRenderingContext2D | null },
  sheet: Sheet,
  layout: WorkbookLayout,
  opts: RenderOptions = {},
): void {
  const ctx = canvas.getContext("2d");
  if (!ctx) throw new Error("no 2d context");

  setActiveTheme(layout.theme);

  const renderHeaders = opts.renderHeaders ?? true;
  const dpr = opts.scale ?? (typeof window !== "undefined" ? window.devicePixelRatio || 1 : 1);
  const zoom = opts.zoom ?? 1;
  const vp = opts.viewport;
  const requiredFarX = vp ? vp.x + vp.w : undefined;
  const requiredFarY = vp ? vp.y + vp.h : undefined;
  const grid: Grid = buildGrid(
    sheet,
    opts.colOverrides,
    opts.rowOverrides,
    requiredFarX,
    requiredFarY,
  );

  const W = vp ? vp.w : grid.totalW;
  const H = vp ? vp.h : grid.totalH;
  const total = zoom * dpr;
  const pixelW = Math.ceil(W * total);
  const pixelH = Math.ceil(H * total);
  if (canvas.width !== pixelW) canvas.width = pixelW;
  if (canvas.height !== pixelH) canvas.height = pixelH;
  if ("style" in canvas && (canvas as HTMLCanvasElement).style) {
    (canvas as HTMLCanvasElement).style.width = `${W * zoom}px`;
    (canvas as HTMLCanvasElement).style.height = `${H * zoom}px`;
  }
  ctx.setTransform(total, 0, 0, total, 0, 0);

  ctx.fillStyle = "#ffffff";
  ctx.fillRect(0, 0, W, H);

  const sel = resolveSelection(opts, grid);
  const panes = splitPanes(sheet, grid, vp ?? null, W, H);

  const pre = getCfPrecompute(sheet, layout);
  const cfLocks = pre.cfLocks;
  const cfTextSuppress = new Set(pre.cfTextSuppress);
  const { cfDxfs: precomputedDxfs, cfIconReserve, cfIconDraw, cfIconSuppress } = pre;
  for (const k of cfIconSuppress) cfTextSuppress.add(k);
  const cfDxfs = new Map(precomputedDxfs);

  const { tableDxfs, filterArrows } = computeTableState(sheet, layout, visibleEnvelope(panes));
  for (const [k, dxf] of tableDxfs) {
    if (!cfDxfs.has(k)) cfDxfs.set(k, dxf);
  }

  const hyperlinkDxfs = pre.hyperlinkDxfs;
  for (const [k, dxf] of hyperlinkDxfs) {
    if (!cfDxfs.has(k)) cfDxfs.set(k, dxf);
  }

  for (const pane of panes) {
    ctx.save();
    ctx.beginPath();
    ctx.rect(pane.cx, pane.cy, pane.cw, pane.ch);
    ctx.clip();
    ctx.translate(pane.tx, pane.ty);

    drawGridLines(ctx, sheet, grid, pane.vis, opts.renderGridLines);
    drawDefaultFills(ctx, sheet, layout, grid, pane.vis);
    drawCellBackgrounds(ctx, sheet, layout, grid, pane.vis);
    drawConditionalFormats(ctx, sheet, layout, grid, pane.vis, cfDxfs, cfLocks);
    const overflowSuppressed = computeOverflowSuppressedSides(ctx, sheet, layout, grid, pane.vis);
    drawCellBorders(ctx, sheet, layout, grid, pane.vis, overflowSuppressed);
    drawCellText(ctx, sheet, layout, grid, pane.vis, cfDxfs, cfTextSuppress, cfIconReserve);
    drawCfIcons(ctx, sheet, grid, pane.vis, cfIconDraw);
    drawSparklines(ctx, sheet, grid, pane.vis);
    drawFilterArrows(ctx, sheet, grid, pane.vis, filterArrows);
    drawDrawings(ctx, sheet, grid);
    drawCommentMarkers(ctx, sheet, grid, pane.vis);
    if (opts.highlights && opts.highlights.length > 0)
      drawHighlights(ctx, grid, opts.highlights);
    if (sel) drawSelection(ctx, sheet, grid, sel, opts.activeCell ?? null);

    ctx.restore();
  }

  drawFreezeIndicators(ctx, sheet, grid, W, H);
  if (renderHeaders) drawHeaders(ctx, sheet, grid, sel, vp ?? null, W, H, panes);
}

interface CfPrecompute {
  cfLocks: Map<string, number>;
  cfDxfs: Map<string, Dxf>;
  cfTextSuppress: Set<string>;
  cfIconReserve: Map<string, number>;
  cfIconDraw: Map<string, { iconSet: string; idx: number; n: number }>;
  cfIconSuppress: Set<string>;
  hyperlinkDxfs: Map<string, Dxf>;
}
const cfPrecomputeCache = new WeakMap<Sheet, WeakMap<WorkbookLayout, CfPrecompute>>();

function getCfPrecompute(sheet: Sheet, layout: WorkbookLayout): CfPrecompute {
  let inner = cfPrecomputeCache.get(sheet);
  if (!inner) {
    inner = new WeakMap();
    cfPrecomputeCache.set(sheet, inner);
  }
  const hit = inner.get(layout);
  if (hit) return hit;
  const cfLocks = computeCfStopLocks(sheet, layout);
  const cfDxfs = computeCfDxfMap(sheet, layout, cfLocks);
  const cfTextSuppress = computeCfTextSuppress(sheet, cfLocks);
  const iconState = computeCfIconState(sheet, cfLocks);
  const hyperlinkDxfs = computeHyperlinkDxfs(sheet, layout);
  const value: CfPrecompute = {
    cfLocks,
    cfDxfs,
    cfTextSuppress,
    cfIconReserve: iconState.cfIconReserve,
    cfIconDraw: iconState.cfIconDraw,
    cfIconSuppress: iconState.cfIconSuppress,
    hyperlinkDxfs,
  };
  inner.set(layout, value);
  return value;
}

function visibleEnvelope(panes: Pane[]): Visible {
  let firstRow = Infinity;
  let lastRow = 0;
  let firstCol = Infinity;
  let lastCol = 0;
  for (const pane of panes) {
    firstRow = Math.min(firstRow, pane.vis.firstRow);
    lastRow = Math.max(lastRow, pane.vis.lastRow);
    firstCol = Math.min(firstCol, pane.vis.firstCol);
    lastCol = Math.max(lastCol, pane.vis.lastCol);
  }
  return {
    firstRow: Number.isFinite(firstRow) ? firstRow : 1,
    lastRow: Math.max(lastRow, 1),
    firstCol: Number.isFinite(firstCol) ? firstCol : 1,
    lastCol: Math.max(lastCol, 1),
  };
}
