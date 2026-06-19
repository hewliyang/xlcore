import type { WorkbookHandle as WasmWorkbookHandle } from "./xlcore_wasm.js";
import type {
  ApiCellValue,
  CellInfo,
  ClearMode,
  CommentInfo,
  CommentPatch,
  DependencyInfo,
  DependencyReference,
  HyperlinkInfo,
  HyperlinkPatch,
  RangeInfo,
  RichText,
  RichTextRun,
  StylePatch,
} from "./api-schema/index.js";
import {
  type CellAddress,
  type RangeAddress,
  type SheetRef,
  resolveCell,
  resolveRange,
} from "./api-refs.js";

export type CellInput = string | number | boolean | null | ApiCellValue;

export class Range {
  constructor(
    protected readonly handle: WasmWorkbookHandle,
    private readonly sheetRef: SheetRef,
    public readonly reference: string,
  ) {}

  get sheet(): string {
    return this.sheetRef.current;
  }

  info(): RangeInfo {
    return this.handle.getRange(this.sheet, this.reference) as RangeInfo;
  }

  values(): ApiCellValue[][] {
    return this.info().values;
  }

  formulas(): Array<Array<string | null>> {
    return this.info().formulas;
  }

  setValues(values: CellInput[][]): this {
    this.handle.setRangeValues(this.sheet, this.reference, values);
    return this;
  }

  setFormulas(formulas: Array<Array<string | null>>): this {
    this.handle.setRangeFormulas(this.sheet, this.reference, formulas);
    return this;
  }

  /**
   * Apply a {@link StylePatch} to every cell in the range.
   *
   * Merged-cell caveat: in OOXML only the **top-left anchor** of a merged region
   * carries a style; the other cells in the merge are written as empty. To restyle
   * a merged range, target its top-left anchor (e.g. `sheet.range("A1")` for a
   * merge anchored at `A1:C3`) rather than the full `A1:C3` reference. Calling
   * `setStyle` on the wider range will still touch the anchor but is otherwise
   * a no-op on the merged remainder.
   */
  setStyle(patch: StylePatch): this {
    this.handle.setStyle(this.sheet, this.reference, patch);
    return this;
  }

  clear(mode?: ClearMode): this {
    if (mode === undefined) {
      this.handle.clearRange(this.sheet, this.reference);
    } else {
      this.handle.clearRangeWith(this.sheet, this.reference, mode);
    }
    return this;
  }

  copyTo(dest: Range | string): Range {
    const destSheet = dest instanceof Range ? dest.sheet : this.sheet;
    const destRef = dest instanceof Range ? dest.reference : dest;
    const info = this.handle.copyRange(this.sheet, this.reference, destSheet, destRef) as RangeInfo;
    return new Range(this.handle, this.sheetRef, info.reference);
  }

  fillTo(dest: Range | string): Range {
    const destSheet = dest instanceof Range ? dest.sheet : this.sheet;
    const destRef = dest instanceof Range ? dest.reference : dest;
    const info = this.handle.fillRange(this.sheet, this.reference, destSheet, destRef) as RangeInfo;
    return new Range(this.handle, this.sheetRef, info.reference);
  }

  moveTo(dest: Range | string): Range {
    const destSheet = dest instanceof Range ? dest.sheet : this.sheet;
    const destRef = dest instanceof Range ? dest.reference : dest;
    const info = this.handle.moveRange(this.sheet, this.reference, destSheet, destRef) as RangeInfo;
    return new Range(this.handle, this.sheetRef, info.reference);
  }

  merge(): this {
    this.handle.addMerge(this.sheet, this.reference);
    return this;
  }

  unmerge(): this {
    this.handle.removeMerge(this.sheet, this.reference);
    return this;
  }
}

export class Cell {
  constructor(
    private readonly handle: WasmWorkbookHandle,
    private readonly sheetRef: SheetRef,
    public readonly reference: string,
  ) {}

  get sheet(): string {
    return this.sheetRef.current;
  }

  asRange(): Range {
    return new Range(this.handle, this.sheetRef, this.reference);
  }

  info(): CellInfo {
    return this.handle.getCell(this.sheet, this.reference) as CellInfo;
  }

  value(): ApiCellValue {
    return this.info().value;
  }

  formula(): string | undefined {
    return this.info().formula;
  }

  setValue(value: CellInput): this {
    this.handle.setValue(this.sheet, this.reference, value);
    return this;
  }

  setFormula(formula: string): this {
    this.handle.setFormula(this.sheet, this.reference, formula);
    return this;
  }

  /** The cell's rich-text runs, if it holds a multi-run inline string. */
  richText(): RichText | undefined {
    return this.info().richText;
  }

  /** Write the cell as a rich-text inline string of formatted runs. */
  setRichText(runs: RichTextRun[]): this {
    this.handle.setRichText(this.sheet, this.reference, runs);
    return this;
  }

  /**
   * Apply a {@link StylePatch} to this cell.
   *
   * If this cell is part of a merged region, the style is only persisted when the
   * reference points at the merge's **top-left anchor** — OOXML stores style on
   * the anchor only and writes the merged remainder as empty. Targeting any other
   * cell of the merge is silently a no-op at render time.
   */
  setStyle(patch: StylePatch): this {
    this.handle.setStyle(this.sheet, this.reference, patch);
    return this;
  }

  clear(mode?: ClearMode): this {
    if (mode === undefined) {
      this.handle.clear(this.sheet, this.reference);
    } else {
      this.handle.clearWith(this.sheet, this.reference, mode);
    }
    return this;
  }

  precedents(): DependencyReference[] {
    return this.handle.precedents(this.sheet, this.reference) as DependencyReference[];
  }

  dependents(): DependencyReference[] {
    return this.handle.dependents(this.sheet, this.reference) as DependencyReference[];
  }

  dependencies(): DependencyInfo {
    return this.handle.dependencies(this.sheet, this.reference) as DependencyInfo;
  }

  setHyperlink(patch: HyperlinkPatch): HyperlinkInfo {
    return this.handle.setHyperlink(this.sheet, this.reference, patch) as HyperlinkInfo;
  }

  removeHyperlink(): HyperlinkInfo[] {
    return this.handle.removeHyperlink(this.sheet, this.reference) as HyperlinkInfo[];
  }

  setComment(patch: CommentPatch): CommentInfo {
    return this.handle.setComment(this.sheet, this.reference, patch) as CommentInfo;
  }

  removeComment(): CommentInfo[] {
    return this.handle.removeComment(this.sheet, this.reference) as CommentInfo[];
  }
}

export function makeRange(
  handle: WasmWorkbookHandle,
  sheetRef: SheetRef,
  addr: RangeAddress,
): Range {
  return new Range(handle, sheetRef, resolveRange(addr));
}

export function makeCell(handle: WasmWorkbookHandle, sheetRef: SheetRef, addr: CellAddress): Cell {
  return new Cell(handle, sheetRef, resolveCell(addr));
}
