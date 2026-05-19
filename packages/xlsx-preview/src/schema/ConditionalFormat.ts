import type { CfRule } from "./CfRule.js";
import type { Merge } from "./Merge.js";

export type ConditionalFormat = { ranges: Array<Merge>; rules: Array<CfRule> };
