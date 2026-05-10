import type { Sheet } from "./types.js";
import { drawChart } from "./chart.js";
import { anchorToRect } from "./grid.js";
import type { Grid } from "./grid.js";

// ---------- drawings (charts/images) ----------

// Decoded HTMLImageElement cache, keyed by dataUri. We hold the image
// across redraws so we don't re-decode the (potentially-megabyte) base64
// blob every frame. When a fresh image finishes decoding, fire an event the
// preview shell listens for so the next paint actually shows the picture.
const imageCache = new Map<string, HTMLImageElement>();

function getOrLoadImage(uri: string): HTMLImageElement | null {
  let img = imageCache.get(uri);
  if (img) return img.complete && img.naturalWidth > 0 ? img : null;
  // ImageBitmap would be slightly faster on Chrome but `<img>` works in
  // every renderer target (browser + node-canvas eventually).
  img = new Image();
  img.decoding = "async";
  img.onload = () => {
    try {
      (globalThis as any).dispatchEvent?.(new Event("xlcore-image-ready"));
    } catch {}
  };
  img.src = uri;
  imageCache.set(uri, img);
  return null;
}

export function drawDrawings(ctx: CanvasRenderingContext2D, sheet: Sheet, g: Grid): void {
  if (!sheet.drawings || sheet.drawings.length === 0) return;
  for (const d of sheet.drawings) {
    const rect = anchorToRect(d, g);
    if (!rect) continue;
    if (d.kind === "chart" && d.chart) {
      drawChart(ctx, d.chart, rect);
    } else if (d.kind === "image" && d.image) {
      const img = getOrLoadImage(d.image.dataUri);
      if (img) {
        ctx.drawImage(img, rect.x, rect.y, rect.w, rect.h);
      } else {
        // Faint placeholder while we decode — keeps layout stable.
        ctx.fillStyle = "#f4f4f5";
        ctx.fillRect(rect.x, rect.y, rect.w, rect.h);
      }
    } else {
      // Placeholder for non-chart, non-image drawings.
      ctx.fillStyle = "#f4f4f5";
      ctx.fillRect(rect.x, rect.y, rect.w, rect.h);
      ctx.strokeStyle = "#d4d4d8";
      ctx.strokeRect(rect.x + 0.5, rect.y + 0.5, rect.w - 1, rect.h - 1);
    }
  }
}
