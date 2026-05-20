import type { Sheet, WorkbookLayout } from "./types.js";
import { drawingHyperlinkAt } from "./drawingHits.js";
import { buildGrid, frozenDims } from "./render.js";
import { createAnnotationLayer } from "./interactAnnotations.js";
import {
  computeOutlineRuns,
  outlineButtonHits,
  outlineCornerHits,
  type OutlineRun,
  OUTLINE_BUTTON_HIT_RADIUS,
} from "./outlineGutter.js";

const RESIZE_TOL = 4;
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

  scrollContainer?: HTMLElement;

  getViewport?: () => { x: number; y: number; w: number; h: number } | null;

  redraw(): void;
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

  function maybeOutlineCursor(cp: { x: number; y: number }): boolean {
    if (outlineButtonAt(cp) || outlineCornerAt(cp)) {
      canvas.style.cursor = "pointer";
      annotations.hidePopover();
      return true;
    }
    return false;
  }

  function onPointerMove(ev: PointerEvent) {
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
    if (maybeOutlineCursor(cp)) return;
    const hit = hitTest(cp.x, cp.y);
    if (hit) {
      canvas.style.cursor = hit.kind === "col" ? "col-resize" : "row-resize";
      annotations.hidePopover();
      return;
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

  function setSelection(active: { r: number; c: number }, range: Selection) {
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

    if (cp.x >= grid.originX && cp.y >= grid.originY) {
      const cell = cellAt(grid, p.x, p.y);
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
    const cur = opts.activeCell.get();
    if (!cur) return;
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
      default:
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

  function onPointerUp(ev: PointerEvent) {
    if (drag || selDrag) {
      try {
        canvas.releasePointerCapture(ev.pointerId);
      } catch {}
      drag = null;
      selDrag = null;
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
  canvas.addEventListener("keydown", onKeyDown);

  canvas.addEventListener("wheel", onWheel, { passive: false });

  return {
    destroy() {
      canvas.removeEventListener("pointermove", onPointerMove);
      canvas.removeEventListener("pointerdown", onPointerDown);
      canvas.removeEventListener("pointerup", onPointerUp);
      canvas.removeEventListener("pointercancel", onPointerUp);
      canvas.removeEventListener("pointerleave", onPointerLeave);
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
