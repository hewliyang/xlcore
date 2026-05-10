import type { Sheet } from "./types.js";
import { resolveCfvoValue } from "./conditionalFormatting.js";

const ICON_RESERVE_PX = 18;

export function computeCfIconState(sheet: Sheet): {
  cfIconReserve: Map<string, number>;
  cfIconDraw: Map<string, { iconSet: string; idx: number; n: number }>;
  cfIconSuppress: Set<string>;
} {
  const cfIconReserve = new Map<string, number>();
  const cfIconDraw = new Map<string, { iconSet: string; idx: number; n: number }>();
  const cfIconSuppress = new Set<string>();
  const cfs = sheet.conditionalFormats;
  if (!cfs || cfs.length === 0) return { cfIconReserve, cfIconDraw, cfIconSuppress };

  // Numeric values for cfvo resolution. Same plumbing as data-bar /
  // color-scale paths.
  const cellNumeric = new Map<string, number>();
  for (const row of sheet.rows) {
    for (const cell of row.cells) {
      if (cell.value === undefined) continue;
      if (cell.type === "n" || cell.type === "f") {
        const n = parseFloat(cell.value);
        if (!Number.isNaN(n)) cellNumeric.set(`${cell.r}:${cell.c}`, n);
      }
    }
  }

  for (const cf of cfs) {
    const rule = cf.rules
      .filter((r) => r.kind === "iconSet" && r.iconSet)
      .sort((a, b) => a.priority - b.priority)[0];
    if (!rule || !rule.iconSet) continue;
    const is = rule.iconSet;
    const n = is.cfvos.length;
    if (n < 3) continue;

    // Gather values inside this rule's ranges to drive percent /
    // percentile / min / max thresholds.
    const values: number[] = [];
    for (const range of cf.ranges) {
      for (let r = range.r1; r <= range.r2; r++) {
        for (let c = range.c1; c <= range.c2; c++) {
          const v = cellNumeric.get(`${r}:${c}`);
          if (v !== undefined) values.push(v);
        }
      }
    }
    if (values.length === 0) continue;
    const dataMin = Math.min(...values);
    const dataMax = Math.max(...values);
    const sorted = [...values].sort((a, b) => a - b);

    // Resolve every cfvo to a numeric threshold. cfvos[0] is the
    // implicit anchor (low icon); thresholds[k] for k>=1 govern when
    // icon k applies (value >= thresholds[k]).
    const thresholds: number[] = is.cfvos.map((s, i) =>
      resolveCfvoValue(s, dataMin, dataMax, sorted, i === 0),
    );

    for (const range of cf.ranges) {
      for (let r = range.r1; r <= range.r2; r++) {
        for (let c = range.c1; c <= range.c2; c++) {
          const k = `${r}:${c}`;
          const v = cellNumeric.get(k);
          if (v === undefined) continue;
          // Largest k such that v >= thresholds[k]; default 0 (low icon).
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
    }
  }
  return { cfIconReserve, cfIconDraw, cfIconSuppress };
}
