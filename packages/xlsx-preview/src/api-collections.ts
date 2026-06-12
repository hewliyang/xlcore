import type { WorkbookHandle as WasmWorkbookHandle } from "./xlcore_wasm.js";
import type {
  AutoFilterColumnInfo,
  AutoFilterColumnPatch,
  AutoFilterCustomCriterion,
  AutoFilterInfo,
  CalcProperties,
  CalcPropertiesPatch,
  ChartInfo,
  ChartPatch,
  ChartUpdate,
  CommentInfo,
  CommentPatch,
  ConditionalFormatRuleInfo,
  ConditionalFormatRulePatch,
  DataValidationInfo,
  DataValidationPatch,
  DefinedNameInfo,
  DefinedNamePatch,
  HyperlinkInfo,
  HyperlinkPatch,
  ImageInfo,
  ImagePatch,
  MergeInfo,
  ShapeInfo,
  ShapePatch,
  PivotGrid,
  PivotInfo,
  PivotPatch,
  PivotUpdate,
  SparklineGroupInfo,
  SparklineGroupPatch,
  TableInfo,
  TablePatch,
  ThreadedNoteInfo,
  ThreadedNotePatch,
  WorkbookProperties,
  WorkbookPropertiesPatch,
  WorkbookProtectionInfo,
  WorkbookProtectionPatch,
} from "./api-schema/index.js";
import { type SheetRef, qualify } from "./api-refs.js";

abstract class SheetScopedCollection {
  constructor(
    protected readonly handle: WasmWorkbookHandle,
    protected readonly sheetRef: SheetRef,
  ) {}
  protected get sheet(): string {
    return this.sheetRef.current;
  }
  protected qref(ref: string): string {
    return qualify(this.sheet, ref);
  }
}

export class MergeCollection extends SheetScopedCollection {
  list(): MergeInfo[] {
    return this.handle.merges(this.sheet) as MergeInfo[];
  }
  add(ref: string): MergeInfo {
    return this.handle.addMerge(this.qref(ref)) as MergeInfo;
  }
  remove(ref: string): MergeInfo | null {
    return (this.handle.removeMerge(this.qref(ref)) as MergeInfo | null) ?? null;
  }
}

export class HyperlinkCollection extends SheetScopedCollection {
  list(): HyperlinkInfo[] {
    return this.handle.hyperlinks(this.sheet) as HyperlinkInfo[];
  }
  set(ref: string, patch: HyperlinkPatch): HyperlinkInfo {
    return this.handle.setHyperlink(this.qref(ref), patch) as HyperlinkInfo;
  }
  remove(ref: string): HyperlinkInfo[] {
    return this.handle.removeHyperlink(this.qref(ref)) as HyperlinkInfo[];
  }
}

export class CommentCollection extends SheetScopedCollection {
  list(): CommentInfo[] {
    return this.handle.comments(this.sheet) as CommentInfo[];
  }
  set(ref: string, patch: CommentPatch): CommentInfo {
    return this.handle.setComment(this.qref(ref), patch) as CommentInfo;
  }
  remove(ref: string): CommentInfo[] {
    return this.handle.removeComment(this.qref(ref)) as CommentInfo[];
  }
}

export class ThreadedNotesCollection extends SheetScopedCollection {
  list(): ThreadedNoteInfo[] {
    return this.handle.threadedNotes(this.sheet) as ThreadedNoteInfo[];
  }
  add(ref: string, patch: ThreadedNotePatch): ThreadedNoteInfo {
    return this.handle.addThreadedNote(this.qref(ref), patch) as ThreadedNoteInfo;
  }
  reply(parentId: string, patch: ThreadedNotePatch): ThreadedNoteInfo {
    return this.handle.replyThreadedNote(parentId, patch) as ThreadedNoteInfo;
  }
  removeThread(ref: string): ThreadedNoteInfo[] {
    return this.handle.removeThreadedThread(this.qref(ref)) as ThreadedNoteInfo[];
  }
}

export class DataValidationCollection extends SheetScopedCollection {
  list(): DataValidationInfo[] {
    return this.handle.dataValidations(this.sheet) as DataValidationInfo[];
  }
  set(ref: string, patch: DataValidationPatch): DataValidationInfo {
    return this.handle.setDataValidation(this.qref(ref), patch) as DataValidationInfo;
  }
  remove(ref: string): DataValidationInfo[] {
    return this.handle.removeDataValidation(this.qref(ref)) as DataValidationInfo[];
  }
}

export class ConditionalFormatCollection extends SheetScopedCollection {
  list(): ConditionalFormatRuleInfo[] {
    return this.handle.conditionalFormats(this.sheet) as ConditionalFormatRuleInfo[];
  }
  set(ref: string, patch: ConditionalFormatRulePatch): ConditionalFormatRuleInfo {
    return this.handle.setConditionalFormat(this.qref(ref), patch) as ConditionalFormatRuleInfo;
  }
  clear(ref: string): ConditionalFormatRuleInfo[] {
    return this.handle.clearConditionalFormats(this.qref(ref)) as ConditionalFormatRuleInfo[];
  }
}

export class AutoFilterApi extends SheetScopedCollection {
  get(): AutoFilterInfo | null {
    return (this.handle.autoFilter(this.sheet) as AutoFilterInfo | null) ?? null;
  }
  set(ref: string): AutoFilterInfo {
    return this.handle.setAutoFilter(this.qref(ref)) as AutoFilterInfo;
  }
  remove(): AutoFilterInfo | null {
    return (this.handle.removeAutoFilter(this.sheet) as AutoFilterInfo | null) ?? null;
  }
  setColumn(patch: AutoFilterColumnPatch): AutoFilterColumnInfo {
    return this.handle.setAutoFilterColumn(this.sheet, patch) as AutoFilterColumnInfo;
  }
  setColumnValues(
    columnOffset: number,
    values: string[],
    opts: { blank?: boolean; hiddenButton?: boolean; showButton?: boolean } = {},
  ): AutoFilterColumnInfo {
    return this.setColumn({
      columnOffset,
      hiddenButton: opts.hiddenButton,
      showButton: opts.showButton,
      criteria: { kind: "values", values, blank: opts.blank },
    });
  }
  setColumnTop10(
    columnOffset: number,
    val: number,
    opts: { top?: boolean; percent?: boolean; hiddenButton?: boolean; showButton?: boolean } = {},
  ): AutoFilterColumnInfo {
    return this.setColumn({
      columnOffset,
      hiddenButton: opts.hiddenButton,
      showButton: opts.showButton,
      criteria: { kind: "top10", top: opts.top, percent: opts.percent, val },
    });
  }
  setColumnCustom(
    columnOffset: number,
    criteria: AutoFilterCustomCriterion[],
    opts: { logicalAnd?: boolean; hiddenButton?: boolean; showButton?: boolean } = {},
  ): AutoFilterColumnInfo {
    return this.setColumn({
      columnOffset,
      hiddenButton: opts.hiddenButton,
      showButton: opts.showButton,
      criteria: { kind: "custom", logical_and: opts.logicalAnd, criteria },
    });
  }
  removeColumn(columnOffset: number): AutoFilterColumnInfo | null {
    return (
      (this.handle.removeAutoFilterColumn(
        this.sheet,
        columnOffset,
      ) as AutoFilterColumnInfo | null) ?? null
    );
  }
}

export class TableCollection extends SheetScopedCollection {
  list(): TableInfo[] {
    return this.handle.tables(this.sheet) as TableInfo[];
  }
  set(patch: TablePatch): TableInfo {
    const qualified: TablePatch = patch.reference
      ? { ...patch, reference: this.qref(patch.reference) }
      : patch;
    return this.handle.setTable(qualified) as TableInfo;
  }
  remove(name: string): TableInfo | null {
    return (this.handle.removeTable(name) as TableInfo | null) ?? null;
  }
}

export class ChartCollection extends SheetScopedCollection {
  list(): ChartInfo[] {
    return this.handle.charts(this.sheet) as ChartInfo[];
  }
  set(patch: ChartPatch): ChartInfo {
    return this.handle.setChart(patch) as ChartInfo;
  }
  update(id: string, update: ChartUpdate): ChartInfo {
    return this.handle.updateChart(this.sheet, id, update) as ChartInfo;
  }
  remove(id: string): ChartInfo | null {
    return (this.handle.removeChart(this.sheet, id) as ChartInfo | null) ?? null;
  }
}

export class ImageCollection extends SheetScopedCollection {
  list(): ImageInfo[] {
    return this.handle.images(this.sheet) as ImageInfo[];
  }
  set(patch: ImagePatch): ImageInfo {
    return this.handle.setImage(patch) as ImageInfo;
  }
  remove(id: string): ImageInfo | null {
    return (this.handle.removeImage(this.sheet, id) as ImageInfo | null) ?? null;
  }
}

export class ShapeCollection extends SheetScopedCollection {
  list(): ShapeInfo[] {
    return this.handle.shapes(this.sheet) as ShapeInfo[];
  }
  set(patch: Omit<ShapePatch, "sheet">): ShapeInfo {
    return this.handle.setShape({ ...patch, sheet: this.sheet }) as ShapeInfo;
  }
  remove(id: string): ShapeInfo | null {
    return (this.handle.removeShape(this.sheet, id) as ShapeInfo | null) ?? null;
  }
}

export class SparklineGroupCollection extends SheetScopedCollection {
  list(): SparklineGroupInfo[] {
    return this.handle.sparklineGroups(this.sheet) as SparklineGroupInfo[];
  }
  set(patch: SparklineGroupPatch): SparklineGroupInfo {
    return this.handle.setSparklineGroup(patch) as SparklineGroupInfo;
  }
  remove(id: string): SparklineGroupInfo | null {
    return (this.handle.removeSparklineGroup(this.sheet, id) as SparklineGroupInfo | null) ?? null;
  }
}

export class PivotCollection extends SheetScopedCollection {
  list(): PivotInfo[] {
    return this.handle.pivots(this.sheet) as PivotInfo[];
  }
  set(patch: Omit<PivotPatch, "sheet">): PivotInfo {
    return this.handle.setPivot({ ...patch, sheet: this.sheet }) as PivotInfo;
  }
  preview(patch: Omit<PivotPatch, "sheet">): PivotGrid {
    return this.handle.pivotPreview({ ...patch, sheet: this.sheet }) as PivotGrid;
  }
  update(id: string, update: PivotUpdate): PivotInfo {
    return this.handle.updatePivot(this.sheet, id, update) as PivotInfo;
  }
  remove(id: string): PivotInfo | null {
    return (this.handle.removePivot(this.sheet, id) as PivotInfo | null) ?? null;
  }
}

export class WorkbookPivots {
  constructor(private readonly handle: WasmWorkbookHandle) {}
  list(): PivotInfo[] {
    return this.handle.pivots(null) as PivotInfo[];
  }
}

export class DefinedNamesCollection {
  constructor(private readonly handle: WasmWorkbookHandle) {}
  list(): DefinedNameInfo[] {
    return this.handle.definedNames() as DefinedNameInfo[];
  }
  set(patch: DefinedNamePatch): DefinedNameInfo {
    return this.handle.setDefinedName(patch) as DefinedNameInfo;
  }
  remove(name: string, scope?: string | null): DefinedNameInfo | null {
    return (this.handle.removeDefinedName(name, scope ?? null) as DefinedNameInfo | null) ?? null;
  }
}

export class WorkbookTables {
  constructor(private readonly handle: WasmWorkbookHandle) {}
  list(): TableInfo[] {
    return this.handle.tables(null) as TableInfo[];
  }
  set(patch: TablePatch): TableInfo {
    return this.handle.setTable(patch) as TableInfo;
  }
  remove(name: string): TableInfo | null {
    return (this.handle.removeTable(name) as TableInfo | null) ?? null;
  }
}

export class WorkbookCharts {
  constructor(private readonly handle: WasmWorkbookHandle) {}
  list(): ChartInfo[] {
    return this.handle.charts(null) as ChartInfo[];
  }
}

export class WorkbookImages {
  constructor(private readonly handle: WasmWorkbookHandle) {}
  list(): ImageInfo[] {
    return this.handle.images(null) as ImageInfo[];
  }
}

export class WorkbookSparklineGroups {
  constructor(private readonly handle: WasmWorkbookHandle) {}
  list(): SparklineGroupInfo[] {
    return this.handle.sparklineGroups(null) as SparklineGroupInfo[];
  }
}

export class WorkbookPropertiesApi {
  constructor(private readonly handle: WasmWorkbookHandle) {}
  get(): WorkbookProperties {
    return this.handle.properties() as WorkbookProperties;
  }
  set(patch: WorkbookPropertiesPatch): WorkbookProperties {
    return this.handle.setProperties(patch) as WorkbookProperties;
  }
}

export class CalcPropertiesApi {
  constructor(private readonly handle: WasmWorkbookHandle) {}
  get(): CalcProperties {
    return this.handle.calcProperties() as CalcProperties;
  }
  set(patch: CalcPropertiesPatch): CalcProperties {
    return this.handle.setCalcProperties(patch) as CalcProperties;
  }
}

export class WorkbookProtection {
  constructor(private readonly handle: WasmWorkbookHandle) {}
  get(): WorkbookProtectionInfo | null {
    return (this.handle.workbookProtection() as WorkbookProtectionInfo | null) ?? null;
  }
  set(patch: WorkbookProtectionPatch): WorkbookProtectionInfo {
    return this.handle.setWorkbookProtection(patch) as WorkbookProtectionInfo;
  }
  remove(): WorkbookProtectionInfo | null {
    return (this.handle.removeWorkbookProtection() as WorkbookProtectionInfo | null) ?? null;
  }
}
