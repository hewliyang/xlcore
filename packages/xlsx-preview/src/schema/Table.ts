import type { Merge } from "./Merge.js";
import type { TableColumn } from "./TableColumn.js";
import type { TableStyle } from "./TableStyle.js";

export type Table = {
  name: string;

  displayName: string;

  range: Merge;

  headerRowCount: number;

  totalsRowCount: number;

  columns: Array<TableColumn>;
  style?: TableStyle;

  hasAutoFilter: boolean;
};
