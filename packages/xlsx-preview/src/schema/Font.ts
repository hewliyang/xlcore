import type { Color } from "./Color.js";

export type Font = {
  name?: string;
  size?: number;
  bold: boolean;
  italic: boolean;
  underline: boolean;

  underlineStyle?: string;
  strike: boolean;
  color?: Color;

  vertAlign?: string;

  family?: number;

  scheme?: string;
};
