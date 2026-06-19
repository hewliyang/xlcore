import type { Selection } from "./interact.js";

function normalize(sel: Selection): { r1: number; c1: number; r2: number; c2: number } {
  return {
    r1: Math.min(sel.r1, sel.r2),
    c1: Math.min(sel.c1, sel.c2),
    r2: Math.max(sel.r1, sel.r2),
    c2: Math.max(sel.c1, sel.c2),
  };
}

export function projectFill(
  sourceValues: string[][],
  target: Selection,
  source: Selection,
): string[][] {
  const s = normalize(source);
  const t = normalize(target);
  const srcRows = s.r2 - s.r1 + 1;
  const srcCols = s.c2 - s.c1 + 1;
  if (srcRows <= 0 || srcCols <= 0) return [];
  const rows = t.r2 - t.r1 + 1;
  const cols = t.c2 - t.c1 + 1;
  const out: string[][] = [];
  for (let i = 0; i < rows; i++) {
    const srcRow = sourceValues[i % srcRows] ?? [];
    const row: string[] = [];
    for (let j = 0; j < cols; j++) {
      row.push(srcRow[j % srcCols] ?? "");
    }
    out.push(row);
  }
  return out;
}
