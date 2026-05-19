import type { Color } from "./Color.js";

export type Dxf = {
  fontColor?: Color;
  bold?: boolean;
  italic?: boolean;
  strike?: boolean;
  underline?: boolean;
  underlineStyle?: string;
  fillColor?: Color;
  numFmt?: string;
  vertAlign?: string;
};
