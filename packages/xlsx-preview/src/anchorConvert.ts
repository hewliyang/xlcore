import type { DrawingAnchor } from "./schema/DrawingAnchor.js";
import type { ChartAnchor } from "./api-schema/ChartAnchor.js";

export function wireAnchorToChartAnchor(a: DrawingAnchor): ChartAnchor {
  const out: ChartAnchor = {
    fromColumn: a.fromCol,
    fromRow: a.fromRow,
    toColumn: a.toCol,
    toRow: a.toRow,
  };
  if (a.fromColOffEmu) out.fromColumnOffsetEmu = BigInt(Math.round(a.fromColOffEmu));
  if (a.fromRowOffEmu) out.fromRowOffsetEmu = BigInt(Math.round(a.fromRowOffEmu));
  if (a.toColOffEmu) out.toColumnOffsetEmu = BigInt(Math.round(a.toColOffEmu));
  if (a.toRowOffEmu) out.toRowOffsetEmu = BigInt(Math.round(a.toRowOffEmu));
  return out;
}

export function chartAnchorToWireAnchor(a: ChartAnchor): DrawingAnchor {
  return {
    fromCol: a.fromColumn,
    fromColOffEmu: a.fromColumnOffsetEmu === undefined ? 0 : Number(a.fromColumnOffsetEmu),
    fromRow: a.fromRow,
    fromRowOffEmu: a.fromRowOffsetEmu === undefined ? 0 : Number(a.fromRowOffsetEmu),
    toCol: a.toColumn,
    toColOffEmu: a.toColumnOffsetEmu === undefined ? 0 : Number(a.toColumnOffsetEmu),
    toRow: a.toRow,
    toRowOffEmu: a.toRowOffsetEmu === undefined ? 0 : Number(a.toRowOffsetEmu),
  };
}
