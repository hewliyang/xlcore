import type { Color } from "./Color.js";

export type TextRun = {
  text: string;
  bold: boolean;
  italic: boolean;
  underline: boolean;

  underlineStyle?: string;
  strike: boolean;

  size?: number;
  fontName?: string;
  color?: Color;

  vertAlign?: string;

  family?: number;

  scheme?: string;
};
