import type { Border } from "./Border.js";
import type { CellFormat } from "./CellFormat.js";
import type { Fill } from "./Fill.js";
import type { Font } from "./Font.js";
import type { NumberFormat } from "./NumberFormat.js";

export type Styles = {
  fonts: Array<Font>;
  fills: Array<Fill>;
  borders: Array<Border>;
  cellXfs: Array<CellFormat>;
  numFmts: Array<NumberFormat>;
  defaultFont: string;
  defaultFontSize: number;
};
