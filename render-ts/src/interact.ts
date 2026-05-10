// Interactive layer for the xlcore canvas renderer.
//
// Adds:
//   * Pinch-to-zoom (ctrl+wheel; trackpad pinches arrive as ctrl+wheel in
//     every modern browser). Cmd/Ctrl + wheel works as a fallback.
//   * Column/row resize by dragging the boundary in the header strip.
//
// Interaction state (zoom value, width/height overrides) lives outside the
// renderer; the host owns it and calls `redraw()` whenever interact updates
// the maps. This keeps the layout JSON immutable.
import type { Sheet, WorkbookLayout, Comment, Hyperlink } from "./types.js";
import { HEADER_H, HEADER_W, buildGrid, frozenDims } from "./render.js";

const RESIZE_TOL = 4; // px on either side of a boundary that's draggable
const MIN_COL_W = 8;
const MIN_ROW_H = 4;
const ZOOM_MIN = 0.25;
const ZOOM_MAX = 4;

export interface InteractHandle {
  /** Detach all listeners. */
  destroy(): void;
}

export interface InteractOptions {
  getSheet(): Sheet;
  getLayout(): WorkbookLayout;
  /** Read/write mailbox for the current zoom factor (1 = 100%). */
  zoom: { get(): number; set(value: number): void };
  /** 1-based column index → width in CSS px. Mutated in place on resize. */
  colOverrides: Map<number, number>;
  /** 1-based row index → height in CSS px. Mutated in place on resize. */
  rowOverrides: Map<number, number>;
  /**
   * Read/write mailbox for the active cell (1-based). `null` means no
   * selection. Updated by clicks and arrow keys; the host can also push
   * external selections in.
   */
  activeCell: {
    get(): { r: number; c: number } | null;
    set(v: { r: number; c: number } | null): void;
  };
  /**
   * Read/write mailbox for the multi-cell selection range (1-based,
   * inclusive). When omitted the renderer falls back to a 1×1 range at
   * `activeCell`. Header clicks expand it to whole columns / rows.
   */
  selection?: { get(): Selection | null; set(v: Selection | null): void };
  /** Optional element to scroll-anchor zoom around and to auto-scroll on arrow-key navigation. */
  scrollContainer?: HTMLElement;
  /**
   * Current viewport offset (logical px, pre-zoom). When provided, the
   * interaction layer assumes the canvas is virtualized: pointer coords get
   * `viewport.x/y` added before being mapped onto the sheet, headers pan
   * with scroll, etc.
   */
  getViewport?: () => { x: number; y: number; w: number; h: number } | null;
  /** Called whenever interact mutates state and the canvas should re-paint. */
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

/**
 * Wire up interactivity on `canvas`. Idempotent per-canvas: call `destroy()`
 * on the returned handle before reattaching.
 */
export function attachInteractivity(
  canvas: HTMLCanvasElement,
  opts: InteractOptions,
): InteractHandle {
  let drag: { hit: HitCol | HitRow; startPx: number; original: number } | null = null;
  const savedCursor = canvas.style.cursor;
  let cachedGrid:
    | {
        sheet: Sheet;
        colOverrides: Map<number, number>;
        rowOverrides: Map<number, number>;
        grid: ReturnType<typeof buildGrid>;
      }
    | null = null;

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

  // ---- annotations: hyperlink click + comment hover popover ----
  //
  // Both maps are keyed by `"r:c"` 1-based cell strings, including every
  // cell of a multi-cell hyperlink range and the comment anchor (which
  // is always a single cell). Rebuilt lazily per sheet via `ensureMaps`
  // below so tab switches in the host don't keep stale entries.
  let mapsForSheet: Sheet | null = null;
  let hyperlinkMap = new Map<string, Hyperlink>();
  let commentMap = new Map<string, Comment>();

  function ensureMaps() {
    const sheet = opts.getSheet();
    if (mapsForSheet === sheet) return;
    mapsForSheet = sheet;
    hyperlinkMap = new Map();
    commentMap = new Map();
    for (const h of sheet.hyperlinks ?? []) {
      for (let r = h.range.r1; r <= h.range.r2; r++) {
        for (let c = h.range.c1; c <= h.range.c2; c++) {
          hyperlinkMap.set(`${r}:${c}`, h);
        }
      }
    }
    for (const cmt of sheet.comments ?? []) {
      commentMap.set(`${cmt.r}:${cmt.c}`, cmt);
    }
  }

  // Resolve a cell to its merge top-left when applicable. Hyperlinks and
  // comments anchor on the merge's top-left in OOXML, so a click inside
  // a merged region needs to look there before checking the maps.
  function resolveAnchor(r: number, c: number): { r: number; c: number } {
    const sheet = opts.getSheet();
    for (const m of sheet.merges) {
      if (r >= m.r1 && r <= m.r2 && c >= m.c1 && c <= m.c2) return { r: m.r1, c: m.c1 };
    }
    return { r, c };
  }

  // Single shared popover element, lazily created on the first comment
  // hover. We attach to `document.body` so it floats above any host
  // chrome (scrollbars, sticky headers) without z-index gymnastics.
  let popoverEl: HTMLDivElement | null = null;
  function ensurePopover(): HTMLDivElement {
    if (popoverEl) return popoverEl;
    const el = document.createElement("div");
    el.setAttribute("data-xlcore", "comment-popover");
    el.style.cssText = [
      "position: fixed",
      "z-index: 10000",
      "max-width: 280px",
      "padding: 6px 10px",
      "background: #fffbcb", // Excel's pale-yellow comment fill
      "border: 1px solid #c0a060",
      "box-shadow: 2px 2px 6px rgba(0,0,0,0.18)",
      "font: 12px -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif",
      "color: #111",
      "white-space: pre-wrap", // preserve newlines inside the body
      "pointer-events: none", // never steal pointer events
      "display: none",
    ].join("; ");
    document.body.appendChild(el);
    popoverEl = el;
    return el;
  }
  function hidePopover() {
    if (popoverEl) popoverEl.style.display = "none";
  }
  function showPopover(cmt: Comment, anchorClient: { left: number; top: number; right: number }) {
    const el = ensurePopover();
    // Clear + rebuild content. Author is bold on its own line; body
    // preserves whitespace so multi-line comments read correctly.
    el.textContent = "";
    if (cmt.author) {
      const a = document.createElement("div");
      a.style.cssText = "font-weight: 600; margin-bottom: 2px;";
      a.textContent = cmt.author;
      el.appendChild(a);
    }
    const body = document.createElement("div");
    body.textContent = cmt.text;
    el.appendChild(body);
    // Anchor: just to the right of the cell, vertically aligned with
    // its top. Falls back to left side if the right edge would clip
    // off the viewport.
    el.style.display = "block";
    const popW = el.offsetWidth;
    const popH = el.offsetHeight;
    let x = anchorClient.right + 6;
    let y = anchorClient.top;
    if (x + popW > window.innerWidth - 4) x = anchorClient.left - popW - 6;
    if (y + popH > window.innerHeight - 4) y = window.innerHeight - popH - 4;
    if (y < 4) y = 4;
    el.style.left = x + "px";
    el.style.top = y + "px";
  }

  // Cell at canvas-local logical position, or null when in a header /
  // outside the grid. Used by hyperlink + comment hit-testing.
  function cellAtLogical(p: { x: number; y: number }): { r: number; c: number } | null {
    if (p.x < HEADER_W || p.y < HEADER_H) return null;
    const grid = getGrid();
    return cellAt(grid, p.x, p.y);
  }

  // Convert client coords → *canvas-local* logical px (pre-zoom). Header
  // strips live in this space (always pinned to canvas edges).
  function toCanvasLocal(ev: { clientX: number; clientY: number }): { x: number; y: number } {
    const r = canvas.getBoundingClientRect();
    const z = opts.zoom.get();
    return {
      x: (ev.clientX - r.left) / z,
      y: (ev.clientY - r.top) / z,
    };
  }

  // Convert client coords → *sheet* logical px (data area). Per-pane: a
  // click in a pinned pane has no scroll offset on the pinned axis, while
  // a click in BR adds the viewport offset on both axes. Returns null if
  // the point is in a header strip / outside any pane.
  function toLogical(ev: { clientX: number; clientY: number }): { x: number; y: number } {
    const p = toCanvasLocal(ev);
    const vp = opts.getViewport?.() ?? null;
    const sheet = opts.getSheet();
    const grid = getGrid();
    const { pcw, prh } = frozenDims(sheet, grid);
    // Per-axis: only the scrolling segment past the freeze split picks up
    // the viewport offset. Pinned col-header / row-header clicks and
    // clicks inside pinned panes map directly to absolute grid coords.
    const sx = vp && p.x > HEADER_W + pcw ? vp.x : 0;
    const sy = vp && p.y > HEADER_H + prh ? vp.y : 0;
    return { x: p.x + sx, y: p.y + sy };
  }

  // x/y here are *canvas-local*. The header strip is split into a pinned
  // segment (cols [1..splitX-1] / rows [1..splitY-1]) that doesn't pan and
  // a scrolling segment that does — we hit-test each separately so resize
  // handles line up exactly with the rendered tab boundaries.
  function hitTest(cx: number, cy: number): Hit {
    const vp = opts.getViewport?.() ?? null;
    const sx = vp?.x ?? 0;
    const sy = vp?.y ?? 0;
    const sheet = opts.getSheet();
    const grid = getGrid();
    const { splitX, splitY, pcw, prh } = frozenDims(sheet, grid);

    if (cy >= 0 && cy <= HEADER_H && cx > HEADER_W) {
      // Pinned col-header segment: canvas x maps directly to absolute grid x.
      if (cx <= HEADER_W + pcw) {
        const edgeIndex = nearestEdgeIndex(grid.colX, cx, 2, splitX);
        if (edgeIndex !== null) {
          return { kind: "col", index: edgeIndex - 1, edgeX: grid.colX[edgeIndex] ?? 0 };
        }
      } else {
        // Scrolling segment: cx + sx → absolute grid x.
        const x = cx + sx;
        const edgeIndex = nearestEdgeIndex(
          grid.colX,
          x,
          Math.max(splitX + 1, 2),
          grid.maxCol + 1,
        );
        if (edgeIndex !== null) {
          return { kind: "col", index: edgeIndex - 1, edgeX: grid.colX[edgeIndex] ?? 0 };
        }
      }
    }
    if (cx >= 0 && cx <= HEADER_W && cy > HEADER_H) {
      if (cy <= HEADER_H + prh) {
        const edgeIndex = nearestEdgeIndex(grid.rowY, cy, 2, splitY);
        if (edgeIndex !== null) {
          return { kind: "row", index: edgeIndex - 1, edgeY: grid.rowY[edgeIndex] ?? 0 };
        }
      } else {
        const y = cy + sy;
        const edgeIndex = nearestEdgeIndex(
          grid.rowY,
          y,
          Math.max(splitY + 1, 2),
          grid.maxRow + 1,
        );
        if (edgeIndex !== null) {
          return { kind: "row", index: edgeIndex - 1, edgeY: grid.rowY[edgeIndex] ?? 0 };
        }
      }
    }
    return null;
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
    const cp = toCanvasLocal(ev);
    const hit = hitTest(cp.x, cp.y);
    if (hit) {
      canvas.style.cursor = hit.kind === "col" ? "col-resize" : "row-resize";
      hidePopover();
      return;
    }

    // No resize hit — check annotations on the cell under the cursor.
    ensureMaps();
    const lp = toLogical(ev);
    const cell = cellAtLogical(lp);
    if (!cell) {
      canvas.style.cursor = savedCursor;
      hidePopover();
      return;
    }
    const anchor = resolveAnchor(cell.r, cell.c);
    const k = `${anchor.r}:${anchor.c}`;
    const link = hyperlinkMap.get(k);
    const cmt = commentMap.get(k);

    canvas.style.cursor = link ? "pointer" : savedCursor;

    if (cmt) {
      // Position the popover relative to the cell's on-screen rect.
      // Map sheet logical coords → client coords through the same
      // pinned/scrolled split that `toLogical` uses in reverse.
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
      // Keep pinned-frozen-pane comments visible: the rect's still on screen.
      // No-op for the regular case.
      void splitX;
      void splitY;
      void pcw;
      void prh;
      showPopover(cmt, { left, top, right });
    } else {
      hidePopover();
    }
  }

  function setSelection(active: { r: number; c: number }, range: Selection) {
    opts.activeCell.set(active);
    opts.selection?.set(range);
  }

  function onPointerDown(ev: PointerEvent) {
    if (ev.button !== 0) return;
    const cp = toCanvasLocal(ev);
    const p = toLogical(ev);
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
    // Header hit-test uses canvas-local because the header strips don't pan
    // on their pinned axis.
    const inColHeader = cp.y >= 0 && cp.y < HEADER_H;
    const inRowHeader = cp.x >= 0 && cp.x < HEADER_W;

    // Top-left gutter intersection: select-all.
    if (inColHeader && inRowHeader) {
      ev.preventDefault();
      setSelection({ r: 1, c: 1 }, { r1: 1, c1: 1, r2: grid.maxRow, c2: grid.maxCol });
      opts.redraw();
      canvas.focus({ preventScroll: true });
      return;
    }
    // Column header (excluding resize zones, handled above): select entire column.
    if (inColHeader && cp.x >= HEADER_W) {
      const cell = cellAt(grid, p.x, HEADER_H + 1);
      if (cell) {
        ev.preventDefault();
        setSelection({ r: 1, c: cell.c }, { r1: 1, c1: cell.c, r2: grid.maxRow, c2: cell.c });
        opts.redraw();
        canvas.focus({ preventScroll: true });
      }
      return;
    }
    // Row header: select entire row.
    if (inRowHeader && cp.y >= HEADER_H) {
      const cell = cellAt(grid, HEADER_W + 1, p.y);
      if (cell) {
        ev.preventDefault();
        setSelection({ r: cell.r, c: 1 }, { r1: cell.r, c1: 1, r2: cell.r, c2: grid.maxCol });
        opts.redraw();
        canvas.focus({ preventScroll: true });
      }
      return;
    }

    // Click landed in the data area: select that cell.
    if (cp.x >= HEADER_W && cp.y >= HEADER_H) {
      const cell = cellAt(grid, p.x, p.y);
      if (cell) {
        // Resolve merge: clicks anywhere inside a merged region select the
        // merge's top-left and the selection rect spans the whole merge.
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
        opts.redraw();
        canvas.focus({ preventScroll: true });

        // Hyperlink: open after the selection update so the cell still
        // gets the focus ring before the new tab steals attention.
        // Excel matches single-click-opens for cells with a hyperlink.
        ensureMaps();
        const link = hyperlinkMap.get(`${anchor.r}:${anchor.c}`);
        if (link) openHyperlink(link);
      }
    }
  }

  /// Open the link target in a new tab. External targets resolve as-is;
  /// in-workbook `location` jumps fall through to the host (we just log
  /// since cross-sheet navigation is host-specific). `target` strings
  /// prefixed with `#` (e.g. `#Sheet1!D7`) come from writers that fold
  /// in-workbook links into the rel target rather than the `location`
  /// attribute — we treat them as locations here too.
  function openHyperlink(link: Hyperlink) {
    const t = link.target ?? "";
    const isInWorkbook = t.startsWith("#") || (!link.target && !!link.location);
    if (isInWorkbook) {
      // Host-specific: dispatch a custom event the embedder can listen to.
      const dest = link.target?.startsWith("#") ? link.target.slice(1) : (link.location ?? "");
      canvas.dispatchEvent(
        new CustomEvent("xlcore-hyperlink-jump", {
          detail: { location: dest },
          bubbles: true,
        }),
      );
      return;
    }
    if (link.target) {
      // `noopener` to keep the new tab from reaching back into our window.
      window.open(link.target, "_blank", "noopener");
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
    // Pinned cells are always visible — frozen panes guarantee it.
    // Otherwise the BR pane starts at (HEADER_W + pcw) * z on canvas, so
    // that's the left/top padding the cell must clear before we scroll.
    const padX = (HEADER_W + pcw) * z;
    const padY = (HEADER_H + prh) * z;
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
    const next = {
      r: clamp(cur.r + dr, 1, grid.maxRow),
      c: clamp(cur.c + dc, 1, grid.maxCol),
    };
    // Arrow-key navigation always collapses any expanded selection back to a
    // single cell, matching Excel.
    setSelection(next, { r1: next.r, c1: next.c, r2: next.r, c2: next.c });
    ensureVisible(next);
    opts.redraw();
  }

  function onPointerUp(ev: PointerEvent) {
    if (drag) {
      try {
        canvas.releasePointerCapture(ev.pointerId);
      } catch {
        /* noop */
      }
      drag = null;
    }
  }

  function onPointerLeave() {
    if (!drag) canvas.style.cursor = savedCursor;
    hidePopover();
  }

  // Trackpad pinch arrives as wheel + ctrlKey. Cmd/Ctrl + wheel is the
  // keyboard-zoom fallback. We swallow these only when a modifier is active
  // so plain scroll continues to work.
  function onWheel(ev: WheelEvent) {
    if (!ev.ctrlKey && !ev.metaKey) return;
    ev.preventDefault();
    const cur = opts.zoom.get();
    // Pinch deltas are small per tick; use exponential scaling so it feels
    // proportional regardless of starting zoom.
    const next = clamp(cur * Math.exp(-ev.deltaY * 0.01), ZOOM_MIN, ZOOM_MAX);
    if (next === cur) return;

    // Anchor: keep the sheet logical point under the cursor stationary on
    // screen. With a virtualized canvas the scroll container hosts a sized
    // spacer (CSS px = logical * zoom), so scrollLeft = viewport.x * zoom.
    const sc = opts.scrollContainer;
    const vp = opts.getViewport?.();
    if (sc && vp) {
      const r = canvas.getBoundingClientRect();
      const cssX = ev.clientX - r.left;
      const cssY = ev.clientY - r.top;
      // newVp.x = vp.x + cssX * (1/cur - 1/next) keeps the same sheet point
      // under the cursor after the zoom change.
      const newVpX = vp.x + cssX * (1 / cur - 1 / next);
      const newVpY = vp.y + cssY * (1 / cur - 1 / next);
      opts.zoom.set(next);
      sc.scrollLeft = Math.max(0, newVpX * next);
      sc.scrollTop = Math.max(0, newVpY * next);
      // Setting scroll dispatches a 'scroll' event which the host uses to
      // recompute viewport + redraw; we still call redraw() in case the
      // host doesn't listen.
      opts.redraw();
    } else if (sc) {
      // Legacy non-virtualized path: canvas itself spans the sheet, so
      // anchoring on canvas-local logical px is correct.
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

  // Make the canvas keyboard-focusable so arrow keys reach onKeyDown.
  if (!canvas.hasAttribute("tabindex")) canvas.tabIndex = 0;
  // Suppress the default focus ring; the active-cell highlight is the
  // affordance and a halo around the whole canvas would be noisy.
  const savedOutline = canvas.style.outline;
  canvas.style.outline = "none";

  canvas.addEventListener("pointermove", onPointerMove);
  canvas.addEventListener("pointerdown", onPointerDown);
  canvas.addEventListener("pointerup", onPointerUp);
  canvas.addEventListener("pointercancel", onPointerUp);
  canvas.addEventListener("pointerleave", onPointerLeave);
  canvas.addEventListener("keydown", onKeyDown);
  // wheel must be non-passive to call preventDefault on pinch events.
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
      if (popoverEl && popoverEl.parentNode) popoverEl.parentNode.removeChild(popoverEl);
      popoverEl = null;
    },
  };
}

function clamp(v: number, lo: number, hi: number): number {
  return v < lo ? lo : v > hi ? hi : v;
}
