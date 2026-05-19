import type { CfvoStop } from "./CfvoStop.js";

export type CfIconSet = {
  iconSet: string;
  cfvos: Array<CfvoStop>;
  showValue: boolean;
  reverse: boolean;
};
