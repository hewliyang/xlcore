import type { Drawing, Sheet } from "./types.js";
import { drawChart } from "./chart.js";
import { anchorToRect } from "./grid.js";
import type { Grid } from "./grid.js";

// ---------- drawings (charts/images) ----------

// Decoded HTMLImageElement cache, keyed by dataUri. We hold the image
// across redraws so we don't re-decode the (potentially-megabyte) base64
// blob every frame. When a fresh image finishes decoding, fire an event the
// preview shell listens for so the next paint actually shows the picture.
export type DrawableImage = {
  complete?: boolean;
  naturalWidth?: number;
  naturalHeight?: number;
  width?: number;
  height?: number;
  decoding?: "async" | "sync" | "auto";
  onload?: ((event?: Event) => void) | null;
  src?: string | Uint8Array | ArrayBuffer;
};

const imageCache = new Map<string, DrawableImage>();

function getOrLoadImage(uri: string): DrawableImage | null {
  const cached = imageCache.get(uri);
  if (cached) return imageHasSize(cached) ? cached : null;
  // ImageBitmap would be slightly faster on Chrome but `<img>` works in
  // every renderer target (browser + node-canvas eventually).
  const img = new Image() as HTMLImageElement & DrawableImage;
  const bytes = dataUriBytes(uri);
  if (bytes) {
    (img as unknown as { src: Uint8Array | ArrayBuffer }).src = bytes;
    imageCache.set(uri, img);
    return imageHasSize(img) ? img : null;
  }
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

function imageHasSize(img: DrawableImage): boolean {
  const measured = img as DrawableImage & { width?: number; height?: number };
  return (img.naturalWidth ?? measured.width ?? 0) > 0 && (img.naturalHeight ?? measured.height ?? 0) > 0;
}

function dataUriBytes(uri: string): Uint8Array | null {
  if (!uri.startsWith("data:")) return null;
  const comma = uri.indexOf(",");
  if (comma < 0 || !uri.slice(0, comma).includes(";base64")) return null;
  const BufferCtor = (globalThis as unknown as { Buffer?: { from(data: string, encoding: "base64"): Uint8Array } }).Buffer;
  return BufferCtor?.from(uri.slice(comma + 1), "base64") ?? null;
}

export async function preloadDrawingImages(
  sheet: Sheet,
  load: (bytes: Uint8Array) => Promise<DrawableImage>,
): Promise<void> {
  await Promise.all(
    (sheet.drawings ?? []).map(async (drawing: Drawing) => {
      if (drawing.kind !== "image" || !drawing.image) return;
      const uri = drawing.image.dataUri;
      const cached = imageCache.get(uri);
      if (cached && imageHasSize(cached)) return;
      const bytes = dataUriBytes(uri);
      if (!bytes) return;
      const img = await load(bytes);
      imageCache.set(uri, img);
    }),
  );
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
        ctx.drawImage(img as CanvasImageSource, rect.x, rect.y, rect.w, rect.h);
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
