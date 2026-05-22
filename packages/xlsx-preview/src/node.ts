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
import { setOffscreenCanvasFactory } from "./canvasFactory.js";
import type { RenderOptions } from "./renderTypes.js";
import type { Sheet as RuntimeSheet, WorkbookLayout } from "./types.js";

export interface RenderPngOptions extends RenderOptions {
  sheetIndex?: number;
  sheetName?: string;

  range?: string;
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

/**
 * Options accepted by {@link loadWorkbookFromCsv}. All fields are optional;
 * delimiter defaults to a sniff over the first line.
 */
export interface LoadWorkbookFromCsvOptions {
  /** Single byte (`,`, `\t`, `;`, `|`) or the literal `"tab"`. */
  delimiter?: string;
  /** Hard cap on rendered rows. Triggers a `report.warnings` entry. */
  maxRows?: number;
  /** Sheet name shown in the renderer's tab strip. */
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

/** Options accepted by {@link loadWorkbookFromParquet}. */
export interface LoadWorkbookFromParquetOptions {
  /** Hard cap on rendered rows, including the synthetic header row. */
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
  const viewport =
    opts.viewport ?? (range ? viewportForRange(sheet, range) : defaultViewport(sheet, opts));
  const canvas = new Canvas(
    Math.ceil(viewport.w * (opts.zoom ?? 1) * (opts.scale ?? 1)),
    Math.ceil(viewport.h * (opts.zoom ?? 1) * (opts.scale ?? 1)),
  );
  render(canvas as unknown as Parameters<typeof render>[0], sheet, layout, {
    ...opts,
    viewport,
  });
  return canvas;
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

function defaultViewport(sheet: RuntimeSheet, opts: RenderOptions) {
  const grid = buildGrid(sheet);
  return {
    x: 0,
    y: 0,
    w: Math.min(grid.totalW, opts.renderHeaders === false ? 1200 : 1244),
    h: Math.min(grid.totalH, opts.renderHeaders === false ? 800 : 822),
  };
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
