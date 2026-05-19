import type { Sheet } from "./types.js";
import { iterAllCells } from "./columnar.js";
import { isCfLocked, resolveCfvoValue } from "./conditionalFormatting.js";

const ICON_RESERVE_PX = 18;

export function computeCfIconState(
  sheet: Sheet,
  locks?: Map<string, number>,
): {
  cfIconReserve: Map<string, number>;
  cfIconDraw: Map<string, { iconSet: string; idx: number; n: number }>;
  cfIconSuppress: Set<string>;
} {
  const cfIconReserve = new Map<string, number>();
  const cfIconDraw = new Map<string, { iconSet: string; idx: number; n: number }>();
  const cfIconSuppress = new Set<string>();
  const cfs = sheet.conditionalFormats;
  if (!cfs || cfs.length === 0) return { cfIconReserve, cfIconDraw, cfIconSuppress };

  const cellNumeric = new Map<string, number>();
  iterAllCells(sheet, (cell) => {
    if (cell.value === undefined) return;
    if (cell.type === "n" || cell.type === "f") {
      const n = parseFloat(cell.value);
      if (!Number.isNaN(n)) cellNumeric.set(`${cell.r}:${cell.c}`, n);
    }
  });

  for (const cf of cfs) {
    const rule = cf.rules
      .filter((r) => r.kind === "iconSet" && r.iconSet)
      .sort((a, b) => a.priority - b.priority)[0];
    if (!rule || !rule.iconSet) continue;
    const is = rule.iconSet;
    const n = is.cfvos.length;
    if (n < 3) continue;

    const values = numericValuesInRanges(cellNumeric, cf.ranges);
    if (values.length === 0) continue;
    const dataMin = Math.min(...values);
    const dataMax = Math.max(...values);
    const sorted = [...values].sort((a, b) => a - b);

    const thresholds: number[] = is.cfvos.map((s, i) =>
      resolveCfvoValue(s, dataMin, dataMax, sorted, i === 0),
    );

    for (const [k, v] of cellsInNumericRanges(cellNumeric, cf.ranges)) {
      if (isCfLocked(locks, k, rule.priority)) continue;

      let idx = 0;
      for (let i = 1; i < n; i++) {
        if (v >= thresholds[i]!) idx = i;
      }
      if (is.reverse) idx = n - 1 - idx;
      cfIconReserve.set(k, ICON_RESERVE_PX);
      cfIconDraw.set(k, { iconSet: is.iconSet, idx, n });
      if (!is.showValue) cfIconSuppress.add(k);
    }
  }
  return { cfIconReserve, cfIconDraw, cfIconSuppress };
}

function numericValuesInRanges(
  cellNumeric: Map<string, number>,
  ranges: { r1: number; r2: number; c1: number; c2: number }[],
): number[] {
  return cellsInNumericRanges(cellNumeric, ranges).map(([, v]) => v);
}

function cellsInNumericRanges(
  cellNumeric: Map<string, number>,
  ranges: { r1: number; r2: number; c1: number; c2: number }[],
): [string, number][] {
  const out: [string, number][] = [];
  for (const [k, v] of cellNumeric) {
    const sep = k.indexOf(":");
    const r = Number(k.slice(0, sep));
    const c = Number(k.slice(sep + 1));
    for (const range of ranges) {
      if (r >= range.r1 && r <= range.r2 && c >= range.c1 && c <= range.c2) {
        out.push([k, v]);
        break;
      }
    }
  }
  return out;
}
