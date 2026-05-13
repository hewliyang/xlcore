import type { Comment, Hyperlink, Sheet } from "./types.js";

type CellRef = { r: number; c: number };
type CommentAnchorRect = { left: number; top: number; right: number };

export interface AnnotationLayer {
  ensureMaps(): void;
  hyperlinkAt(cell: CellRef): Hyperlink | undefined;
  commentAt(cell: CellRef): Comment | undefined;
  hidePopover(): void;
  showPopover(comment: Comment, anchorClient: CommentAnchorRect): void;
  openHyperlink(link: Hyperlink): void;
  destroy(): void;
}

export function createAnnotationLayer(
  canvas: HTMLCanvasElement,
  getSheet: () => Sheet,
): AnnotationLayer {
  let mapsForSheet: Sheet | null = null;
  let hyperlinkMap = new Map<string, Hyperlink>();
  let commentMap = new Map<string, Comment>();
  let popoverEl: HTMLDivElement | null = null;

  function ensureMaps() {
    const sheet = getSheet();
    if (mapsForSheet === sheet) return;
    mapsForSheet = sheet;
    hyperlinkMap = new Map();
    commentMap = new Map();
    for (const h of sheet.hyperlinks ?? []) {
      for (let r = h.range.r1; r <= h.range.r2; r++) {
        for (let c = h.range.c1; c <= h.range.c2; c++) {
          hyperlinkMap.set(key({ r, c }), h);
        }
      }
    }
    for (const comment of sheet.comments ?? []) {
      commentMap.set(key(comment), comment);
    }
  }

  function ensurePopover(): HTMLDivElement {
    if (popoverEl) return popoverEl;
    const el = document.createElement("div");
    el.setAttribute("data-xlcore", "comment-popover");
    el.style.cssText = [
      "position: fixed",
      "z-index: 10000",
      "max-width: 280px",
      "padding: 6px 10px",
      "background: #fffbcb",
      "border: 1px solid #c0a060",
      "box-shadow: 2px 2px 6px rgba(0,0,0,0.18)",
      "font: 12px -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif",
      "color: #111",
      "white-space: pre-wrap",
      "pointer-events: none",
      "display: none",
    ].join("; ");
    document.body.appendChild(el);
    popoverEl = el;
    return el;
  }

  function hidePopover() {
    if (popoverEl) popoverEl.style.display = "none";
  }

  function showPopover(comment: Comment, anchorClient: CommentAnchorRect) {
    const el = ensurePopover();
    el.textContent = "";
    if (comment.author) {
      const author = document.createElement("div");
      author.style.cssText = "font-weight: 600; margin-bottom: 2px;";
      author.textContent = comment.author;
      el.appendChild(author);
    }
    const body = document.createElement("div");
    body.textContent = comment.text;
    el.appendChild(body);

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

  function openHyperlink(link: Hyperlink) {
    const target = link.target ?? "";
    const isInWorkbook = target.startsWith("#") || (!link.target && !!link.location);
    if (isInWorkbook) {
      const location = link.target?.startsWith("#") ? link.target.slice(1) : (link.location ?? "");
      canvas.dispatchEvent(
        new CustomEvent("xlcore-hyperlink-jump", {
          detail: { location },
          bubbles: true,
        }),
      );
      return;
    }
    if (link.target) window.open(link.target, "_blank", "noopener");
  }

  function destroy() {
    if (popoverEl?.parentNode) popoverEl.parentNode.removeChild(popoverEl);
    popoverEl = null;
  }

  return {
    ensureMaps,
    hyperlinkAt: (cell) => hyperlinkMap.get(key(cell)),
    commentAt: (cell) => commentMap.get(key(cell)),
    hidePopover,
    showPopover,
    openHyperlink,
    destroy,
  };
}

function key(cell: CellRef): string {
  return `${cell.r}:${cell.c}`;
}
