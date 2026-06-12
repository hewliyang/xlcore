import init, { WorkbookHandle as WasmWorkbookHandle } from "./xlcore_wasm.js";
import {
  CalcPropertiesApi,
  DefinedNamesCollection,
  WorkbookCharts,
  WorkbookImages,
  WorkbookPivots,
  WorkbookPropertiesApi,
  WorkbookProtection,
  WorkbookSparklineGroups,
  WorkbookTables,
} from "./api-collections.js";
import { Worksheet } from "./api-worksheet.js";
import type {
  ApiWarning,
  LayoutOptions as WorkbookLayoutOptions,
  RecalcWorkbook,
  SearchMatch,
  SearchOptions,
  SheetInfo,
} from "./api-schema/index.js";
import type { WorkbookLayout } from "./types.js";

export { Cell, Range } from "./api-range.js";
export { Worksheet } from "./api-worksheet.js";
export {
  AutoFilterApi,
  CalcPropertiesApi,
  ChartCollection,
  CommentCollection,
  ConditionalFormatCollection,
  DataValidationCollection,
  DefinedNamesCollection,
  HyperlinkCollection,
  ImageCollection,
  MergeCollection,
  PivotCollection,
  SparklineGroupCollection,
  TableCollection,
  ThreadedNotesCollection,
  WorkbookCharts,
  WorkbookImages,
  WorkbookPivots,
  WorkbookPropertiesApi,
  WorkbookProtection,
  WorkbookSparklineGroups,
  WorkbookTables,
} from "./api-collections.js";
export {
  SheetFreeze,
  SheetPageSetupApi,
  SheetPropertiesApi,
  SheetProtection,
} from "./api-worksheet.js";
export { NumberFormat } from "./number-formats.js";
export type { NumberFormatCode, NumberFormatKey } from "./number-formats.js";
export { absoluteAnchor, anchorA1, cellA1, colLetter, rangeA1 } from "./api-refs.js";
export { distinctValuesFor } from "./pivotSource.js";
export type { AbsoluteAnchorOptions, AnchorA1, CellAddress, RangeAddress } from "./api-refs.js";

export type {
  AlignmentPatch,
  ApiCellValue,
  ApiError,
  ApiError as ApiErrorPayload,
  ApiErrorCode,
  ApiWarning,
  AutoFilterColumnInfo,
  AutoFilterColumnPatch,
  AutoFilterCriteria,
  AutoFilterCustomCriterion,
  AutoFilterInfo,
  AutoFilterOperator,
  BorderLinePatch,
  BorderLineStyle,
  BorderPatch,
  CalcMode,
  CalcProperties,
  CalcPropertiesPatch,
  CellInfo,
  CfOperator,
  CfRuleKind,
  ChartAnchor,
  ChartInfo,
  ChartKind,
  ChartLegendPosition,
  ChartPatch,
  ChartSeriesInfo,
  ChartSeriesPatch,
  ClearMode,
  CommentInfo,
  CommentPatch,
  ConditionalFormatRuleInfo,
  ConditionalFormatRulePatch,
  ThreadedNoteInfo,
  ThreadedNotePatch,
  DataValidationErrorStyle,
  DataValidationInfo,
  DataValidationOperator,
  DataValidationPatch,
  DataValidationType,
  DefinedNameInfo,
  DefinedNamePatch,
  DependencyInfo,
  DependencyReference,
  EngineCellValue,
  FillPatch,
  FontPatch,
  FormulaFallback,
  FreezeInfo,
  HorizontalAlign,
  HyperlinkInfo,
  HyperlinkPatch,
  ImageFormat,
  ImageInfo,
  ImagePatch,
  LayoutOptions as WorkbookLayoutOptions,
  MergeInfo,
  RangeInfo,
  RecalcCell,
  RecalcSheet,
  RecalcWorkbook,
  SearchHit,
  SearchMatch,
  SearchMode,
  SearchOptions,
  SearchTarget,
  HeaderFooterInfo,
  HeaderFooterPatch,
  PageMarginsInfo,
  PageMarginsPatch,
  PageOrder,
  PageOrientation,
  PageSetupSettings,
  PageSetupSettingsPatch,
  PatternType,
  PivotAggregation,
  PivotCellRole,
  PivotDataField,
  PivotFieldFilter,
  PivotGrid,
  PivotGridCell,
  PivotInfo,
  PivotPatch,
  PrintCellComments,
  PrintErrors,
  PrintOptionsInfo,
  PrintOptionsPatch,
  SheetInfo,
  SheetPageSetup,
  SheetPageSetupPatch,
  SheetProperties,
  SheetPropertiesPatch,
  SheetProtectionInfo,
  SheetProtectionPatch,
  SheetVisibility,
  SparklineAxisType,
  SparklineDisplayBlanks,
  SparklineEntry,
  SparklineGroupInfo,
  SparklineGroupPatch,
  SparklineKind,
  StylePatch,
  TableColumnInfo,
  TableColumnPatch,
  TableInfo,
  TablePatch,
  TableStylePatch,
  TableStyleSettings,
  TableTotalsFunction,
  UnderlinePatch,
  VerticalAlign,
  WorkbookProperties,
  WorkbookPropertiesPatch,
  WorkbookProtectionInfo,
  WorkbookProtectionPatch,
} from "./api-schema/index.js";

const DEFAULT_WASM_BINARY_URL = new URL("./xlcore_wasm_bg.wasm", import.meta.url).href;

let wasmReady: Promise<void> | null = null;

function isNode(): boolean {
  return (
    typeof process !== "undefined" &&
    process.versions != null &&
    typeof process.versions.node === "string"
  );
}

async function resolveDefaultWasmInput(): Promise<
  string | URL | RequestInfo | BufferSource | WebAssembly.Module
> {
  if (isNode()) {
    const { readFileSync } = await import("node:fs");
    return readFileSync(new URL("./xlcore_wasm_bg.wasm", import.meta.url));
  }
  return DEFAULT_WASM_BINARY_URL;
}

export interface WorkbookApiOptions {
  wasmBinaryUrl?: string | URL | RequestInfo | BufferSource | WebAssembly.Module;
}

export interface RecalcOptions {
  errorsOnly?: boolean;
}

export type { CellInput } from "./api-range.js";

export class Workbook {
  static async create(options: WorkbookApiOptions = {}): Promise<Workbook> {
    await ensureWasm(options);
    return new Workbook(new WasmWorkbookHandle());
  }

  static async open(
    bytes: ArrayBuffer | Uint8Array,
    options: WorkbookApiOptions = {},
  ): Promise<Workbook> {
    await ensureWasm(options);
    return new Workbook(WasmWorkbookHandle.open(toUint8Array(bytes)));
  }

  readonly definedNames: DefinedNamesCollection;
  readonly allTables: WorkbookTables;
  readonly allCharts: WorkbookCharts;
  readonly allImages: WorkbookImages;
  readonly allSparklineGroups: WorkbookSparklineGroups;
  readonly allPivots: WorkbookPivots;
  readonly properties: WorkbookPropertiesApi;
  readonly calcProperties: CalcPropertiesApi;
  readonly protection: WorkbookProtection;

  private readonly worksheetCache = new Map<number, Worksheet>();

  private constructor(private handle: WasmWorkbookHandle) {
    this.definedNames = new DefinedNamesCollection(handle);
    this.allTables = new WorkbookTables(handle);
    this.allCharts = new WorkbookCharts(handle);
    this.allImages = new WorkbookImages(handle);
    this.allSparklineGroups = new WorkbookSparklineGroups(handle);
    this.allPivots = new WorkbookPivots(handle);
    this.properties = new WorkbookPropertiesApi(handle);
    this.calcProperties = new CalcPropertiesApi(handle);
    this.protection = new WorkbookProtection(handle);
  }

  /**
   * Resolve a stable {@link Worksheet} object for a sheet info. Worksheets are
   * cached by their stable `SheetInfo.id`, so `wb.sheet('X')` twice returns the
   * same object and a `rename()` on one handle is visible through every other.
   */
  private worksheetFor(info: SheetInfo): Worksheet {
    const cached = this.worksheetCache.get(info.id);
    if (cached) {
      cached.syncName(info.name);
      return cached;
    }
    const ws = new Worksheet(this.handle, info.name);
    this.worksheetCache.set(info.id, ws);
    return ws;
  }

  sheet(name: string): Worksheet {
    const info = (this.handle.sheets() as SheetInfo[]).find((s) => s.name === name);
    if (!info) {
      throw new Error(`worksheet '${name}' does not exist`);
    }
    return this.worksheetFor(info);
  }

  worksheets(): Worksheet[] {
    return (this.handle.sheets() as SheetInfo[]).map((info) => this.worksheetFor(info));
  }

  activeSheet(): Worksheet {
    const infos = this.handle.sheets() as SheetInfo[];
    const active = infos.find((s) => s.active) ?? infos[0];
    if (!active) {
      throw new Error("workbook has no sheets");
    }
    return this.worksheetFor(active);
  }

  addSheet(name: string): Worksheet {
    this.handle.createSheet(name);
    return this.sheet(name);
  }

  removeSheet(name: string): void {
    const info = (this.handle.sheets() as SheetInfo[]).find((s) => s.name === name);
    this.handle.deleteSheet(name);
    if (info) {
      this.worksheetCache.delete(info.id);
    }
  }

  warnings(): ApiWarning[] {
    return this.handle.warnings() as ApiWarning[];
  }

  takeWarnings(): ApiWarning[] {
    return this.handle.takeWarnings() as ApiWarning[];
  }

  /**
   * Search cells across the workbook.
   *
   * Defaults: `target: "values"`, `mode: "substring"`, `caseSensitive: false`,
   * `includeHidden: true` (hidden and very-hidden sheets are searched by
   * default; set `includeHidden: false` to skip them, unless an explicit
   * `sheet` is named — a named sheet is always searched regardless of
   * visibility).
   */
  search(query: string, options: Partial<SearchOptions> = {}): SearchMatch[] {
    return this.handle.search(query, options) as SearchMatch[];
  }

  /**
   * Recalculate all formulas. By default the returned report lists only cells
   * that produced an engine error (those with a `fallback`), dropping sheets
   * with no errors — recalc always evaluates every cell, so read computed
   * values via `Cell.info()` / `Range.values()`. Pass `{ errorsOnly: false }`
   * to get the full cell-by-cell report.
   */
  recalculate(options: RecalcOptions = {}): RecalcWorkbook {
    return this.handle.recalculate(options.errorsOnly ?? true) as RecalcWorkbook;
  }

  layout(options: WorkbookLayoutOptions = {}): WorkbookLayout {
    return this.handle.layout(options) as WorkbookLayout;
  }

  save(): Uint8Array {
    return this.handle.save();
  }

  /**
   * Escape hatch: list every part path in the package (the modeled object
   * graph serialized to OPC), e.g. `xl/workbook.xml`. Use with
   * {@link getPart}/{@link setPart} to read or author schema we don't model.
   */
  partNames(): string[] {
    return this.handle.partNames() as string[];
  }

  /**
   * Escape hatch: read a part's raw XML by path, or `undefined` if absent.
   * Leading `/` is optional. Throws on non-UTF-8 (binary) parts.
   */
  getPart(name: string): string | undefined {
    return (this.handle.getPartXml(name) as string | null) ?? undefined;
  }

  /**
   * Escape hatch: write a part's raw XML, overwriting or creating it. For a
   * brand-new part path you must also declare its content type by editing
   * `[Content_Types].xml` via this same method. Round-trips verbatim.
   */
  setPart(name: string, xml: string): void {
    this.handle.setPartXml(name, xml);
  }

  /** Escape hatch: delete a part by path. Returns whether it existed. */
  removePart(name: string): boolean {
    return this.handle.removePartXml(name) as boolean;
  }

  dispose(): void {
    this.handle.dispose();
  }
}

async function ensureWasm(options: WorkbookApiOptions): Promise<void> {
  wasmReady ??= (async () => {
    const module_or_path = options.wasmBinaryUrl ?? (await resolveDefaultWasmInput());
    await init({ module_or_path });
  })();
  await wasmReady;
}

function toUint8Array(bytes: ArrayBuffer | Uint8Array): Uint8Array {
  if (bytes instanceof Uint8Array) {
    return bytes;
  }
  return new Uint8Array(bytes);
}
