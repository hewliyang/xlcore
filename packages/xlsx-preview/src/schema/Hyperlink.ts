import type { Merge } from "./Merge.js";

export type Hyperlink = {
  range: Merge;
  target?: string;
  location?: string;
  tooltip?: string;
  display?: string;
};
