import type { TextRun } from "./TextRun.js";

export type Comment = {
  r: number;

  c: number;

  author: string;

  text: string;

  runs: Array<TextRun>;
};
