import type { Cell, CellFormat, WorkbookLayout } from "./types.js";
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
      const n = parseFloat(v);
      if (Number.isNaN(n)) return { text: v, defaultAlign: "left" };
      const numFmtId = xf?.numFmtId;
      let code: string | undefined;
      if (numFmtId !== undefined) {
        code =
          layout.styles.numFmts.find((nf) => nf.id === numFmtId)?.formatCode ??
          BUILTIN_NUMFMT[numFmtId];
      }
      const r = formatValue(n, code);
      return { text: r.text, defaultAlign: "right", formatColor: r.color };
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
