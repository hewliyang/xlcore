import { decodeWorkbookLayout } from "./columnar.js";
import type { WorkbookLayout } from "./types.js";

export function patchWorkbookSheet(base: WorkbookLayout, incoming: WorkbookLayout): boolean {
  const sheet = incoming.sheets[0];
  if (!sheet) return false;
  const targetIndex = base.sheets.findIndex((s) => s.name === sheet.name);
  if (targetIndex < 0) return false;
  base.styles = incoming.styles;
  base.sharedStrings = incoming.sharedStrings;
  base.sharedStringRuns = incoming.sharedStringRuns;
  base.dxfs = incoming.dxfs;
  base.tableStyles = incoming.tableStyles;
  base.theme = incoming.theme;
  base.definedNames = incoming.definedNames;
  base.sheets[targetIndex] = sheet;
  decodeWorkbookLayout(base);
  return true;
}
