import type { CellRef } from "./CellRef.js";
import type { Merge } from "./Merge.js";

export type Pivot = { name: string; range: Merge; filterArrowCells: Array<CellRef> };
