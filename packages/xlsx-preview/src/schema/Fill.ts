import type { Color } from "./Color.js";
import type { GradientStop } from "./GradientStop.js";

export type Fill = {
  patternType?: string;
  fgColor?: Color;
  bgColor?: Color;

  gradientStops: Array<GradientStop>;

  gradientType?: string;

  gradientDegree?: number;

  gradientLeft?: number;
  gradientRight?: number;
  gradientTop?: number;
  gradientBottom?: number;
};
