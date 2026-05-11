import type { Sheet } from "./types.js";
export declare function computeCfIconState(sheet: Sheet, locks?: Map<string, number>): {
    cfIconReserve: Map<string, number>;
    cfIconDraw: Map<string, {
        iconSet: string;
        idx: number;
        n: number;
    }>;
    cfIconSuppress: Set<string>;
};
