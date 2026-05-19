import type { CustomTableStyle } from "./CustomTableStyle.js";
import type { DefinedName } from "./DefinedName.js";
import type { Dxf } from "./Dxf.js";
import type { Sheet } from "./Sheet.js";
import type { Styles } from "./Styles.js";
import type { TextRun } from "./TextRun.js";
import type { Theme } from "./Theme.js";

export type WorkbookLayout = {
  sheets: Array<Sheet>;
  styles: Styles;
  sharedStrings: Array<string>;

  sharedStringRuns: Array<Array<TextRun>>;

  dxfs: Array<Dxf>;

  tableStyles: Array<CustomTableStyle>;

  theme?: Theme;

  definedNames: Array<DefinedName>;

  activeSheetIndex?: number;
};
