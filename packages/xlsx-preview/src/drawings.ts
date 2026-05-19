import type { Sheet } from "./types.js";
import { drawChart } from "./chart.js";
import { drawShape } from "./shape.js";
import { anchorToRect } from "./grid.js";
import type { Grid } from "./grid.js";
import {
  type DrawableImage,
  dataUriBytes,
  getCachedImage,
  getOrLoadImage,
  imageHasSize,
  putCachedImage,
} from "./imageCache.js";

export type { DrawableImage } from "./imageCache.js";

export async function preloadDrawingImages(
  sheet: Sheet,
  load: (bytes: Uint8Array) => Promise<DrawableImage>,
): Promise<void> {
  const uris: string[] = [];
  for (const drawing of sheet.drawings ?? []) {
    if (drawing.kind === "image" && drawing.image) {
      uris.push(drawing.image.dataUri);
    } else if (drawing.kind === "shape" && drawing.shape) {
      for (const node of drawing.shape.nodes) {
        if (node.imageDataUri) uris.push(node.imageDataUri);
      }
    }
  }
  await Promise.all(
    uris.map(async (uri) => {
      const cached = getCachedImage(uri);
      if (cached && imageHasSize(cached)) return;
      const bytes = dataUriBytes(uri);
      if (!bytes) return;
      const img = await load(bytes);
      putCachedImage(uri, img);
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
    } else if (d.kind === "shape" && d.shape) {
      drawShape(ctx, d.shape, rect);
    } else if (d.kind === "image" && d.image) {
      const img = getOrLoadImage(d.image.dataUri);
      if (img) {
        ctx.drawImage(img as CanvasImageSource, rect.x, rect.y, rect.w, rect.h);
      } else {
        ctx.fillStyle = "#f4f4f5";
        ctx.fillRect(rect.x, rect.y, rect.w, rect.h);
      }
    } else {
      ctx.fillStyle = "#f4f4f5";
      ctx.fillRect(rect.x, rect.y, rect.w, rect.h);
      ctx.strokeStyle = "#d4d4d8";
      ctx.strokeRect(rect.x + 0.5, rect.y + 0.5, rect.w - 1, rect.h - 1);
    }
  }
}
