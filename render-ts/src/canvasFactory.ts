type CanvasLike = {
  width: number;
  height: number;
  getContext(t: "2d"): CanvasRenderingContext2D | null;
};

let createOffscreenCanvas: ((width: number, height: number) => CanvasLike) | null = null;

export function setOffscreenCanvasFactory(
  factory: ((width: number, height: number) => CanvasLike) | null,
): void {
  createOffscreenCanvas = factory;
}

export function makeOffscreenCanvas(width: number, height: number): CanvasLike {
  if (createOffscreenCanvas) return createOffscreenCanvas(width, height);
  if (typeof document !== "undefined") {
    const canvas = document.createElement("canvas");
    canvas.width = width;
    canvas.height = height;
    return canvas;
  }
  throw new Error("no offscreen canvas factory configured");
}
