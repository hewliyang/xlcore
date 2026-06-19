import type { ChartAnchor } from "./api-schema/ChartAnchor.js";
import type { ChartInfo } from "./api-schema/ChartInfo.js";

function anchorCellsMatch(a: ChartAnchor, b: ChartAnchor): boolean {
  return (
    a.fromColumn === b.fromColumn &&
    a.fromRow === b.fromRow &&
    a.toColumn === b.toColumn &&
    a.toRow === b.toRow
  );
}

export function resolveChartId(
  charts: ChartInfo[],
  prevAnchor: ChartAnchor,
  chartOrdinal?: number,
): string | null {
  const matches = charts.filter((c) => anchorCellsMatch(c.anchor, prevAnchor));
  if (matches.length === 1 && matches[0]) return matches[0].id;
  if (chartOrdinal !== undefined) return charts[chartOrdinal]?.id ?? null;
  return null;
}
