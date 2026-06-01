import init, { WorkbookHandle as WasmWorkbookHandle } from "./xlcore_wasm.js";
import {
  CalcPropertiesApi,
  DefinedNamesCollection,
  WorkbookCharts,
  WorkbookImages,
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
  SparklineGroupCollection,
  TableCollection,
  ThreadedNotesCollection,
  WorkbookCharts,
  WorkbookImages,
  WorkbookPropertiesApi,
  WorkbookProtection,
  WorkbookSparklineGroups,
  WorkbookTables,
} from "./api-collections.js";
export { SheetFreeze, SheetPageSetupApi, SheetProtection } from "./api-worksheet.js";
export type { CellAddress, RangeAddress } from "./api-refs.js";

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
  PrintCellComments,
  PrintErrors,
  PrintOptionsInfo,
  PrintOptionsPatch,
  SheetInfo,
  SheetPageSetup,
  SheetPageSetupPatch,
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

export interface WorkbookApiOptions {
  wasmBinaryUrl?: string | URL | RequestInfo | BufferSource | WebAssembly.Module;
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
  readonly properties: WorkbookPropertiesApi;
  readonly calcProperties: CalcPropertiesApi;
  readonly protection: WorkbookProtection;

  private constructor(private handle: WasmWorkbookHandle) {
    this.definedNames = new DefinedNamesCollection(handle);
    this.allTables = new WorkbookTables(handle);
    this.allCharts = new WorkbookCharts(handle);
    this.allImages = new WorkbookImages(handle);
    this.allSparklineGroups = new WorkbookSparklineGroups(handle);
    this.properties = new WorkbookPropertiesApi(handle);
    this.calcProperties = new CalcPropertiesApi(handle);
    this.protection = new WorkbookProtection(handle);
  }

  sheet(name: string): Worksheet {
    const exists = (this.handle.sheets() as SheetInfo[]).some((s) => s.name === name);
    if (!exists) {
      throw new Error(`worksheet '${name}' does not exist`);
    }
    return new Worksheet(this.handle, name);
  }

  worksheets(): Worksheet[] {
    return (this.handle.sheets() as SheetInfo[]).map(
      (s) => new Worksheet(this.handle, s.name),
    );
  }

  activeSheet(): Worksheet {
    const infos = this.handle.sheets() as SheetInfo[];
    const active = infos.find((s) => s.active) ?? infos[0];
    if (!active) {
      throw new Error("workbook has no sheets");
    }
    return new Worksheet(this.handle, active.name);
  }

  addSheet(name: string): Worksheet {
    this.handle.createSheet(name);
    return new Worksheet(this.handle, name);
  }

  removeSheet(name: string): void {
    this.handle.deleteSheet(name);
  }

  warnings(): ApiWarning[] {
    return this.handle.warnings() as ApiWarning[];
  }

  takeWarnings(): ApiWarning[] {
    return this.handle.takeWarnings() as ApiWarning[];
  }

  search(query: string, options: Partial<SearchOptions> = {}): SearchMatch[] {
    const full: SearchOptions = {
      sheet: options.sheet,
      range: options.range,
      target: options.target ?? "values",
      mode: options.mode ?? "substring",
      caseSensitive: options.caseSensitive ?? false,
      maxResults: options.maxResults,
    };
    return this.handle.search(query, full) as SearchMatch[];
  }

  recalculate(): RecalcWorkbook {
    return this.handle.recalculate() as RecalcWorkbook;
  }

  layout(options: WorkbookLayoutOptions = {}): WorkbookLayout {
    return this.handle.layout(options) as WorkbookLayout;
  }

  save(): Uint8Array {
    return this.handle.save();
  }

  dispose(): void {
    this.handle.dispose();
  }
}

async function ensureWasm(options: WorkbookApiOptions): Promise<void> {
  wasmReady ??= init({
    module_or_path: options.wasmBinaryUrl ?? DEFAULT_WASM_BINARY_URL,
  }).then(() => undefined);
  await wasmReady;
}

function toUint8Array(bytes: ArrayBuffer | Uint8Array): Uint8Array {
  if (bytes instanceof Uint8Array) {
    return bytes;
  }
  return new Uint8Array(bytes);
}

