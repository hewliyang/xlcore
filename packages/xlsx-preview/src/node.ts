import { Canvas, Image } from "skia-canvas";
import { readFile } from "node:fs/promises";
import { readFileSync } from "node:fs";
import initWasm, { extract_csv, extract_parquet, extract_xlsx } from "xlcore-wasm";
import { decodeWorkbookLayout } from "./columnar.js";
import {
  EMPTY_LOAD_REPORT,
  type LoadReport,
  XlsxLoadError,
  xlsxLoadErrorPayloadFromUnknown,
} from "./errors.js";
import { render, buildGrid } from "./render.js";
import { anchorToRect } from "./grid.js";
import { setOffscreenCanvasFactory } from "./canvasFactory.js";
import type { RenderOptions, Viewport } from "./renderTypes.js";
import type { Sheet as RuntimeSheet, WorkbookLayout } from "./types.js";

export interface RenderPngOptions extends RenderOptions {
  sheetIndex?: number;
  sheetName?: string;

  range?: string;

  /**
   * Target canvas width/height in logical px (before `scale`/`zoom`). With
   * headers on this is the total canvas size (headers included, like the
   * built-in default); with `renderHeaders: false` it is exactly the cell
   * content size starting at A1. Ignored when `range` or `viewport` is given.
   */
  width?: number;
  height?: number;

  /** Receives non-fatal render warnings (default: `console.error`). */
  onWarning?: (message: string) => void;
}

setOffscreenCanvasFactory(
  (width, height) =>
    new Canvas(width, height) as unknown as {
      width: number;
      height: number;
      getContext(t: "2d"): CanvasRenderingContext2D | null;
    },
);

if (typeof globalThis.Image === "undefined") {
  (globalThis as unknown as { Image: typeof Image }).Image = Image;
}

let wasmReady: Promise<unknown> | null = null;

export interface LoadWorkbookFromXlsxOptions {
  sheetIndex?: number;
  sheetName?: string;
}

export async function loadWorkbookFromXlsx(
  input: string | ArrayBuffer | Uint8Array,
  options: LoadWorkbookFromXlsxOptions = {},
): Promise<WorkbookLayout> {
  return (await loadWorkbookFromXlsxWithReport(input, options)).layout;
}

export interface LoadedWorkbookNode {
  layout: WorkbookLayout;
  report: LoadReport;
}

export async function loadWorkbookFromXlsxWithReport(
  input: string | ArrayBuffer | Uint8Array,
  options: LoadWorkbookFromXlsxOptions = {},
): Promise<LoadedWorkbookNode> {
  await ensureWasm();
  const bytes = await bytesFromInput(input);
  let envelope: { layout: WorkbookLayout; report?: LoadReport };
  try {
    envelope = extract_xlsx(bytes, {
      sheetIndex: options.sheetIndex,
      sheetName: options.sheetName,
    }) as {
      layout: WorkbookLayout;
      report?: LoadReport;
    };
  } catch (err) {
    throw new XlsxLoadError(xlsxLoadErrorPayloadFromUnknown(err));
  }
  return { layout: envelope.layout, report: envelope.report ?? EMPTY_LOAD_REPORT };
}

export interface LoadWorkbookFromCsvOptions {
  delimiter?: string;
  maxRows?: number;
  sheetName?: string;
}

export async function loadWorkbookFromCsv(
  input: string | ArrayBuffer | Uint8Array,
  options: LoadWorkbookFromCsvOptions = {},
): Promise<WorkbookLayout> {
  return (await loadWorkbookFromCsvWithReport(input, options)).layout;
}

export async function loadWorkbookFromCsvWithReport(
  input: string | ArrayBuffer | Uint8Array,
  options: LoadWorkbookFromCsvOptions = {},
): Promise<LoadedWorkbookNode> {
  await ensureWasm();
  const bytes = await bytesFromInput(input);
  let envelope: { layout: WorkbookLayout; report?: LoadReport };
  try {
    envelope = extract_csv(bytes, options) as {
      layout: WorkbookLayout;
      report?: LoadReport;
    };
  } catch (err) {
    throw new XlsxLoadError(xlsxLoadErrorPayloadFromUnknown(err));
  }
  return { layout: envelope.layout, report: envelope.report ?? EMPTY_LOAD_REPORT };
}

export interface LoadWorkbookFromParquetOptions {
  maxRows?: number;
  sheetName?: string;
}

export async function loadWorkbookFromParquet(
  input: string | ArrayBuffer | Uint8Array,
  options: LoadWorkbookFromParquetOptions = {},
): Promise<WorkbookLayout> {
  return (await loadWorkbookFromParquetWithReport(input, options)).layout;
}

export async function loadWorkbookFromParquetWithReport(
  input: string | ArrayBuffer | Uint8Array,
  options: LoadWorkbookFromParquetOptions = {},
): Promise<LoadedWorkbookNode> {
  await ensureWasm();
  const bytes = await bytesFromInput(input);
  let envelope: { layout: WorkbookLayout; report?: LoadReport };
  try {
    envelope = extract_parquet(bytes, options) as {
      layout: WorkbookLayout;
      report?: LoadReport;
    };
  } catch (err) {
    throw new XlsxLoadError(xlsxLoadErrorPayloadFromUnknown(err));
  }
  return { layout: envelope.layout, report: envelope.report ?? EMPTY_LOAD_REPORT };
}

export async function renderXlsxToCanvas(
  input: string | ArrayBuffer | Uint8Array,
  opts: RenderPngOptions = {},
): Promise<Canvas> {
  return renderToCanvas(await loadWorkbookFromXlsx(input, loadOptionsFromRenderOptions(opts)), {
    ...opts,
    sheetIndex: undefined,
    sheetName: undefined,
  });
}

export async function renderXlsxToPng(
  input: string | ArrayBuffer | Uint8Array,
  opts: RenderPngOptions = {},
): Promise<Buffer> {
  return renderToCanvas(await loadWorkbookFromXlsx(input, loadOptionsFromRenderOptions(opts)), {
    ...opts,
    sheetIndex: undefined,
    sheetName: undefined,
  }).toBuffer("png");
}

export function renderToCanvas(layout: WorkbookLayout, opts: RenderPngOptions = {}): Canvas {
  decodeWorkbookLayout(layout);
  const range = opts.range ? parseRangeRef(opts.range) : null;
  const sheet = pickSheet(layout, opts.sheetIndex, opts.sheetName ?? range?.sheetName);
  const { viewport, crop } = resolveViewport(sheet, opts, range);
  const total = (opts.zoom ?? 1) * (opts.scale ?? 1);
  const canvas = new Canvas(Math.ceil(viewport.w * total), Math.ceil(viewport.h * total));
  render(canvas as unknown as Parameters<typeof render>[0], sheet, layout, {
    ...opts,
    viewport,
  });
  if (!crop) return canvas;
  const cropped = new Canvas(
    Math.max(1, Math.ceil((viewport.w - crop.x) * total)),
    Math.max(1, Math.ceil((viewport.h - crop.y) * total)),
  );
  const ctx = cropped.getContext("2d");
  ctx.drawImage(canvas, -Math.round(crop.x * total), -Math.round(crop.y * total));
  return cropped;
}

/**
 * Resolve the render viewport. Headerless renders (`renderHeaders: false`)
 * historically kept a white band where the headers would have been (the pane
 * clip starts at the grid origin); we now render at the natural size and crop
 * the origin band away so the output contains cell content only.
 */
function resolveViewport(
  sheet: RuntimeSheet,
  opts: RenderPngOptions,
  range: ParsedRange | null,
): { viewport: Viewport; crop: { x: number; y: number } | null } {
  if (opts.viewport) return { viewport: opts.viewport, crop: null };
  const headerless = opts.renderHeaders === false;
  const crop = headerless ? originOf(sheet) : null;
  if (range) return { viewport: viewportForRange(sheet, range), crop };
  return { viewport: defaultViewport(sheet, opts), crop };
}

function originOf(sheet: RuntimeSheet): { x: number; y: number } {
  const grid = buildGrid(sheet);
  return { x: grid.originX, y: grid.originY };
}

export async function renderToPng(
  layout: WorkbookLayout,
  opts: RenderPngOptions = {},
): Promise<Buffer> {
  return renderToCanvas(layout, opts).toBuffer("png");
}

async function ensureWasm(): Promise<void> {
  wasmReady ??= initWasm({
    module_or_path: readFileSync(new URL("./xlcore_wasm_bg.wasm", import.meta.url)),
  });
  await wasmReady;
}

function loadOptionsFromRenderOptions(options: RenderPngOptions): LoadWorkbookFromXlsxOptions {
  const rangeSheetName = options.range ? parseRangeRef(options.range).sheetName : undefined;
  return {
    sheetIndex: options.sheetIndex,
    sheetName: options.sheetName ?? rangeSheetName,
  };
}

async function bytesFromInput(input: string | ArrayBuffer | Uint8Array): Promise<Uint8Array> {
  if (typeof input === "string") return readFile(input);
  if (input instanceof Uint8Array) return input;
  return new Uint8Array(input);
}

function pickSheet(
  layout: WorkbookLayout,
  sheetIndex: number | undefined,
  sheetName: string | undefined,
): RuntimeSheet {
  const index = sheetName
    ? layout.sheets.findIndex((s) => s.name === sheetName)
    : (sheetIndex ?? layout.activeSheetIndex ?? 0);
  if (index < 0) throw new Error(`sheet not found: ${sheetName}`);
  const sheet = layout.sheets[index];
  if (!sheet) throw new Error(`sheetIndex out of range: ${index}`);
  return sheet as unknown as RuntimeSheet;
}

/** Hard cap for the auto-grown default viewport, in logical px (pre-scale). */
const MAX_AUTO_VIEWPORT_PX = 4096;
const DRAWING_EDGE_PAD_PX = 8;

function defaultViewport(sheet: RuntimeSheet, opts: RenderPngOptions): Viewport {
  const grid = buildGrid(sheet);
  const headerless = opts.renderHeaders === false;
  const padX = headerless ? grid.originX : 0;
  const padY = headerless ? grid.originY : 0;

  if (opts.width !== undefined || opts.height !== undefined) {
    const w = requirePositive("width", opts.width ?? 1244 - padX);
    const h = requirePositive("height", opts.height ?? 822 - padY);
    return { x: 0, y: 0, w: w + padX, h: h + padY };
  }

  // Grow the historical 1244×822 default to fit drawings (charts, shapes,
  // images) so they are not silently clipped; cap to keep canvases sane.
  let drawingFarX = 0;
  let drawingFarY = 0;
  for (const d of sheet.drawings ?? []) {
    const rect = anchorToRect(d, grid);
    if (!rect) continue;
    drawingFarX = Math.max(drawingFarX, rect.x + rect.w + DRAWING_EDGE_PAD_PX);
    drawingFarY = Math.max(drawingFarY, rect.y + rect.h + DRAWING_EDGE_PAD_PX);
  }
  const cap = MAX_AUTO_VIEWPORT_PX;
  const w = Math.min(grid.totalW, Math.max(1244, Math.ceil(drawingFarX)), cap);
  const h = Math.min(grid.totalH, Math.max(822, Math.ceil(drawingFarY)), cap);
  if (Math.ceil(drawingFarX) > w || Math.ceil(drawingFarY) > h) {
    const warn = opts.onWarning ?? ((m: string) => console.error(m));
    warn(
      `xlsx-preview: sheet "${sheet.name}" has drawings extending to ` +
        `${Math.ceil(drawingFarX)}\u00d7${Math.ceil(drawingFarY)}px, beyond the ` +
        `${cap}px auto-viewport cap; output is clipped. Pass width/height (CLI: ` +
        `--width/--height) or a range to render the full extent.`,
    );
  }
  return { x: 0, y: 0, w, h };
}

function requirePositive(name: string, value: number): number {
  if (!Number.isFinite(value) || value <= 0) {
    throw new Error(`invalid ${name}: ${value} (expected a positive number)`);
  }
  return value;
}

interface ParsedRange {
  sheetName?: string;
  r1: number;
  c1: number;
  r2: number;
  c2: number;
}

function viewportForRange(sheet: RuntimeSheet, range: ParsedRange) {
  const grid = buildGrid(sheet);
  const r1 = clampInt(Math.min(range.r1, range.r2), 1, grid.maxRow);
  const r2 = clampInt(Math.max(range.r1, range.r2), 1, grid.maxRow);
  const c1 = clampInt(Math.min(range.c1, range.c2), 1, grid.maxCol);
  const c2 = clampInt(Math.max(range.c1, range.c2), 1, grid.maxCol);
  const left = (grid.colX[c1] ?? grid.originX) - grid.originX;
  const top = (grid.rowY[r1] ?? grid.originY) - grid.originY;
  const right = (grid.colX[c2 + 1] ?? grid.totalW) - grid.originX;
  const bottom = (grid.rowY[r2 + 1] ?? grid.totalH) - grid.originY;
  return {
    x: left,
    y: top,
    w: grid.originX + Math.max(1, right - left),
    h: grid.originY + Math.max(1, bottom - top),
  };
}

function parseRangeRef(input: string): ParsedRange {
  const trimmed = input.trim();
  if (!trimmed) throw new Error("range cannot be empty");
  const { sheetName, ref } = splitSheetRef(trimmed);
  const parts = ref.replaceAll("$", "").split(":");
  const startText = parts[0];
  if (!startText) throw new Error(`invalid range: ${input}`);
  const endText = parts[1] ?? startText;
  const start = parseCellRef(startText);
  const end = parseCellRef(endText);
  if (!start || !end) throw new Error(`invalid range: ${input}`);
  return { sheetName, r1: start.r, c1: start.c, r2: end.r, c2: end.c };
}

function splitSheetRef(input: string): { sheetName?: string; ref: string } {
  let inQuote = false;
  for (let i = input.length - 1; i >= 0; i--) {
    const ch = input[i];
    if (ch === "'") inQuote = !inQuote;
    if (ch === "!" && !inQuote) {
      return { sheetName: unquoteSheetName(input.slice(0, i)), ref: input.slice(i + 1) };
    }
  }
  return { ref: input };
}

function unquoteSheetName(name: string): string {
  const trimmed = name.trim();
  if (trimmed.startsWith("'") && trimmed.endsWith("'"))
    return trimmed.slice(1, -1).replaceAll("''", "'");
  return trimmed;
}

function parseCellRef(input: string): { r: number; c: number } | null {
  const match = /^([A-Z]+)([1-9][0-9]*)$/i.exec(input.trim());
  if (!match) return null;
  let c = 0;
  for (const ch of match[1]!.toUpperCase()) c = c * 26 + (ch.charCodeAt(0) - 64);
  return { r: Number(match[2]), c };
}

function clampInt(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, Math.floor(value)));
}
