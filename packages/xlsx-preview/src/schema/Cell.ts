import type { TextRun } from "./TextRun.js";

export type Cell = {
  r: number;
  c: number;
  type: string;
  value?: string;
  formula?: string;
  styleIndex?: number;
  runs: Array<TextRun>;
};
