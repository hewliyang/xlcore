import type { Cell } from "./Cell.js";

export type Row = {
  index: number;
  heightPx?: number;
  cells: Array<Cell>;
  styleIndex?: number;
  hidden: boolean;
  outlineLevel: number;
};
