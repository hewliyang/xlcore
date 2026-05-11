import type { CfRule } from "./CfRule.js";
import type { Merge } from "./Merge.js";
export type ConditionalFormat = {
    /**
     * One or more rectangular ranges (from the sqref attribute).
     */
    ranges: Array<Merge>;
    rules: Array<CfRule>;
};
