import type { Sheet, WorkbookLayout } from "./types.js";
import { drawingHyperlinkAt, drawingIndexAtPoint } from "./drawingHits.js";
import { cellRect } from "./geometry.js";
import {
  filterArrowRect,
  pivotFilterArrows,
  tableFilterArrows,
  validationArrowRect,
} from "./sheetChrome.js";
import type { PivotArrowHit } from "./sheetChrome.js";
import type { TableFilterArrow } from "./schema/TableFilterArrow.js";
import { buildGrid, frozenDims } from "./render.js";
import { anchorToRect, rectToAnchor } from "./grid.js";
import { drawingHandleAtPoint, drawingHandleCursor, resizeRect } from "./drawingSelection.js";
import type { DrawingAnchor } from "./schema/DrawingAnchor.js";
import { cellA1, rangeA1 } from "./api-refs.js";
import { createAnnotationLayer } from "./interactAnnotations.js";
import {
  computeOutlineRuns,
  outlineButtonHits,
  outlineCornerHits,
  type OutlineRun,
  OUTLINE_BUTTON_HIT_RADIUS,
} from "./outlineGutter.js";

const RESIZE_TOL = 4;

function anchorChanged(a: DrawingAnchor, b: DrawingAnchor): boolean {
  return (
    a.fromCol !== b.fromCol ||
    a.fromRow !== b.fromRow ||
    a.toCol !== b.toCol ||
    a.toRow !== b.toRow ||
    a.fromColOffEmu !== b.fromColOffEmu ||
    a.fromRowOffEmu !== b.fromRowOffEmu ||
    a.toColOffEmu !== b.toColOffEmu ||
    a.toRowOffEmu !== b.toRowOffEmu ||
    a.extEmuCx !== b.extEmuCx ||
    a.extEmuCy !== b.extEmuCy
  );
}
const MIN_COL_W = 8;
const MIN_ROW_H = 4;
const ZOOM_MIN = 0.25;
const ZOOM_MAX = 4;

export interface InteractHandle {
  destroy(): void;
}

export interface InteractOptions {
  getSheet(): Sheet;
  getLayout(): WorkbookLayout;

  zoom: { get(): number; set(value: number): void };

  colOverrides: Map<number, number>;

  rowOverrides: Map<number, number>;

  activeCell: {
    get(): { r: number; c: number } | null;
    set(v: { r: number; c: number } | null): void;
  };

  selection?: { get(): Selection | null; set(v: Selection | null): void };

  selectedDrawing?: { get(): number | null; set(v: number | null): void };

  scrollContainer?: HTMLElement;

  getViewport?: () => { x: number; y: number; w: number; h: number } | null;

  onPivotFilter?: (info: PivotFilterEvent) => void;

  onTableFilter?: (info: TableFilterEvent) => void;

  onValidationPick?: (info: ValidationPickEvent) => void;

  onEditStart?: (cell: { r: number; c: number }, initialText: string | null) => void;

  onCopy?: (selection: Selection, isCut: boolean) => void;
  onPaste?: (target: { r: number; c: number }) => void;

  onFill?: (source: Selection, target: Selection) => void;

  onClear?: (selection: Selection) => void;

  isPointModeActive?: () => boolean;

  onDrawingMoved?: (info: {
    index: number;
    prevAnchor: DrawingAnchor;
    anchor: DrawingAnchor;
  }) => void;

  onDrawingDelete?: (info: { index: number }) => void;

  onPointModeRef?: (rangeRef: string, opts: { extend: boolean }) => void;

  redraw(): void;
}

export interface PivotFilterEvent {
  pivot: string;
  field: string;
  axis: "row" | "column";
  rect: { left: number; top: number; right: number; bottom: number };
}

export interface TableFilterEvent {
  field: string;
  columnOffset: number;
  rangeRef: string;
  rect: { left: number; top: number; right: number; bottom: number };
}

export interface ValidationPickEvent {
  r: number;
  c: number;
  options: string[];
  rect: { left: number; top: number; right: number; bottom: number };
}

interface ValidationArrowHit {
  r: number;
  c: number;
  options: string[];
}

export interface Selection {
  r1: number;
  c1: number;
  r2: number;
  c2: number;
}

interface HitCol {
  kind: "col";
  index: number;
  edgeX: number;
}
interface HitRow {
  kind: "row";
  index: number;
  edgeY: number;
}
type Hit = HitCol | HitRow | null;

export function attachInteractivity(
  canvas: HTMLCanvasElement,
  opts: InteractOptions,
): InteractHandle {
  let drag: { hit: HitCol | HitRow; startPx: number; original: number } | null = null;

  let selDrag: {
    kind: "cell" | "col" | "row";
    anchor: { r: number; c: number };
  } | null = null;
  let pointDrag: { anchor: { r: number; c: number } } | null = null;
  let drawDrag: {
    index: number;
    startX: number;
    startY: number;
    startRect: { x: number; y: number; w: number; h: number };
    prevAnchor: DrawingAnchor;
  } | null = null;
  let resizeDrag: {
    index: number;
    handle: number;
    startX: number;
    startY: number;
    startRect: { x: number; y: number; w: number; h: number };
    prevAnchor: DrawingAnchor;
  } | null = null;
  let fillDrag: { source: Selection } | null = null;
  let pointAnchor: { r: number; c: number } | null = null;
  const savedCursor = canvas.style.cursor;
  let cachedGrid: {
    sheet: Sheet;
    colOverrides: Map<number, number>;
    rowOverrides: Map<number, number>;
    grid: ReturnType<typeof buildGrid>;
  } | null = null;

  function invalidateGrid() {
    cachedGrid = null;
  }

  function getGrid(): ReturnType<typeof buildGrid> {
    const sheet = opts.getSheet();
    if (
      cachedGrid &&
      cachedGrid.sheet === sheet &&
      cachedGrid.colOverrides === opts.colOverrides &&
      cachedGrid.rowOverrides === opts.rowOverrides
    ) {
      return cachedGrid.grid;
    }
    const grid = buildGrid(sheet, opts.colOverrides, opts.rowOverrides);
    cachedGrid = { sheet, colOverrides: opts.colOverrides, rowOverrides: opts.rowOverrides, grid };
    return grid;
  }

  const annotations = createAnnotationLayer(canvas, opts.getSheet);

  function resolveAnchor(r: number, c: number): { r: number; c: number } {
    const sheet = opts.getSheet();
    for (const m of sheet.merges) {
      if (r >= m.r1 && r <= m.r2 && c >= m.c1 && c <= m.c2) return { r: m.r1, c: m.c1 };
    }
    return { r, c };
  }

  function cellAtLogical(p: { x: number; y: number }): { r: number; c: number } | null {
    const grid = getGrid();
    if (p.x < grid.originX || p.y < grid.originY) return null;
    return cellAt(grid, p.x, p.y);
  }

  function toCanvasLocal(ev: { clientX: number; clientY: number }): { x: number; y: number } {
    const r = canvas.getBoundingClientRect();
    const z = opts.zoom.get();
    return {
      x: (ev.clientX - r.left) / z,
      y: (ev.clientY - r.top) / z,
    };
  }

  function toLogical(ev: { clientX: number; clientY: number }): { x: number; y: number } {
    const p = toCanvasLocal(ev);
    const vp = opts.getViewport?.() ?? null;
    const sheet = opts.getSheet();
    const grid = getGrid();
    const { pcw, prh } = frozenDims(sheet, grid);

    const sx = vp && p.x > grid.originX + pcw ? vp.x : 0;
    const sy = vp && p.y > grid.originY + prh ? vp.y : 0;
    return { x: p.x + sx, y: p.y + sy };
  }

  function hitTest(cx: number, cy: number): Hit {
    const vp = opts.getViewport?.() ?? null;
    const sx = vp?.x ?? 0;
    const sy = vp?.y ?? 0;
    const sheet = opts.getSheet();
    const grid = getGrid();
    const { splitX, splitY, pcw, prh } = frozenDims(sheet, grid);

    if (cy >= grid.colGutterH && cy <= grid.originY && cx > grid.originX) {
      if (cx <= grid.originX + pcw) {
        const edgeIndex = nearestEdgeIndex(grid.colX, cx, 2, splitX);
        if (edgeIndex !== null) {
          return { kind: "col", index: edgeIndex - 1, edgeX: grid.colX[edgeIndex] ?? 0 };
        }
      } else {
        const x = cx + sx;
        const edgeIndex = nearestEdgeIndex(grid.colX, x, Math.max(splitX + 1, 2), grid.maxCol + 1);
        if (edgeIndex !== null) {
          return { kind: "col", index: edgeIndex - 1, edgeX: grid.colX[edgeIndex] ?? 0 };
        }
      }
    }
    if (cx >= grid.rowGutterW && cx <= grid.originX && cy > grid.originY) {
      if (cy <= grid.originY + prh) {
        const edgeIndex = nearestEdgeIndex(grid.rowY, cy, 2, splitY);
        if (edgeIndex !== null) {
          return { kind: "row", index: edgeIndex - 1, edgeY: grid.rowY[edgeIndex] ?? 0 };
        }
      } else {
        const y = cy + sy;
        const edgeIndex = nearestEdgeIndex(grid.rowY, y, Math.max(splitY + 1, 2), grid.maxRow + 1);
        if (edgeIndex !== null) {
          return { kind: "row", index: edgeIndex - 1, edgeY: grid.rowY[edgeIndex] ?? 0 };
        }
      }
    }
    return null;
  }

  function pivotArrowAt(lp: { x: number; y: number }): PivotArrowHit | null {
    const sheet = opts.getSheet();
    const arrows = pivotFilterArrows(sheet);
    if (arrows.length === 0) return null;
    const grid = getGrid();
    for (const a of arrows) {
      const box = filterArrowRect(cellRect(grid, a.r, a.c));
      if (lp.x >= box.x && lp.x <= box.x + box.w && lp.y >= box.y && lp.y <= box.y + box.h) {
        return a;
      }
    }
    return null;
  }

  function firePivotFilter(a: PivotArrowHit) {
    if (!opts.onPivotFilter) return;
    const grid = getGrid();
    const sheet = opts.getSheet();
    const box = filterArrowRect(cellRect(grid, a.r, a.c));
    const z = opts.zoom.get();
    const r = canvas.getBoundingClientRect();
    const vp = opts.getViewport?.() ?? null;
    const { splitX, splitY } = frozenDims(sheet, grid);
    const sx = vp && a.c >= splitX ? vp.x : 0;
    const sy = vp && a.r >= splitY ? vp.y : 0;
    const left = r.left + (box.x - sx) * z;
    const top = r.top + (box.y - sy) * z;
    opts.onPivotFilter({
      pivot: a.pivot,
      field: a.field,
      axis: a.axis,
      rect: { left, top, right: left + box.w * z, bottom: top + box.h * z },
    });
  }

  function tableArrowAt(lp: { x: number; y: number }): TableFilterArrow | null {
    const sheet = opts.getSheet();
    const arrows = tableFilterArrows(sheet);
    if (arrows.length === 0) return null;
    const grid = getGrid();
    for (const a of arrows) {
      const box = filterArrowRect(cellRect(grid, a.r, a.c));
      if (lp.x >= box.x && lp.x <= box.x + box.w && lp.y >= box.y && lp.y <= box.y + box.h) {
        return a;
      }
    }
    return null;
  }

  function fireTableFilter(a: TableFilterArrow) {
    if (!opts.onTableFilter) return;
    const grid = getGrid();
    const sheet = opts.getSheet();
    const box = filterArrowRect(cellRect(grid, a.r, a.c));
    const z = opts.zoom.get();
    const r = canvas.getBoundingClientRect();
    const vp = opts.getViewport?.() ?? null;
    const { splitX, splitY } = frozenDims(sheet, grid);
    const sx = vp && a.c >= splitX ? vp.x : 0;
    const sy = vp && a.r >= splitY ? vp.y : 0;
    const left = r.left + (box.x - sx) * z;
    const top = r.top + (box.y - sy) * z;
    opts.onTableFilter({
      field: a.columnName,
      columnOffset: a.columnOffset,
      rangeRef: a.rangeRef,
      rect: { left, top, right: left + box.w * z, bottom: top + box.h * z },
    });
  }

  function validationArrowAt(lp: { x: number; y: number }): ValidationArrowHit | null {
    const sheet = opts.getSheet();
    const dropdowns = sheet.validationDropdowns ?? [];
    if (dropdowns.length === 0) return null;
    const active = opts.activeCell.get();
    if (!active) return null;
    const d = dropdowns.find((dd) => dd.r === active.r && dd.c === active.c);
    if (!d) return null;
    const lists = sheet.validationLists ?? [];
    const grid = getGrid();
    const box = validationArrowRect(cellRect(grid, d.r, d.c));
    if (lp.x >= box.x && lp.x <= box.x + box.w && lp.y >= box.y && lp.y <= box.y + box.h) {
      return { r: d.r, c: d.c, options: lists[d.list] ?? [] };
    }
    return null;
  }

  function fireValidationPick(a: ValidationArrowHit) {
    if (!opts.onValidationPick) return;
    const grid = getGrid();
    const sheet = opts.getSheet();
    const cell = cellRect(grid, a.r, a.c);
    const z = opts.zoom.get();
    const r = canvas.getBoundingClientRect();
    const vp = opts.getViewport?.() ?? null;
    const { splitX, splitY } = frozenDims(sheet, grid);
    const sx = vp && a.c >= splitX ? vp.x : 0;
    const sy = vp && a.r >= splitY ? vp.y : 0;
    const left = r.left + (cell.x - sx) * z;
    const top = r.top + (cell.y - sy) * z;
    opts.onValidationPick({
      r: a.r,
      c: a.c,
      options: a.options,
      rect: { left, top, right: left + cell.w * z, bottom: top + cell.h * z },
    });
  }

  function maybeOutlineCursor(cp: { x: number; y: number }): boolean {
    if (outlineButtonAt(cp) || outlineCornerAt(cp)) {
      canvas.style.cursor = "pointer";
      annotations.hidePopover();
      return true;
    }
    return false;
  }

  function onPointerMove(ev: PointerEvent) {
    if (resizeDrag) {
      const grid = getGrid();
      const p = toLogical(ev);
      const sheet = opts.getSheet();
      const d = sheet.drawings[resizeDrag.index];
      if (d) {
        const r = resizeRect(
          resizeDrag.startRect,
          resizeDrag.handle,
          p.x - resizeDrag.startX,
          p.y - resizeDrag.startY,
        );
        d.anchor = rectToAnchor(r, grid, resizeDrag.prevAnchor);
      }
      invalidateGrid();
      opts.redraw();
      return;
    }
    if (drawDrag) {
      const grid = getGrid();
      const p = toLogical(ev);
      const { startRect } = drawDrag;
      const x = Math.max(grid.originX, startRect.x + (p.x - drawDrag.startX));
      const y = Math.max(grid.originY, startRect.y + (p.y - drawDrag.startY));
      const sheet = opts.getSheet();
      const d = sheet.drawings[drawDrag.index];
      if (d) {
        d.anchor = rectToAnchor(
          { x, y, w: startRect.w, h: startRect.h },
          grid,
          drawDrag.prevAnchor,
        );
      }
      invalidateGrid();
      opts.redraw();
      return;
    }
    if (drag) {
      const p = toLogical(ev);
      if (drag.hit.kind === "col") {
        const delta = p.x - drag.startPx;
        const next = Math.max(MIN_COL_W, drag.original + delta);
        opts.colOverrides.set(drag.hit.index, next);
      } else {
        const delta = p.y - drag.startPx;
        const next = Math.max(MIN_ROW_H, drag.original + delta);
        opts.rowOverrides.set(drag.hit.index, next);
      }
      invalidateGrid();
      opts.redraw();
      return;
    }
    if (pointDrag) {
      const grid = getGrid();
      const lp = toLogical(ev);
      const cx = Math.max(grid.originX + 0.5, lp.x);
      const cy = Math.max(grid.originY + 0.5, lp.y);
      const cell = cellAt(grid, cx, cy);
      if (!cell) return;
      const cur = expandThroughMerge(cell.r, cell.c);
      opts.onPointModeRef?.(pointRef(pointDrag.anchor, { r: cur.r2, c: cur.c2 }), {
        extend: false,
      });
      return;
    }
    if (fillDrag) {
      const grid = getGrid();
      const lp = toLogical(ev);
      const cx = Math.max(grid.originX + 0.5, lp.x);
      const cy = Math.max(grid.originY + 0.5, lp.y);
      const cell = cellAt(grid, cx, cy);
      if (!cell) return;
      const s = fillDrag.source;
      const rowExtend = cell.r - s.r2;
      const colExtend = cell.c - s.c2;
      const target =
        rowExtend >= colExtend
          ? { r1: s.r1, c1: s.c1, r2: Math.max(s.r2, cell.r), c2: s.c2 }
          : { r1: s.r1, c1: s.c1, r2: s.r2, c2: Math.max(s.c2, cell.c) };
      opts.selection?.set(target);
      opts.redraw();
      return;
    }
    if (selDrag) {
      const grid = getGrid();
      const lp = toLogical(ev);

      const cx = Math.max(grid.originX + 0.5, lp.x);
      const cy = Math.max(grid.originY + 0.5, lp.y);
      const cell = cellAt(grid, cx, cy);
      if (!cell) return;
      const a = selDrag.anchor;
      const cur = expandThroughMerge(cell.r, cell.c);
      const anc = expandThroughMerge(a.r, a.c);
      let r1 = Math.min(anc.r1, cur.r1);
      let r2 = Math.max(anc.r2, cur.r2);
      let c1 = Math.min(anc.c1, cur.c1);
      let c2 = Math.max(anc.c2, cur.c2);
      if (selDrag.kind === "col") {
        r1 = 1;
        r2 = grid.maxRow;
      } else if (selDrag.kind === "row") {
        c1 = 1;
        c2 = grid.maxCol;
      }
      setSelection(a, { r1, c1, r2, c2 });
      opts.redraw();
      return;
    }
    const cp = toCanvasLocal(ev);
    if (cp.x >= 0 && fillHandleAt(toLogical(ev))) {
      canvas.style.cursor = "crosshair";
      annotations.hidePopover();
      return;
    }
    if (maybeOutlineCursor(cp)) return;
    if (opts.onPivotFilter) {
      const lp = toLogical(ev);
      if (pivotArrowAt(lp)) {
        canvas.style.cursor = "pointer";
        annotations.hidePopover();
        return;
      }
    }
    if (opts.onTableFilter) {
      const lp = toLogical(ev);
      if (tableArrowAt(lp)) {
        canvas.style.cursor = "pointer";
        annotations.hidePopover();
        return;
      }
    }
    if (opts.onValidationPick) {
      const lp = toLogical(ev);
      if (validationArrowAt(lp)) {
        canvas.style.cursor = "pointer";
        annotations.hidePopover();
        return;
      }
    }
    const hit = hitTest(cp.x, cp.y);
    if (hit) {
      canvas.style.cursor = hit.kind === "col" ? "col-resize" : "row-resize";
      annotations.hidePopover();
      return;
    }

    const sel = opts.selectedDrawing?.get();
    if (sel != null) {
      const grid = getGrid();
      const lp2 = toLogical(ev);
      const selDrawing = opts.getSheet().drawings?.[sel];
      const rect = selDrawing ? anchorToRect(selDrawing, grid) : null;
      if (rect) {
        const hi = drawingHandleAtPoint(rect, lp2.x, lp2.y);
        if (hi != null) {
          canvas.style.cursor = drawingHandleCursor(hi);
          annotations.hidePopover();
          return;
        }
      }
      if (
        lp2.x >= grid.originX &&
        lp2.y >= grid.originY &&
        drawingIndexAtPoint(opts.getSheet(), grid, lp2.x, lp2.y) === sel
      ) {
        canvas.style.cursor = "move";
        annotations.hidePopover();
        return;
      }
    }

    annotations.ensureMaps();
    const lp = toLogical(ev);
    const cell = cellAtLogical(lp);
    if (!cell) {
      canvas.style.cursor = savedCursor;
      annotations.hidePopover();
      return;
    }
    const anchor = resolveAnchor(cell.r, cell.c);
    const grid = getGrid();
    const link =
      drawingHyperlinkAt(opts.getSheet(), grid, lp.x, lp.y) ?? annotations.hyperlinkAt(anchor);
    const cmt = annotations.commentAt(anchor);

    canvas.style.cursor = link ? "pointer" : savedCursor;

    if (cmt) {
      const grid = getGrid();
      const z = opts.zoom.get();
      const r = canvas.getBoundingClientRect();
      const vp = opts.getViewport?.() ?? null;
      const { splitX, splitY, pcw, prh } = frozenDims(opts.getSheet(), grid);
      const cx = grid.colX[anchor.c] ?? 0;
      const cy = grid.rowY[anchor.r] ?? 0;
      const cw = grid.colW[anchor.c] ?? 0;
      const sx = vp && anchor.c >= splitX ? vp.x : 0;
      const sy = vp && anchor.r >= splitY ? vp.y : 0;
      const left = r.left + (cx - sx) * z;
      const top = r.top + (cy - sy) * z;
      const right = left + cw * z;

      void splitX;
      void splitY;
      void pcw;
      void prh;
      annotations.showPopover(cmt, { left, top, right });
    } else {
      annotations.hidePopover();
    }
  }

  function normalizeSel(s: Selection): Selection {
    return {
      r1: Math.min(s.r1, s.r2),
      c1: Math.min(s.c1, s.c2),
      r2: Math.max(s.r1, s.r2),
      c2: Math.max(s.c1, s.c2),
    };
  }

  function fillHandleAt(lp: { x: number; y: number }): Selection | null {
    if (!opts.onFill) return null;
    const sel = opts.selection?.get();
    if (!sel) return null;
    const n = normalizeSel(sel);
    const grid = getGrid();
    const x2 = grid.colX[n.c2 + 1] ?? 0;
    const y2 = grid.rowY[n.r2 + 1] ?? 0;
    if (x2 === 0 || y2 === 0) return null;
    if (Math.abs(lp.x - x2) <= 5 && Math.abs(lp.y - y2) <= 5) return n;
    return null;
  }

  function pointRef(anchor: { r: number; c: number }, cur: { r: number; c: number }): string {
    const r1 = Math.min(anchor.r, cur.r);
    const c1 = Math.min(anchor.c, cur.c);
    const r2 = Math.max(anchor.r, cur.r);
    const c2 = Math.max(anchor.c, cur.c);
    if (r1 === r2 && c1 === c2) return cellA1(r1, c1);
    return rangeA1(r1, c1, r2 - r1 + 1, c2 - c1 + 1);
  }

  function setSelection(active: { r: number; c: number }, range: Selection) {
    opts.selectedDrawing?.set(null);
    opts.activeCell.set(active);
    opts.selection?.set(range);
  }

  function expandThroughMerge(
    r: number,
    c: number,
  ): {
    r1: number;
    c1: number;
    r2: number;
    c2: number;
  } {
    const sheet = opts.getSheet();
    for (const m of sheet.merges) {
      if (r >= m.r1 && r <= m.r2 && c >= m.c1 && c <= m.c2) {
        return { r1: m.r1, c1: m.c1, r2: m.r2, c2: m.c2 };
      }
    }
    return { r1: r, c1: c, r2: r, c2: c };
  }

  function outlineButtonAt(cp: { x: number; y: number }): {
    run: OutlineRun;
    collapsed: boolean;
  } | null {
    const sheet = opts.getSheet();
    const grid = getGrid();
    if (grid.rowGutterW === 0 && grid.colGutterH === 0) return null;
    const vp = opts.getViewport?.() ?? null;
    const { splitX, splitY, pcw, prh } = frozenDims(sheet, grid);
    const view = {
      sx: vp?.x ?? 0,
      sy: vp?.y ?? 0,
      splitX,
      splitY,
      pcw,
      prh,

      canvasW: canvas.clientWidth || canvas.width,
      canvasH: canvas.clientHeight || canvas.height,
    };
    const hits = outlineButtonHits(sheet, grid, view);
    let best: (typeof hits)[number] | null = null;
    let bestD = Infinity;
    for (const h of hits) {
      const d = Math.max(Math.abs(cp.x - h.cx), Math.abs(cp.y - h.cy));
      if (d <= OUTLINE_BUTTON_HIT_RADIUS && d < bestD) {
        best = h;
        bestD = d;
      }
    }
    return best ? { run: best.run, collapsed: best.collapsed } : null;
  }

  function outlineCornerAt(cp: {
    x: number;
    y: number;
  }): { axis: "row" | "col"; level: number } | null {
    const grid = getGrid();
    if (grid.rowGutterW === 0 && grid.colGutterH === 0) return null;

    if (cp.x > Math.max(grid.rowGutterW, grid.originX)) return null;
    if (cp.y > Math.max(grid.colGutterH, grid.originY)) return null;
    const hits = outlineCornerHits(grid);
    let best: (typeof hits)[number] | null = null;
    let bestD = Infinity;
    for (const h of hits) {
      const d = Math.max(Math.abs(cp.x - h.cx), Math.abs(cp.y - h.cy));
      if (d <= OUTLINE_BUTTON_HIT_RADIUS && d < bestD) {
        best = h;
        bestD = d;
      }
    }
    return best ? { axis: best.axis, level: best.level } : null;
  }

  function naturalRowHeight(sheet: Sheet, r: number): number {
    const meta = sheet.decodedRowMeta;
    if (meta) {
      for (let i = 0; i < meta.count; i++) {
        if (meta.index[i] === r) {
          const h = meta.heightPx[i];
          if (h !== undefined && !Number.isNaN(h)) return h;
          break;
        }
      }
    }
    return sheet.defaultRowHeightPx;
  }

  function naturalColWidth(sheet: Sheet, c: number): number {
    for (const col of sheet.cols) {
      if (c >= col.min && c <= col.max) return col.widthPx;
    }
    return sheet.defaultColWidthPx;
  }

  function setRunCollapsed(run: OutlineRun, collapsed: boolean) {
    const sheet = opts.getSheet();
    if (run.axis === "row") {
      for (let r = run.start; r <= run.end; r++) {
        if (collapsed) opts.rowOverrides.set(r, 0);
        else opts.rowOverrides.set(r, Math.max(1, naturalRowHeight(sheet, r)));
      }
    } else {
      for (let c = run.start; c <= run.end; c++) {
        if (collapsed) opts.colOverrides.set(c, 0);
        else opts.colOverrides.set(c, Math.max(1, naturalColWidth(sheet, c)));
      }
    }
  }

  function applyCornerCollapse(target: { axis: "row" | "col"; level: number }) {
    const sheet = opts.getSheet();
    const grid = getGrid();
    const runs = computeOutlineRuns(sheet, grid);
    for (const run of runs) {
      if (run.axis !== target.axis) continue;
      const shouldCollapse = run.level >= target.level;
      setRunCollapsed(run, shouldCollapse);
    }
    invalidateGrid();
    opts.redraw();
  }

  function onPointerDown(ev: PointerEvent) {
    if (ev.button !== 0) return;
    const cp = toCanvasLocal(ev);
    const p = toLogical(ev);
    const shift = ev.shiftKey;

    if (opts.onPivotFilter) {
      const arrow = pivotArrowAt(p);
      if (arrow) {
        ev.preventDefault();
        firePivotFilter(arrow);
        canvas.focus({ preventScroll: true });
        return;
      }
    }

    if (opts.onTableFilter) {
      const arrow = tableArrowAt(p);
      if (arrow) {
        ev.preventDefault();
        fireTableFilter(arrow);
        canvas.focus({ preventScroll: true });
        return;
      }
    }

    if (opts.onValidationPick) {
      const arrow = validationArrowAt(p);
      if (arrow) {
        ev.preventDefault();
        fireValidationPick(arrow);
        canvas.focus({ preventScroll: true });
        return;
      }
    }

    const ob = outlineButtonAt(cp);
    if (ob) {
      ev.preventDefault();
      setRunCollapsed(ob.run, !ob.collapsed);
      invalidateGrid();
      opts.redraw();
      canvas.focus({ preventScroll: true });
      return;
    }
    const oc = outlineCornerAt(cp);
    if (oc) {
      ev.preventDefault();
      applyCornerCollapse(oc);
      canvas.focus({ preventScroll: true });
      return;
    }

    const hit = hitTest(cp.x, cp.y);
    if (hit) {
      ev.preventDefault();
      canvas.setPointerCapture(ev.pointerId);
      const grid = getGrid();
      if (hit.kind === "col") {
        drag = { hit, startPx: p.x, original: grid.colW[hit.index] ?? 0 };
      } else {
        drag = { hit, startPx: p.y, original: grid.rowH[hit.index] ?? 0 };
      }
      return;
    }

    const grid = getGrid();

    const inColHeader = cp.y >= grid.colGutterH && cp.y < grid.originY;
    const inRowHeader = cp.x >= grid.rowGutterW && cp.x < grid.originX;

    if (inColHeader && inRowHeader) {
      ev.preventDefault();
      setSelection({ r: 1, c: 1 }, { r1: 1, c1: 1, r2: grid.maxRow, c2: grid.maxCol });
      opts.redraw();
      canvas.focus({ preventScroll: true });
      return;
    }

    if (inColHeader && cp.x >= grid.originX) {
      const cell = cellAt(grid, p.x, grid.originY + 1);
      if (cell) {
        ev.preventDefault();
        canvas.setPointerCapture(ev.pointerId);
        const cur = opts.activeCell.get();
        if (shift && cur) {
          const c1 = Math.min(cur.c, cell.c);
          const c2 = Math.max(cur.c, cell.c);
          selDrag = { kind: "col", anchor: { r: 1, c: cur.c } };
          setSelection({ r: 1, c: cur.c }, { r1: 1, c1, r2: grid.maxRow, c2 });
        } else {
          selDrag = { kind: "col", anchor: { r: 1, c: cell.c } };
          setSelection({ r: 1, c: cell.c }, { r1: 1, c1: cell.c, r2: grid.maxRow, c2: cell.c });
        }
        opts.redraw();
        canvas.focus({ preventScroll: true });
      }
      return;
    }

    if (inRowHeader && cp.y >= grid.originY) {
      const cell = cellAt(grid, grid.originX + 1, p.y);
      if (cell) {
        ev.preventDefault();
        canvas.setPointerCapture(ev.pointerId);
        const cur = opts.activeCell.get();
        if (shift && cur) {
          const r1 = Math.min(cur.r, cell.r);
          const r2 = Math.max(cur.r, cell.r);
          selDrag = { kind: "row", anchor: { r: cur.r, c: 1 } };
          setSelection({ r: cur.r, c: 1 }, { r1, c1: 1, r2, c2: grid.maxCol });
        } else {
          selDrag = { kind: "row", anchor: { r: cell.r, c: 1 } };
          setSelection({ r: cell.r, c: 1 }, { r1: cell.r, c1: 1, r2: cell.r, c2: grid.maxCol });
        }
        opts.redraw();
        canvas.focus({ preventScroll: true });
      }
      return;
    }

    if (cp.x >= grid.originX && cp.y >= grid.originY && opts.selectedDrawing) {
      const selIdx = opts.selectedDrawing.get();
      if (selIdx != null) {
        const selD = opts.getSheet().drawings[selIdx];
        const selRect = selD ? anchorToRect(selD, grid) : null;
        const handle = selRect ? drawingHandleAtPoint(selRect, p.x, p.y) : null;
        if (selD && selRect && handle !== null) {
          ev.preventDefault();
          canvas.setPointerCapture(ev.pointerId);
          resizeDrag = {
            index: selIdx,
            handle,
            startX: p.x,
            startY: p.y,
            startRect: selRect,
            prevAnchor: { ...selD.anchor },
          };
          canvas.focus({ preventScroll: true });
          return;
        }
      }
      const di = drawingIndexAtPoint(opts.getSheet(), grid, p.x, p.y);
      if (di !== null) {
        ev.preventDefault();
        const d = opts.getSheet().drawings[di];
        const rect = d ? anchorToRect(d, grid) : null;
        if (opts.selectedDrawing.get() !== di) {
          opts.selectedDrawing.set(di);
          opts.selection?.set(null);
          opts.redraw();
        }
        if (d && rect) {
          canvas.setPointerCapture(ev.pointerId);
          drawDrag = {
            index: di,
            startX: p.x,
            startY: p.y,
            startRect: rect,
            prevAnchor: { ...d.anchor },
          };
        }
        canvas.focus({ preventScroll: true });
        return;
      }
    }

    if (cp.x >= grid.originX && cp.y >= grid.originY) {
      const fillSource = fillHandleAt(p);
      if (fillSource) {
        ev.preventDefault();
        canvas.setPointerCapture(ev.pointerId);
        fillDrag = { source: fillSource };
        canvas.focus({ preventScroll: true });
        return;
      }
      const cell = cellAt(grid, p.x, p.y);
      if (cell && opts.isPointModeActive?.()) {
        ev.preventDefault();
        canvas.setPointerCapture(ev.pointerId);
        const tgt = expandThroughMerge(cell.r, cell.c);
        const anchor = shift && pointAnchor ? pointAnchor : { r: tgt.r1, c: tgt.c1 };
        if (!shift) pointAnchor = anchor;
        pointDrag = { anchor };
        opts.onPointModeRef?.(pointRef(anchor, { r: tgt.r2, c: tgt.c2 }), { extend: shift });
        return;
      }
      if (cell) {
        ev.preventDefault();
        canvas.setPointerCapture(ev.pointerId);
        const cur = opts.activeCell.get();
        if (shift && cur) {
          const anc = expandThroughMerge(cur.r, cur.c);
          const tgt = expandThroughMerge(cell.r, cell.c);
          const r1 = Math.min(anc.r1, tgt.r1);
          const r2 = Math.max(anc.r2, tgt.r2);
          const c1 = Math.min(anc.c1, tgt.c1);
          const c2 = Math.max(anc.c2, tgt.c2);
          selDrag = { kind: "cell", anchor: cur };
          setSelection(cur, { r1, c1, r2, c2 });
          opts.redraw();
          canvas.focus({ preventScroll: true });
          return;
        }

        const sheet = opts.getSheet();
        let anchor = cell;
        for (const m of sheet.merges) {
          if (cell.r >= m.r1 && cell.r <= m.r2 && cell.c >= m.c1 && cell.c <= m.c2) {
            anchor = { r: m.r1, c: m.c1 };
            setSelection({ r: m.r1, c: m.c1 }, { r1: m.r1, c1: m.c1, r2: m.r2, c2: m.c2 });
            break;
          }
        }
        if (anchor === cell) {
          setSelection(cell, { r1: cell.r, c1: cell.c, r2: cell.r, c2: cell.c });
        }

        selDrag = { kind: "cell", anchor };
        opts.redraw();
        canvas.focus({ preventScroll: true });

        annotations.ensureMaps();
        const grid = getGrid();
        const link = drawingHyperlinkAt(sheet, grid, p.x, p.y) ?? annotations.hyperlinkAt(anchor);
        if (link) {
          annotations.openHyperlink(link);
          return;
        }
      }
    }
  }

  function cellAt(
    grid: ReturnType<typeof buildGrid>,
    x: number,
    y: number,
  ): { r: number; c: number } | null {
    const c = edgeOwnerIndex(grid.colX, x, 1, grid.maxCol);
    const r = edgeOwnerIndex(grid.rowY, y, 1, grid.maxRow);
    if (r === null || c === null) return null;
    return { r, c };
  }

  function edgeOwnerIndex(
    edges: number[],
    px: number,
    minIndex: number,
    maxIndex: number,
  ): number | null {
    if (maxIndex < minIndex) return null;
    if (px < (edges[minIndex] ?? 0) || px >= (edges[maxIndex + 1] ?? 0)) return null;
    let lo = minIndex + 1;
    let hi = maxIndex + 1;
    while (lo < hi) {
      const mid = (lo + hi) >> 1;
      if ((edges[mid] ?? 0) <= px) lo = mid + 1;
      else hi = mid;
    }
    return lo - 1;
  }

  function nearestEdgeIndex(
    edges: number[],
    px: number,
    minEdgeIndex: number,
    maxEdgeIndex: number,
  ): number | null {
    if (maxEdgeIndex < minEdgeIndex) return null;
    let lo = minEdgeIndex;
    let hi = maxEdgeIndex + 1;
    while (lo < hi) {
      const mid = (lo + hi) >> 1;
      if ((edges[mid] ?? 0) < px) lo = mid + 1;
      else hi = mid;
    }

    let best: number | null = null;
    let bestDist = Infinity;
    for (const i of [lo - 1, lo]) {
      if (i < minEdgeIndex || i > maxEdgeIndex) continue;
      const dist = Math.abs(px - (edges[i] ?? 0));
      if (dist < bestDist) {
        best = i;
        bestDist = dist;
      }
    }
    return bestDist <= RESIZE_TOL ? best : null;
  }

  function ensureVisible(cell: { r: number; c: number }) {
    const sc = opts.scrollContainer;
    if (!sc) return;
    const sheet = opts.getSheet();
    const grid = getGrid();
    const z = opts.zoom.get();
    const { splitX, splitY, pcw, prh } = frozenDims(sheet, grid);
    const x = (grid.colX[cell.c] ?? 0) * z;
    const y = (grid.rowY[cell.r] ?? 0) * z;
    const w = (grid.colW[cell.c] ?? 0) * z;
    const h = (grid.rowH[cell.r] ?? 0) * z;

    const padX = (grid.originX + pcw) * z;
    const padY = (grid.originY + prh) * z;
    if (cell.c >= splitX) {
      if (x < sc.scrollLeft + padX) sc.scrollLeft = x - padX;
      else if (x + w > sc.scrollLeft + sc.clientWidth) sc.scrollLeft = x + w - sc.clientWidth;
    }
    if (cell.r >= splitY) {
      if (y < sc.scrollTop + padY) sc.scrollTop = y - padY;
      else if (y + h > sc.scrollTop + sc.clientHeight) sc.scrollTop = y + h - sc.clientHeight;
    }
  }

  function onKeyDown(ev: KeyboardEvent) {
    if (ev.key === "Escape" && opts.selectedDrawing?.get() != null) {
      ev.preventDefault();
      opts.selectedDrawing.set(null);
      opts.redraw();
      return;
    }
    const selDi = opts.selectedDrawing?.get();
    if (selDi != null) {
      if (ev.key === "Delete" || ev.key === "Backspace") {
        ev.preventDefault();
        opts.onDrawingDelete?.({ index: selDi });
        opts.selectedDrawing?.set(null);
        return;
      }
      let ndx = 0,
        ndy = 0;
      switch (ev.key) {
        case "ArrowUp":
          ndy = -1;
          break;
        case "ArrowDown":
          ndy = 1;
          break;
        case "ArrowLeft":
          ndx = -1;
          break;
        case "ArrowRight":
          ndx = 1;
          break;
      }
      if (ndx !== 0 || ndy !== 0) {
        ev.preventDefault();
        const step = ev.shiftKey ? 10 : 1;
        const grid = getGrid();
        const sheet = opts.getSheet();
        const d = sheet.drawings[selDi];
        if (d) {
          const prevAnchor = d.anchor;
          const r = anchorToRect(d, grid);
          if (!r) return;
          const x = Math.max(grid.originX, r.x + ndx * step);
          const y = Math.max(grid.originY, r.y + ndy * step);
          d.anchor = rectToAnchor({ x, y, w: r.w, h: r.h }, grid, prevAnchor);
          invalidateGrid();
          opts.redraw();
          opts.onDrawingMoved?.({ index: selDi, prevAnchor, anchor: d.anchor });
        }
        return;
      }
    }
    const cur = opts.activeCell.get();
    if (!cur) return;
    if ((ev.ctrlKey || ev.metaKey) && (ev.key === "c" || ev.key === "x") && opts.onCopy) {
      ev.preventDefault();
      const sel = opts.selection?.get() ?? { r1: cur.r, c1: cur.c, r2: cur.r, c2: cur.c };
      opts.onCopy(sel, ev.key === "x");
      return;
    }
    if ((ev.ctrlKey || ev.metaKey) && ev.key === "v" && opts.onPaste) {
      ev.preventDefault();
      opts.onPaste({ r: cur.r, c: cur.c });
      return;
    }
    if ((ev.key === "Delete" || ev.key === "Backspace") && opts.onClear) {
      ev.preventDefault();
      const sel = opts.selection?.get() ?? { r1: cur.r, c1: cur.c, r2: cur.r, c2: cur.c };
      opts.onClear(sel);
      return;
    }
    let dr = 0,
      dc = 0;
    switch (ev.key) {
      case "ArrowUp":
        dr = -1;
        break;
      case "ArrowDown":
        dr = 1;
        break;
      case "ArrowLeft":
        dc = -1;
        break;
      case "ArrowRight":
        dc = 1;
        break;
      case "Tab":
        dc = ev.shiftKey ? -1 : 1;
        break;
      case "Enter":
        dr = ev.shiftKey ? -1 : 1;
        break;
      case "F2":
        ev.preventDefault();
        opts.onEditStart?.(cur, null);
        return;
      default:
        if (opts.onEditStart && ev.key.length === 1 && !ev.ctrlKey && !ev.metaKey) {
          ev.preventDefault();
          opts.onEditStart(cur, ev.key);
        }
        return;
    }
    ev.preventDefault();
    const grid = getGrid();

    if (ev.shiftKey && (dr !== 0 || dc !== 0) && opts.selection) {
      const sel = opts.selection.get() ?? {
        r1: cur.r,
        c1: cur.c,
        r2: cur.r,
        c2: cur.c,
      };
      const leadR = sel.r1 === cur.r ? sel.r2 : sel.r1;
      const leadC = sel.c1 === cur.c ? sel.c2 : sel.c1;
      const nextLeadR = clamp(leadR + dr, 1, grid.maxRow);
      const nextLeadC = clamp(leadC + dc, 1, grid.maxCol);
      const r1 = Math.min(cur.r, nextLeadR);
      const r2 = Math.max(cur.r, nextLeadR);
      const c1 = Math.min(cur.c, nextLeadC);
      const c2 = Math.max(cur.c, nextLeadC);
      setSelection(cur, { r1, c1, r2, c2 });
      ensureVisible({ r: nextLeadR, c: nextLeadC });
      opts.redraw();
      return;
    }
    const next = {
      r: clamp(cur.r + dr, 1, grid.maxRow),
      c: clamp(cur.c + dc, 1, grid.maxCol),
    };

    setSelection(next, { r1: next.r, c1: next.c, r2: next.r, c2: next.c });
    ensureVisible(next);
    opts.redraw();
  }

  function onDoubleClick(ev: MouseEvent) {
    if (!opts.onEditStart) return;
    const cp = toCanvasLocal(ev);
    const p = toLogical(ev);
    const grid = getGrid();
    if (cp.x < grid.originX || cp.y < grid.originY) return;
    const cell = cellAt(grid, p.x, p.y);
    if (!cell) return;
    ev.preventDefault();
    opts.onEditStart(resolveAnchor(cell.r, cell.c), null);
  }

  function onPointerUp(ev: PointerEvent) {
    if (resizeDrag) {
      try {
        canvas.releasePointerCapture(ev.pointerId);
      } catch {}
      const moved = opts.getSheet().drawings[resizeDrag.index];
      if (moved && anchorChanged(resizeDrag.prevAnchor, moved.anchor)) {
        opts.onDrawingMoved?.({
          index: resizeDrag.index,
          prevAnchor: resizeDrag.prevAnchor,
          anchor: moved.anchor,
        });
      }
      resizeDrag = null;
    }
    if (drawDrag) {
      try {
        canvas.releasePointerCapture(ev.pointerId);
      } catch {}
      const moved = opts.getSheet().drawings[drawDrag.index];
      if (moved && anchorChanged(drawDrag.prevAnchor, moved.anchor)) {
        opts.onDrawingMoved?.({
          index: drawDrag.index,
          prevAnchor: drawDrag.prevAnchor,
          anchor: moved.anchor,
        });
      }
      drawDrag = null;
    }
    if (fillDrag) {
      try {
        canvas.releasePointerCapture(ev.pointerId);
      } catch {}
      const s = fillDrag.source;
      const target = opts.selection?.get();
      fillDrag = null;
      if (target) {
        const t = normalizeSel(target);
        if (t.r2 > s.r2 || t.c2 > s.c2) opts.onFill?.(s, t);
      }
    }
    if (drag || selDrag || pointDrag) {
      try {
        canvas.releasePointerCapture(ev.pointerId);
      } catch {}
      drag = null;
      selDrag = null;
      pointDrag = null;
    }
  }

  function onPointerLeave() {
    if (!drag) canvas.style.cursor = savedCursor;
    annotations.hidePopover();
  }

  function onWheel(ev: WheelEvent) {
    if (!ev.ctrlKey && !ev.metaKey) return;
    ev.preventDefault();
    const cur = opts.zoom.get();

    const next = clamp(cur * Math.exp(-ev.deltaY * 0.01), ZOOM_MIN, ZOOM_MAX);
    if (next === cur) return;

    const sc = opts.scrollContainer;
    const vp = opts.getViewport?.();
    if (sc && vp) {
      const r = canvas.getBoundingClientRect();
      const cssX = ev.clientX - r.left;
      const cssY = ev.clientY - r.top;

      const newVpX = vp.x + cssX * (1 / cur - 1 / next);
      const newVpY = vp.y + cssY * (1 / cur - 1 / next);
      opts.zoom.set(next);
      sc.scrollLeft = Math.max(0, newVpX * next);
      sc.scrollTop = Math.max(0, newVpY * next);

      opts.redraw();
    } else if (sc) {
      const r = canvas.getBoundingClientRect();
      const px = ev.clientX - r.left;
      const py = ev.clientY - r.top;
      const lx = px / cur;
      const ly = py / cur;
      opts.zoom.set(next);
      opts.redraw();
      const newPx = lx * next;
      const newPy = ly * next;
      sc.scrollLeft += newPx - px;
      sc.scrollTop += newPy - py;
    } else {
      opts.zoom.set(next);
      opts.redraw();
    }
  }

  if (!canvas.hasAttribute("tabindex")) canvas.tabIndex = 0;

  const savedOutline = canvas.style.outline;
  canvas.style.outline = "none";

  canvas.addEventListener("pointermove", onPointerMove);
  canvas.addEventListener("pointerdown", onPointerDown);
  canvas.addEventListener("pointerup", onPointerUp);
  canvas.addEventListener("pointercancel", onPointerUp);
  canvas.addEventListener("pointerleave", onPointerLeave);
  canvas.addEventListener("dblclick", onDoubleClick);
  canvas.addEventListener("keydown", onKeyDown);

  canvas.addEventListener("wheel", onWheel, { passive: false });

  return {
    destroy() {
      canvas.removeEventListener("pointermove", onPointerMove);
      canvas.removeEventListener("pointerdown", onPointerDown);
      canvas.removeEventListener("pointerup", onPointerUp);
      canvas.removeEventListener("pointercancel", onPointerUp);
      canvas.removeEventListener("pointerleave", onPointerLeave);
      canvas.removeEventListener("dblclick", onDoubleClick);
      canvas.removeEventListener("keydown", onKeyDown);
      canvas.removeEventListener("wheel", onWheel);
      canvas.style.cursor = savedCursor;
      canvas.style.outline = savedOutline;
      annotations.destroy();
    },
  };
}

function clamp(v: number, lo: number, hi: number): number {
  return v < lo ? lo : v > hi ? hi : v;
}
