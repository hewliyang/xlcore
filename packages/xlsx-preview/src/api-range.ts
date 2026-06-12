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
  StylePatch,
} from "./api-schema/index.js";
import {
  type CellAddress,
  type RangeAddress,
  type SheetRef,
  qualify,
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

  get qualifiedReference(): string {
    return qualify(this.sheet, this.reference);
  }

  info(): RangeInfo {
    return this.handle.getRange(this.qualifiedReference) as RangeInfo;
  }

  values(): ApiCellValue[][] {
    return this.info().values;
  }

  formulas(): Array<Array<string | null>> {
    return this.info().formulas;
  }

  setValues(values: CellInput[][]): this {
    this.handle.setRangeValues(this.qualifiedReference, values);
    return this;
  }

  setFormulas(formulas: Array<Array<string | null>>): this {
    this.handle.setRangeFormulas(this.qualifiedReference, formulas);
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
    this.handle.setStyle(this.qualifiedReference, patch);
    return this;
  }

  clear(mode?: ClearMode): this {
    if (mode === undefined) {
      this.handle.clearRange(this.qualifiedReference);
    } else {
      this.handle.clearRangeWith(this.qualifiedReference, mode);
    }
    return this;
  }

  copyTo(dest: Range | string): Range {
    const destRef = dest instanceof Range ? dest.qualifiedReference : qualify(this.sheet, dest);
    this.handle.copyRange(this.qualifiedReference, destRef);
    return new Range(this.handle, this.sheetRef, refOnly(destRef, this.sheet));
  }

  fillTo(dest: Range | string): Range {
    const destRef = dest instanceof Range ? dest.qualifiedReference : qualify(this.sheet, dest);
    this.handle.fillRange(this.qualifiedReference, destRef);
    return new Range(this.handle, this.sheetRef, refOnly(destRef, this.sheet));
  }

  merge(): this {
    this.handle.addMerge(this.qualifiedReference);
    return this;
  }

  unmerge(): this {
    this.handle.removeMerge(this.qualifiedReference);
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

  get qualifiedReference(): string {
    return qualify(this.sheet, this.reference);
  }

  asRange(): Range {
    return new Range(this.handle, this.sheetRef, this.reference);
  }

  info(): CellInfo {
    return this.handle.getCell(this.qualifiedReference) as CellInfo;
  }

  value(): ApiCellValue {
    return this.info().value;
  }

  formula(): string | undefined {
    return this.info().formula;
  }

  setValue(value: CellInput): this {
    this.handle.setValue(this.qualifiedReference, value);
    return this;
  }

  setFormula(formula: string): this {
    this.handle.setFormula(this.qualifiedReference, formula);
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
    this.handle.setStyle(this.qualifiedReference, patch);
    return this;
  }

  clear(mode?: ClearMode): this {
    if (mode === undefined) {
      this.handle.clear(this.qualifiedReference);
    } else {
      this.handle.clearWith(this.qualifiedReference, mode);
    }
    return this;
  }

  precedents(): DependencyReference[] {
    return this.handle.precedents(this.qualifiedReference) as DependencyReference[];
  }

  dependents(): DependencyReference[] {
    return this.handle.dependents(this.qualifiedReference) as DependencyReference[];
  }

  dependencies(): DependencyInfo {
    return this.handle.dependencies(this.qualifiedReference) as DependencyInfo;
  }

  setHyperlink(patch: HyperlinkPatch): HyperlinkInfo {
    return this.handle.setHyperlink(this.qualifiedReference, patch) as HyperlinkInfo;
  }

  removeHyperlink(): HyperlinkInfo[] {
    return this.handle.removeHyperlink(this.qualifiedReference) as HyperlinkInfo[];
  }

  setComment(patch: CommentPatch): CommentInfo {
    return this.handle.setComment(this.qualifiedReference, patch) as CommentInfo;
  }

  removeComment(): CommentInfo[] {
    return this.handle.removeComment(this.qualifiedReference) as CommentInfo[];
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

function refOnly(qualified: string, sheet: string): string {
  const prefix = qualify(sheet, "");
  if (qualified.startsWith(prefix)) return qualified.slice(prefix.length);
  const bang = findUnquotedBang(qualified);
  return bang >= 0 ? qualified.slice(bang + 1) : qualified;
}

function findUnquotedBang(s: string): number {
  let quoted = false;
  for (let i = 0; i < s.length; i++) {
    const ch = s[i];
    if (ch === "'") {
      if (quoted && s[i + 1] === "'") {
        i++;
        continue;
      }
      quoted = !quoted;
    } else if (ch === "!" && !quoted) {
      return i;
    }
  }
  return -1;
}
