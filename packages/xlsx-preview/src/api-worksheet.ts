import type { WorkbookHandle as WasmWorkbookHandle } from "./xlcore_wasm.js";
import type {
  FreezeInfo,
  SheetInfo,
  SheetPageSetup,
  SheetPageSetupPatch,
  SheetProtectionInfo,
  SheetProtectionPatch,
  SheetVisibility,
  StylePatch,
} from "./api-schema/index.js";
import {
  AutoFilterApi,
  ChartCollection,
  CommentCollection,
  ConditionalFormatCollection,
  DataValidationCollection,
  HyperlinkCollection,
  ImageCollection,
  PivotCollection,
  MergeCollection,
  SparklineGroupCollection,
  TableCollection,
  ThreadedNotesCollection,
} from "./api-collections.js";
import { type CellAddress, type RangeAddress, type SheetRef, qualify } from "./api-refs.js";
import { Cell, Range, makeCell, makeRange } from "./api-range.js";

abstract class SheetScopedApi {
  constructor(
    protected readonly handle: WasmWorkbookHandle,
    protected readonly sheetRef: SheetRef,
  ) {}
  protected get sheet(): string {
    return this.sheetRef.current;
  }
}

export class SheetFreeze extends SheetScopedApi {
  get(): FreezeInfo {
    return this.handle.getFreeze(this.sheet) as FreezeInfo;
  }
  set(frozenRows: number, frozenColumns: number): FreezeInfo {
    return this.handle.setFreeze(this.sheet, frozenRows, frozenColumns) as FreezeInfo;
  }
}

export class SheetPageSetupApi extends SheetScopedApi {
  get(): SheetPageSetup {
    return this.handle.pageSetup(this.sheet) as SheetPageSetup;
  }
  set(patch: SheetPageSetupPatch): SheetPageSetup {
    return this.handle.setPageSetup(this.sheet, patch) as SheetPageSetup;
  }
  clear(): SheetPageSetup {
    return this.handle.removePageSetup(this.sheet) as SheetPageSetup;
  }
}

export class SheetProtection extends SheetScopedApi {
  get(): SheetProtectionInfo | null {
    return (this.handle.sheetProtection(this.sheet) as SheetProtectionInfo | null) ?? null;
  }
  set(patch: SheetProtectionPatch): SheetProtectionInfo {
    return this.handle.setSheetProtection(this.sheet, patch) as SheetProtectionInfo;
  }
  remove(): SheetProtectionInfo | null {
    return (this.handle.removeSheetProtection(this.sheet) as SheetProtectionInfo | null) ?? null;
  }
}

export class Worksheet {
  private readonly sheetRef: SheetRef;
  readonly merges: MergeCollection;
  readonly hyperlinks: HyperlinkCollection;
  readonly comments: CommentCollection;
  readonly threadedNotes: ThreadedNotesCollection;
  readonly dataValidations: DataValidationCollection;
  readonly conditionalFormats: ConditionalFormatCollection;
  readonly autoFilter: AutoFilterApi;
  readonly tables: TableCollection;
  readonly charts: ChartCollection;
  readonly images: ImageCollection;
  readonly sparklineGroups: SparklineGroupCollection;
  readonly pivots: PivotCollection;
  readonly freeze: SheetFreeze;
  readonly pageSetup: SheetPageSetupApi;
  readonly protection: SheetProtection;

  constructor(
    private readonly handle: WasmWorkbookHandle,
    initialName: string,
  ) {
    this.sheetRef = { current: initialName };
    this.merges = new MergeCollection(handle, this.sheetRef);
    this.hyperlinks = new HyperlinkCollection(handle, this.sheetRef);
    this.comments = new CommentCollection(handle, this.sheetRef);
    this.threadedNotes = new ThreadedNotesCollection(handle, this.sheetRef);
    this.dataValidations = new DataValidationCollection(handle, this.sheetRef);
    this.conditionalFormats = new ConditionalFormatCollection(handle, this.sheetRef);
    this.autoFilter = new AutoFilterApi(handle, this.sheetRef);
    this.tables = new TableCollection(handle, this.sheetRef);
    this.charts = new ChartCollection(handle, this.sheetRef);
    this.images = new ImageCollection(handle, this.sheetRef);
    this.sparklineGroups = new SparklineGroupCollection(handle, this.sheetRef);
    this.pivots = new PivotCollection(handle, this.sheetRef);
    this.freeze = new SheetFreeze(handle, this.sheetRef);
    this.pageSetup = new SheetPageSetupApi(handle, this.sheetRef);
    this.protection = new SheetProtection(handle, this.sheetRef);
  }

  get name(): string {
    return this.sheetRef.current;
  }

  info(): SheetInfo {
    const found = (this.handle.sheets() as SheetInfo[]).find((s) => s.name === this.name);
    if (!found) {
      throw new Error(`worksheet '${this.name}' no longer exists`);
    }
    return found;
  }

  get index(): number {
    return this.info().index;
  }
  get rowCount(): number {
    return this.info().rowCount;
  }
  get columnCount(): number {
    return this.info().columnCount;
  }
  get visibility(): SheetVisibility | undefined {
    return this.info().state;
  }
  get active(): boolean {
    return this.info().active;
  }

  range(addr: RangeAddress): Range {
    return makeRange(this.handle, this.sheetRef, addr);
  }

  cell(addr: CellAddress): Cell {
    return makeCell(this.handle, this.sheetRef, addr);
  }

  /**
   * Bulk-apply a map (or `[ref, patch]` iterable) of styles to this sheet.
   *
   * Each entry is forwarded to the underlying `setStyle`, so the same merged-cell
   * caveat applies: target the merge's **top-left anchor** rather than the full
   * merged range — OOXML only stores style on the anchor.
   */
  setStyles(patches: Record<string, StylePatch> | Iterable<[string, StylePatch]>): this {
    const entries: Array<[string, StylePatch]> =
      typeof (patches as Iterable<[string, StylePatch]>)[Symbol.iterator] === "function"
        ? Array.from(patches as Iterable<[string, StylePatch]>)
        : Object.entries(patches as Record<string, StylePatch>);
    for (const [ref, patch] of entries) {
      if (typeof ref !== "string" || ref.length === 0) {
        throw new TypeError(`setStyles: reference must be a non-empty string, got ${String(ref)}`);
      }
      if (patch === null || typeof patch !== "object") {
        throw new TypeError(`setStyles: patch for '${ref}' must be a StylePatch object`);
      }
      this.handle.setStyle(qualify(this.name, ref), patch);
    }
    return this;
  }

  setShowGridLines(visible: boolean): this {
    this.handle.setShowGridLines(this.name, visible);
    return this;
  }
  getShowGridLines(): boolean {
    return this.handle.getShowGridLines(this.name) as boolean;
  }
  /** @param row 1-based row index (row 1 = A1's row). */
  setRowHeight(row: number, height: number): this {
    this.handle.setRowHeight(this.name, row, height);
    return this;
  }
  /** @param row 1-based row index (row 1 = A1's row). */
  setRowVisible(row: number, visible: boolean): this {
    this.handle.setRowVisible(this.name, row, visible);
    return this;
  }
  /** @param column 1-based column index (column 1 = A). */
  setColumnWidth(column: number, width: number): this {
    this.handle.setColumnWidth(this.name, column, width);
    return this;
  }
  /** @param column 1-based column index (column 1 = A). */
  setColumnVisible(column: number, visible: boolean): this {
    this.handle.setColumnVisible(this.name, column, visible);
    return this;
  }
  /** @param before 1-based row index to insert before (row 1 = A1's row). */
  insertRows(before: number, count: number): this {
    this.handle.insertRows(this.name, before, count);
    return this;
  }
  /** @param start 1-based row index of the first row to delete (row 1 = A1's row). */
  deleteRows(start: number, count: number): this {
    this.handle.deleteRows(this.name, start, count);
    return this;
  }
  /** @param before 1-based column index to insert before (column 1 = A). */
  insertColumns(before: number, count: number): this {
    this.handle.insertColumns(this.name, before, count);
    return this;
  }
  /** @param start 1-based column index of the first column to delete (column 1 = A). */
  deleteColumns(start: number, count: number): this {
    this.handle.deleteColumns(this.name, start, count);
    return this;
  }

  setVisibility(visibility: SheetVisibility): SheetInfo {
    return this.handle.setSheetVisibility(this.name, visibility) as SheetInfo;
  }

  activate(): SheetInfo {
    return this.handle.setActiveSheet(this.name) as SheetInfo;
  }

  rename(newName: string): this {
    this.handle.renameSheet(this.name, newName);
    this.sheetRef.current = newName;
    return this;
  }

  moveTo(toIndex: number): SheetInfo {
    return this.handle.moveSheet(this.name, toIndex) as SheetInfo;
  }

  remove(): void {
    this.handle.deleteSheet(this.name);
  }
}
