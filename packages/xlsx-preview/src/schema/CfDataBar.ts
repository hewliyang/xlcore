import type { CfvoStop } from "./CfvoStop.js";
import type { Color } from "./Color.js";

export type CfDataBar = {
  min: CfvoStop;
  max: CfvoStop;
  color: Color;
  negativeColor?: Color;
  minLengthPct: number;
  maxLengthPct: number;
  showValue: boolean;
  gradient: boolean;
};
