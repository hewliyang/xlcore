import type { ChartAnchor } from "./api-schema/ChartAnchor.js";

export interface AnchoredDrawing {
  id: string;
  anchor: ChartAnchor;
}

function anchorCellsMatch(a: ChartAnchor, b: ChartAnchor): boolean {
  return (
    a.fromColumn === b.fromColumn &&
    a.fromRow === b.fromRow &&
    a.toColumn === b.toColumn &&
    a.toRow === b.toRow
  );
}

export function resolveDrawingId(
  items: AnchoredDrawing[],
  prevAnchor: ChartAnchor,
  ordinal?: number,
): string | null {
  const matches = items.filter((c) => anchorCellsMatch(c.anchor, prevAnchor));
  if (matches.length === 1 && matches[0]) return matches[0].id;
  if (ordinal !== undefined) return items[ordinal]?.id ?? null;
  return null;
}

export function resolveChartId(
  charts: AnchoredDrawing[],
  prevAnchor: ChartAnchor,
  chartOrdinal?: number,
): string | null {
  return resolveDrawingId(charts, prevAnchor, chartOrdinal);
}
