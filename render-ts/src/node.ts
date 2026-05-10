import { Canvas, createCanvas } from "@napi-rs/canvas";
import { decodeWorkbookLayout } from "./columnar.js";
import { render, buildGrid } from "./render.js";
import { setOffscreenCanvasFactory } from "./canvasFactory.js";
import type { RenderOptions } from "./renderTypes.js";
import type { Sheet as RuntimeSheet, WorkbookLayout } from "./types.js";

export interface RenderPngOptions extends RenderOptions {
  sheetIndex?: number;
}

setOffscreenCanvasFactory(
  (width, height) =>
    createCanvas(width, height) as unknown as {
      width: number;
      height: number;
      getContext(t: "2d"): CanvasRenderingContext2D | null;
    },
);

export function renderToCanvas(layout: WorkbookLayout, opts: RenderPngOptions = {}): Canvas {
  decodeWorkbookLayout(layout);
  const sheet = pickSheet(layout, opts.sheetIndex);
  const viewport = opts.viewport ?? defaultViewport(sheet, opts);
  const canvas = createCanvas(
    Math.ceil(viewport.w * (opts.zoom ?? 1) * (opts.scale ?? 1)),
    Math.ceil(viewport.h * (opts.zoom ?? 1) * (opts.scale ?? 1)),
  );
  render(canvas as unknown as Parameters<typeof render>[0], sheet, layout, {
    ...opts,
    viewport,
  });
  return canvas;
}

export function renderToPng(layout: WorkbookLayout, opts: RenderPngOptions = {}): Buffer {
  return renderToCanvas(layout, opts).toBuffer("image/png");
}

function pickSheet(layout: WorkbookLayout, sheetIndex: number | undefined): RuntimeSheet {
  const index = sheetIndex ?? layout.activeSheetIndex ?? 0;
  const sheet = layout.sheets[index];
  if (!sheet) throw new Error(`sheetIndex out of range: ${index}`);
  return sheet as unknown as RuntimeSheet;
}

function defaultViewport(sheet: RuntimeSheet, opts: RenderOptions) {
  const grid = buildGrid(sheet);
  return {
    x: 0,
    y: 0,
    w: Math.min(grid.totalW, opts.renderHeaders === false ? 1200 : 1244),
    h: Math.min(grid.totalH, opts.renderHeaders === false ? 800 : 822),
  };
}
