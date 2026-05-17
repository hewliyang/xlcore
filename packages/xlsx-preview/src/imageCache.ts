// Shared decoded-image cache, used by both the top-level image-anchor
// painter (drawings.ts) and the nested-picture-inside-shape painter
// (shape.ts). Keyed by `data:` URI so identical embeds share a single
// HTMLImageElement.
//
// Decoding is async in the browser path; we hand back `null` until
// the image has dimensions and fire a `xlcore-image-ready` event so
// the preview shell can request a repaint. The Node path
// (`renderToPng`) preloads everything via `preloadDrawingImages`
// before the first paint, so the cache lookup is synchronous there.

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

export function getCachedImage(uri: string): DrawableImage | undefined {
  return imageCache.get(uri);
}

export function putCachedImage(uri: string, img: DrawableImage): void {
  imageCache.set(uri, img);
}

export function imageHasSize(img: DrawableImage): boolean {
  const measured = img as DrawableImage & { width?: number; height?: number };
  return (
    (img.naturalWidth ?? measured.width ?? 0) > 0 && (img.naturalHeight ?? measured.height ?? 0) > 0
  );
}

export function dataUriBytes(uri: string): Uint8Array | null {
  if (!uri.startsWith("data:")) return null;
  const comma = uri.indexOf(",");
  if (comma < 0 || !uri.slice(0, comma).includes(";base64")) return null;
  const BufferCtor = (
    globalThis as unknown as { Buffer?: { from(data: string, encoding: "base64"): Uint8Array } }
  ).Buffer;
  return BufferCtor?.from(uri.slice(comma + 1), "base64") ?? null;
}

export function getOrLoadImage(uri: string): DrawableImage | null {
  const cached = imageCache.get(uri);
  if (cached) return imageHasSize(cached) ? cached : null;
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
