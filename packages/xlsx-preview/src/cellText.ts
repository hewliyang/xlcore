import type { Cell, CellFormat, Sheet, WorkbookLayout } from "./types.js";
import { formatValue } from "./numfmt.js";

const BUILTIN_NUMFMT: Record<number, string> = {
  0: "General",
  1: "0",
  2: "0.00",
  3: "#,##0",
  4: "#,##0.00",
  5: "$#,##0_);($#,##0)",
  6: "$#,##0_);[Red]($#,##0)",
  7: "$#,##0.00_);($#,##0.00)",
  8: "$#,##0.00_);[Red]($#,##0.00)",
  9: "0%",
  10: "0.00%",
  11: "0.00E+00",
  12: "# ?/?",
  13: "# ??/??",
  14: "m/d/yyyy",
  15: "d-mmm-yy",
  16: "d-mmm",
  17: "mmm-yy",
  18: "h:mm AM/PM",
  19: "h:mm:ss AM/PM",
  20: "h:mm",
  21: "h:mm:ss",
  22: "m/d/yyyy h:mm",
  37: "#,##0;(#,##0)",
  38: "#,##0;[Red](#,##0)",
  39: "#,##0.00;(#,##0.00)",
  40: "#,##0.00;[Red](#,##0.00)",
  41: '_(* #,##0_);_(* (#,##0);_(* "-"_);_(@_)',
  42: '_("$"* #,##0_);_("$"* (#,##0);_("$"* "-"_);_(@_)',
  43: '_(* #,##0.00_);_(* (#,##0.00);_(* "-"??_);_(@_)',
  44: '_("$"* #,##0.00_);_("$"* (#,##0.00);_("$"* "-"??_);_(@_)',
  45: "mm:ss",
  46: "[h]:mm:ss",
  47: "mm:ss.0",
  48: "##0.0E+0",
  49: "@",
};

export interface ResolvedText {
  text: string;
  defaultAlign: "left" | "right" | "center";
  formatColor?: string;
  /** Accounting `*x` fill chars (see `FormatResult.fills`). Each `\u0001`
   *  in `text` is a placeholder the caller must expand against the cell
   *  width. Undefined when the format had no `*x` token. */
  fills?: string[];
}

const NUMFMT_CODE_CACHE = new WeakMap<WorkbookLayout, Map<number, string>>();

// Per-sheet column-style lookup. Built lazily once per sheet from
// `sheet.cols[].styleIndex`, since column-level styles rarely overlap and
// most workbooks have <100 col specs. Map<colIndex 0-based, xfId>.
const COL_STYLE_CACHE = new WeakMap<Sheet, Map<number, number>>();

function colStyleMap(sheet: Sheet): Map<number, number> {
  let m = COL_STYLE_CACHE.get(sheet);
  if (m) return m;
  m = new Map<number, number>();
  for (const col of sheet.cols) {
    if (col.styleIndex === undefined) continue;
    // OOXML `<col min/max>` are 1-based, inclusive. Cell.c is 0-based.
    for (let i = col.min - 1; i <= col.max - 1; i++) m.set(i, col.styleIndex);
  }
  COL_STYLE_CACHE.set(sheet, m);
  return m;
}

/** Resolve the effective `CellFormat` (xf) for a cell, applying the
 *  OOXML §18.3.1.4 fallback chain:
 *
 *    cell.s → row.s → col.style → xf 0
 *
 *  Excel writes formula cells (`<c><f/><v/></c>`) without an `s`
 *  attribute when their style matches xf 0; we used to fall through
 *  to `undefined` and then render with the `formatGeneral()` default,
 *  which dropped thousands separators / accounting parens / decimal
 *  precision on every formula cell whose author left the format on
 *  the default xf. Walking the row/col fallbacks first matches what
 *  Excel and SpreadJS do. */
export function resolveCellXf(
  cell: Cell,
  sheet: Sheet,
  layout: WorkbookLayout,
): CellFormat | undefined {
  const xfs = layout.styles.cellXfs;
  if (cell.styleIndex !== undefined) return xfs[cell.styleIndex];
  const meta = sheet.decodedRowMeta;
  const rowSlot = meta.byIndex.get(cell.r);
  if (rowSlot !== undefined) {
    const sIdx = meta.styleIdx[rowSlot] ?? -1;
    if (sIdx >= 0) return xfs[sIdx];
  }
  const colXf = colStyleMap(sheet).get(cell.c);
  if (colXf !== undefined) return xfs[colXf];
  // Spec default: every cell without explicit/inherited style uses xf 0.
  return xfs[0];
}

function numFmtCode(layout: WorkbookLayout, id: number): string | undefined {
  let cache = NUMFMT_CODE_CACHE.get(layout);
  if (!cache) {
    cache = new Map<number, string>();
    for (const nf of layout.styles.numFmts) cache.set(nf.id, nf.formatCode);
    NUMFMT_CODE_CACHE.set(layout, cache);
  }
  return cache.get(id) ?? BUILTIN_NUMFMT[id];
}

export function resolveCellText(
  cell: Cell,
  layout: WorkbookLayout,
  xf: CellFormat | undefined,
): ResolvedText {
  const v = cell.value ?? "";
  switch (cell.type) {
    case "s": {
      const idx = parseInt(v, 10);
      const s = layout.sharedStrings[idx] ?? "";
      return { text: s, defaultAlign: "left" };
    }
    case "inline":
    case "str":
      return { text: v, defaultAlign: "left" };
    case "b":
      return { text: v === "1" ? "TRUE" : "FALSE", defaultAlign: "center" };
    case "e":
      return { text: v, defaultAlign: "center" };
    case "f":
    case "n": {
      if (!v) return { text: "", defaultAlign: "right" };
      // Use `Number(v)` (strict) instead of `parseFloat(v)` — the latter
      // happily parses `"1Q24"` as `1` and silently strips the suffix,
      // which breaks formula cells whose cached `<v>` is a string
      // result (e.g. `=TEXT(...)` producing `"1Q24"`). Excel writes
      // those as `<c t="str">` but plenty of authoring tools —
      // including SpreadJS round-trips of analyst models like
      // `e-007_input-3.xlsx` — leave the original `<c t="f">` tag
      // intact with a non-numeric `<v>`. `Number("1Q24")` returns
      // `NaN`, falling through to the left-aligned string branch
      // below, which then renders the cached text verbatim.
      const n = Number(v);
      if (Number.isNaN(n)) return { text: v, defaultAlign: "left" };
      const numFmtId = xf?.numFmtId;
      let code: string | undefined;
      if (numFmtId !== undefined) {
        code = numFmtCode(layout, numFmtId);
      }
      const r = formatValue(n, code);
      return {
        text: r.text,
        defaultAlign: "right",
        formatColor: r.color,
        fills: r.fills,
      };
    }
    default:
      return { text: v, defaultAlign: "left" };
  }
}

export function cellTextValue(cell: Cell, layout: WorkbookLayout): string {
  if (cell.value === undefined) return "";
  switch (cell.type) {
    case "s": {
      const idx = parseInt(cell.value, 10);
      return layout.sharedStrings[idx] ?? "";
    }
    case "inline":
    case "str":
      return cell.value;
    default:
      return cell.value;
  }
}

export function cellNumericValue(cell: Cell): number | null {
  if (cell.value === undefined) return null;
  if (cell.type === "n" || cell.type === "f" || cell.type === "b") {
    const n = parseFloat(cell.value);
    return Number.isNaN(n) ? null : n;
  }
  return null;
}
