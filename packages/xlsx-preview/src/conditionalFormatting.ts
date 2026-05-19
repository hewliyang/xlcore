import type { Cell, CfvoStop, CfRule, Dxf, Merge, Sheet, WorkbookLayout } from "./types.js";
import { cellNumericValue, cellTextValue } from "./cellText.js";
import { colorToCss } from "./color.js";
import { interpolateStops, resolveColorScaleStops } from "./cfColorScale.js";
import { withAlpha } from "./chartUtils.js";
import { iterAllCells } from "./columnar.js";
import type { Grid } from "./grid.js";
import { buildMergeMaps, cellRect, mergedRect } from "./geometry.js";
import type { Visible } from "./renderTypes.js";

const PREDICATE_KINDS = new Set([
  "cellIs",
  "top10",
  "aboveAverage",
  "duplicateValues",
  "uniqueValues",
  "containsText",
  "notContainsText",
  "beginsWith",
  "endsWith",
  "timePeriod",
]);

export function computeCfStopLocks(sheet: Sheet, layout: WorkbookLayout): Map<string, number> {
  const locks = new Map<string, number>();
  const cfs = sheet.conditionalFormats;
  if (!cfs || cfs.length === 0) return locks;

  const cellByKey = new Map<string, Cell>();
  iterAllCells(sheet, (cell) => {
    cellByKey.set(`${cell.r}:${cell.c}`, cell);
  });

  const entries: { rule: CfRule; ranges: Merge[] }[] = [];
  for (const cf of cfs) for (const rule of cf.rules) entries.push({ rule, ranges: cf.ranges });
  entries.sort((a, b) => a.rule.priority - b.rule.priority);

  for (const { rule, ranges } of entries) {
    if (!rule.stopIfTrue) continue;
    let matched: Iterable<string>;
    if (PREDICATE_KINDS.has(rule.kind)) {
      matched = computeRuleMatchSet(rule, ranges, cellByKey, layout);
    } else if (rule.kind === "colorScale" || rule.kind === "dataBar" || rule.kind === "iconSet") {
      const all: string[] = [];

      for (const range of ranges) {
        const r1 = Math.max(1, range.r1);
        const r2 = Math.min(sheet.maxRow, range.r2);
        const c1 = Math.max(1, range.c1);
        const c2 = Math.min(sheet.maxCol, range.c2);
        for (let r = r1; r <= r2; r++) {
          for (let c = c1; c <= c2; c++) all.push(`${r}:${c}`);
        }
      }
      matched = all;
    } else {
      continue;
    }
    for (const k of matched) {
      const cur = locks.get(k);
      if (cur === undefined || rule.priority < cur) locks.set(k, rule.priority);
    }
  }
  return locks;
}

export function isCfLocked(
  locks: Map<string, number> | undefined,
  cellKey: string,
  rulePriority: number,
): boolean {
  if (!locks) return false;
  const at = locks.get(cellKey);
  return at !== undefined && at < rulePriority;
}

export function computeCfDxfMap(
  sheet: Sheet,
  layout: WorkbookLayout,
  locks?: Map<string, number>,
): Map<string, Dxf> {
  const out = new Map<string, Dxf>();
  const dxfs = layout.dxfs ?? [];
  if (dxfs.length === 0) return out;
  const cfs = sheet.conditionalFormats;
  if (!cfs || cfs.length === 0) return out;

  const cellByKey = new Map<string, Cell>();
  iterAllCells(sheet, (cell) => {
    cellByKey.set(`${cell.r}:${cell.c}`, cell);
  });

  const locallyLocked = new Set<string>();

  for (const cf of cfs) {
    const sortedRules = [...cf.rules].sort((a, b) => a.priority - b.priority);
    for (const rule of sortedRules) {
      if (!PREDICATE_KINDS.has(rule.kind)) continue;
      if (rule.dxfId === undefined) continue;
      const dxf = dxfs[rule.dxfId];
      if (!dxf) continue;

      const matched = computeRuleMatchSet(rule, cf.ranges, cellByKey, layout);
      if (matched.size === 0) continue;

      for (const k of matched) {
        if (locallyLocked.has(k)) continue;
        if (isCfLocked(locks, k, rule.priority)) continue;
        const prev = out.get(k);
        out.set(k, prev ? mergeDxf(prev, dxf) : dxf);
        if (rule.stopIfTrue) locallyLocked.add(k);
      }
    }
  }
  return out;
}

function actualCellsInRanges(
  cellByKey: Map<string, Cell>,
  ranges: Merge[],
): { k: string; cell: Cell }[] {
  const out: { k: string; cell: Cell }[] = [];
  for (const [k, cell] of cellByKey) {
    if (cellInRanges(cell.r, cell.c, ranges)) out.push({ k, cell });
  }
  return out;
}

function numericValuesInRanges(cellNumeric: Map<string, number>, ranges: Merge[]): number[] {
  const out: number[] = [];
  for (const [k, v] of cellNumeric) {
    const sep = k.indexOf(":");
    const r = Number(k.slice(0, sep));
    const c = Number(k.slice(sep + 1));
    if (cellInRanges(r, c, ranges)) out.push(v);
  }
  return out;
}

function cellInRanges(r: number, c: number, ranges: Merge[]): boolean {
  for (const range of ranges) {
    if (r >= range.r1 && r <= range.r2 && c >= range.c1 && c <= range.c2) return true;
  }
  return false;
}

function computeRuleMatchSet(
  rule: CfRule,
  ranges: Merge[],
  cellByKey: Map<string, Cell>,
  layout: WorkbookLayout,
): Set<string> {
  const out = new Set<string>();

  const covered = actualCellsInRanges(cellByKey, ranges);

  switch (rule.kind) {
    case "cellIs": {
      for (const { k, cell } of covered) {
        if (evaluateCellIs(cell, rule.operator, rule.operands, layout)) out.add(k);
      }
      break;
    }
    case "top10": {
      const nums: { k: string; v: number }[] = [];
      for (const { k, cell } of covered) {
        if (!cell) continue;
        const v = cellNumericValue(cell);
        if (v !== null) nums.push({ k, v });
      }
      if (nums.length === 0) break;
      const rankRaw = rule.rank ?? 10;
      let n: number;
      if (rule.percent) {
        const pct = Math.max(0, Math.min(100, rankRaw));
        n = Math.max(1, Math.min(nums.length, Math.ceil((nums.length * pct) / 100)));
      } else {
        n = Math.max(1, Math.min(nums.length, rankRaw));
      }

      nums.sort((a, b) => (rule.bottom ? a.v - b.v : b.v - a.v));

      const cutoff = nums[n - 1]!.v;
      for (const { k, v } of nums) {
        if (rule.bottom ? v <= cutoff : v >= cutoff) out.add(k);
      }
      break;
    }
    case "aboveAverage": {
      const nums: { k: string; v: number }[] = [];
      for (const { k, cell } of covered) {
        if (!cell) continue;
        const v = cellNumericValue(cell);
        if (v !== null) nums.push({ k, v });
      }
      if (nums.length === 0) break;
      const above = rule.aboveAverage ?? true;
      const mean = nums.reduce((s, x) => s + x.v, 0) / nums.length;
      let threshold = mean;
      if (rule.stdDev !== undefined && rule.stdDev !== null) {
        const variance = nums.reduce((s, x) => s + (x.v - mean) ** 2, 0) / nums.length;
        const sd = Math.sqrt(variance);
        const k = Math.abs(rule.stdDev);
        threshold = above ? mean + k * sd : mean - k * sd;
      }
      for (const { k, v } of nums) {
        let hit: boolean;
        if (above) {
          hit = rule.equalAverage ? v >= threshold : v > threshold;
        } else {
          hit = rule.equalAverage ? v <= threshold : v < threshold;
        }
        if (hit) out.add(k);
      }
      break;
    }
    case "duplicateValues":
    case "uniqueValues": {
      const counts = new Map<string, number>();
      const keyOf: { k: string; bucket: string | null }[] = [];
      for (const { k, cell } of covered) {
        if (!cell || cell.value === undefined || cell.value === "") {
          keyOf.push({ k, bucket: null });
          continue;
        }
        const num = cellNumericValue(cell);
        const bucket = num !== null ? `n:${num}` : `s:${cellTextValue(cell, layout)}`;
        keyOf.push({ k, bucket });
        counts.set(bucket, (counts.get(bucket) ?? 0) + 1);
      }
      const wantDup = rule.kind === "duplicateValues";
      for (const { k, bucket } of keyOf) {
        if (bucket === null) continue;
        const c = counts.get(bucket) ?? 0;
        if (wantDup ? c > 1 : c === 1) out.add(k);
      }
      break;
    }
    case "containsText":
    case "notContainsText":
    case "beginsWith":
    case "endsWith": {
      const needle = (rule.text ?? "").toLowerCase();
      if (needle.length === 0) break;
      for (const { k, cell } of covered) {
        if (!cell) {
          if (rule.kind === "notContainsText") out.add(k);
          continue;
        }
        const hay = cellTextValue(cell, layout).toLowerCase();
        let hit = false;
        switch (rule.kind) {
          case "containsText":
            hit = hay.includes(needle);
            break;
          case "notContainsText":
            hit = !hay.includes(needle);
            break;
          case "beginsWith":
            hit = hay.startsWith(needle);
            break;
          case "endsWith":
            hit = hay.endsWith(needle);
            break;
        }
        if (hit) out.add(k);
      }
      break;
    }
    case "timePeriod": {
      const today = new Date();
      today.setHours(0, 0, 0, 0);
      const period = rule.timePeriod ?? "today";
      for (const { k, cell } of covered) {
        if (!cell) continue;
        const v = cellNumericValue(cell);
        if (v === null) continue;
        const cellDate = excelSerialToDate(v);
        if (!cellDate) continue;
        cellDate.setHours(0, 0, 0, 0);
        if (matchesTimePeriod(cellDate, today, period)) out.add(k);
      }
      break;
    }
  }
  return out;
}

function excelSerialToDate(serial: number): Date | null {
  if (!isFinite(serial) || serial < 0) return null;

  const ms = (serial - 25569) * 86400 * 1000;
  const d = new Date(ms);
  return Number.isNaN(d.getTime()) ? null : d;
}

function matchesTimePeriod(cellDay: Date, today: Date, period: string): boolean {
  const dayMs = 86400 * 1000;
  const diffDays = Math.round((cellDay.getTime() - today.getTime()) / dayMs);
  switch (period) {
    case "today":
      return diffDays === 0;
    case "yesterday":
      return diffDays === -1;
    case "tomorrow":
      return diffDays === 1;
    case "last7Days":
      return diffDays <= 0 && diffDays >= -6;
    case "thisWeek": {
      const tow = today.getDay();
      const start = new Date(today);
      start.setDate(today.getDate() - tow);
      const end = new Date(start);
      end.setDate(start.getDate() + 6);
      return cellDay >= start && cellDay <= end;
    }
    case "lastWeek": {
      const tow = today.getDay();
      const start = new Date(today);
      start.setDate(today.getDate() - tow - 7);
      const end = new Date(start);
      end.setDate(start.getDate() + 6);
      return cellDay >= start && cellDay <= end;
    }
    case "nextWeek": {
      const tow = today.getDay();
      const start = new Date(today);
      start.setDate(today.getDate() - tow + 7);
      const end = new Date(start);
      end.setDate(start.getDate() + 6);
      return cellDay >= start && cellDay <= end;
    }
    case "thisMonth":
      return (
        cellDay.getFullYear() === today.getFullYear() && cellDay.getMonth() === today.getMonth()
      );
    case "lastMonth": {
      const m = new Date(today.getFullYear(), today.getMonth() - 1, 1);
      return cellDay.getFullYear() === m.getFullYear() && cellDay.getMonth() === m.getMonth();
    }
    case "nextMonth": {
      const m = new Date(today.getFullYear(), today.getMonth() + 1, 1);
      return cellDay.getFullYear() === m.getFullYear() && cellDay.getMonth() === m.getMonth();
    }
  }
  return false;
}

function mergeDxf(base: Dxf, overlay: Dxf): Dxf {
  return {
    fontColor: base.fontColor ?? overlay.fontColor,
    bold: base.bold ?? overlay.bold,
    italic: base.italic ?? overlay.italic,
    strike: base.strike ?? overlay.strike,
    underline: base.underline ?? overlay.underline,
    underlineStyle: base.underlineStyle ?? overlay.underlineStyle,
    fillColor: base.fillColor ?? overlay.fillColor,
    numFmt: base.numFmt ?? overlay.numFmt,
  };
}

function evaluateCellIs(
  cell: Cell | undefined,
  operator: string | undefined,
  operands: string[],
  _layout: WorkbookLayout,
): boolean {
  if (!cell || !operator || operands.length === 0) return false;
  const cellNum = cellNumericValue(cell);
  const cellStr = cellTextValue(cell, _layout);
  const a = parseCfOperand(operands[0]!);
  const b = operands.length > 1 ? parseCfOperand(operands[1]!) : undefined;
  if (a === null) return false;

  const cellIsText = cellNum === null && cellStr.length > 0;
  const aNum = typeof a === "number" ? a : NaN;
  const bNum = b !== undefined && typeof b === "number" ? b : NaN;

  const cmp = (
    lhsNum: number | null,
    lhsStr: string,
    op: string,
    rhsNum: number,
    rhsIsStr: boolean,
    rhsStr: string,
  ): boolean | null => {
    if (rhsIsStr) {
      if (lhsNum !== null) return op === "notEqual";
      switch (op) {
        case "equal":
          return lhsStr === rhsStr;
        case "notEqual":
          return lhsStr !== rhsStr;
        case "greaterThan":
          return lhsStr > rhsStr;
        case "greaterThanOrEqual":
          return lhsStr >= rhsStr;
        case "lessThan":
          return lhsStr < rhsStr;
        case "lessThanOrEqual":
          return lhsStr <= rhsStr;
      }
      return false;
    }

    const lhs = lhsNum !== null ? lhsNum : lhsStr.length > 0 ? Infinity : null;
    if (lhs === null) return null;
    switch (op) {
      case "equal":
        return lhs === rhsNum;
      case "notEqual":
        return lhs !== rhsNum;
      case "greaterThan":
        return lhs > rhsNum;
      case "greaterThanOrEqual":
        return lhs >= rhsNum;
      case "lessThan":
        return lhs < rhsNum;
      case "lessThanOrEqual":
        return lhs <= rhsNum;
    }
    return false;
  };

  switch (operator) {
    case "equal":
    case "notEqual":
    case "greaterThan":
    case "greaterThanOrEqual":
    case "lessThan":
    case "lessThanOrEqual":
      return (
        cmp(
          cellNum,
          cellStr,
          operator,
          aNum,
          typeof a === "string",
          typeof a === "string" ? a : "",
        ) === true
      );
    case "between":
    case "notBetween": {
      if (b === undefined) return false;

      if (typeof a !== "number" || typeof b !== "number") return false;
      const lo = Math.min(aNum, bNum),
        hi = Math.max(aNum, bNum);

      if (cellIsText) return operator === "notBetween";
      if (cellNum === null) return false;
      const inside = cellNum >= lo && cellNum <= hi;
      return operator === "between" ? inside : !inside;
    }
  }
  return false;
}

function parseCfOperand(s: string): number | string | null {
  const t = s.trim();
  if (t.length === 0) return null;
  if (t.startsWith('"') && t.endsWith('"')) {
    return t.slice(1, -1).replace(/""/g, '"');
  }

  const body = t.startsWith("=") ? t.slice(1).trim() : t;
  if (/^-?\d+(\.\d+)?([eE][-+]?\d+)?$/.test(body)) {
    const n = parseFloat(body);
    return Number.isNaN(n) ? null : n;
  }
  return null;
}

export function drawConditionalFormats(
  ctx: CanvasRenderingContext2D,
  sheet: Sheet,
  layout: WorkbookLayout,
  g: Grid,
  vis: Visible,
  cfDxfs: Map<string, Dxf>,
  locks?: Map<string, number>,
): void {
  if (cfDxfs.size > 0) {
    const { covered, topLeftOf } = buildMergeMaps(sheet);
    for (let r = vis.firstRow; r <= vis.lastRow; r++) {
      for (let c = vis.firstCol; c <= vis.lastCol; c++) {
        const k = `${r}:${c}`;
        const dxf = cfDxfs.get(k);
        if (!dxf || !dxf.fillColor) continue;
        if (covered.has(k)) continue;
        const rect = topLeftOf.has(k) ? mergedRect(g, topLeftOf.get(k)!) : cellRect(g, r, c);
        ctx.fillStyle = colorToCss(dxf.fillColor, "#ffffff");
        ctx.fillRect(rect.x, rect.y, rect.w, rect.h);
      }
    }
  }

  const cfs = sheet.conditionalFormats;
  if (!cfs || cfs.length === 0) return;
  const { covered, topLeftOf } = buildMergeMaps(sheet);

  const cellNumeric = getNumericCellMap(sheet);

  for (const cf of cfs) {
    const rule = cf.rules
      .filter((r) => r.kind === "colorScale" && r.colorScale)
      .sort((a, b) => a.priority - b.priority)[0];
    if (!rule || !rule.colorScale) continue;

    let stops = colorScaleStopsCache.get(rule);
    if (!stops) {
      const values = numericValuesInRanges(cellNumeric, cf.ranges);
      if (values.length === 0) continue;
      stops = resolveColorScaleStops(rule.colorScale, values);
      colorScaleStopsCache.set(rule, stops);
    }
    if (stops.length < 2) continue;

    for (const range of cf.ranges) {
      const r1 = Math.max(range.r1, vis.firstRow);
      const r2 = Math.min(range.r2, vis.lastRow);
      const c1 = Math.max(range.c1, vis.firstCol);
      const c2 = Math.min(range.c2, vis.lastCol);
      for (let r = r1; r <= r2; r++) {
        for (let c = c1; c <= c2; c++) {
          const k = `${r}:${c}`;
          if (covered.has(k)) continue;
          if (isCfLocked(locks, k, rule.priority)) continue;
          const v = cellNumeric.get(k);
          if (v === undefined) continue;
          const css = interpolateStops(stops, v);
          if (!css) continue;
          const rect = topLeftOf.has(k) ? mergedRect(g, topLeftOf.get(k)!) : cellRect(g, r, c);
          ctx.fillStyle = css;
          ctx.fillRect(rect.x, rect.y, rect.w, rect.h);
        }
      }
    }
  }

  for (const cf of cfs) {
    const rule = cf.rules
      .filter((r) => r.kind === "dataBar" && r.dataBar)
      .sort((a, b) => a.priority - b.priority)[0];
    if (!rule || !rule.dataBar) continue;
    const db = rule.dataBar;

    let bounds = dataBarBoundsCache.get(rule);
    if (bounds === undefined) {
      const values = numericValuesInRanges(cellNumeric, cf.ranges);
      if (values.length === 0) {
        dataBarBoundsCache.set(rule, null);
        continue;
      }
      const dataMin = Math.min(...values);
      const dataMax = Math.max(...values);
      const sorted = [...values].sort((a, b) => a - b);
      const minVal = resolveCfvoValue(db.min, dataMin, dataMax, sorted, true);
      const maxVal = resolveCfvoValue(db.max, dataMin, dataMax, sorted, false);
      bounds = isFinite(minVal) && isFinite(maxVal) && maxVal > minVal ? { minVal, maxVal } : null;
      dataBarBoundsCache.set(rule, bounds);
    }
    if (!bounds) continue;
    const { minVal, maxVal } = bounds;

    const minPct = (db.minLengthPct ?? 10) / 100;
    const maxPct = (db.maxLengthPct ?? 90) / 100;
    const posCss = colorToCss(db.color, "#638EC6");
    const negCss = db.negativeColor ? colorToCss(db.negativeColor, "#FF0000") : "#FF0000";

    const straddles = minVal < 0 && maxVal > 0;
    const axisFrac = straddles ? -minVal / (maxVal - minVal) : 0;

    for (const range of cf.ranges) {
      const r1 = Math.max(range.r1, vis.firstRow);
      const r2 = Math.min(range.r2, vis.lastRow);
      const c1 = Math.max(range.c1, vis.firstCol);
      const c2 = Math.min(range.c2, vis.lastCol);
      for (let r = r1; r <= r2; r++) {
        for (let c = c1; c <= c2; c++) {
          const k = `${r}:${c}`;
          if (covered.has(k)) continue;
          if (isCfLocked(locks, k, rule.priority)) continue;
          const v = cellNumeric.get(k);
          if (v === undefined) continue;
          const rect = topLeftOf.has(k) ? mergedRect(g, topLeftOf.get(k)!) : cellRect(g, r, c);

          const inset = 1;
          const bx = rect.x + inset;
          const by = rect.y + inset;
          const bw = Math.max(0, rect.w - inset * 2);
          const bh = Math.max(0, rect.h - inset * 2);
          if (bw <= 0 || bh <= 0) continue;

          const fillBar = (
            x: number,
            y: number,
            w: number,
            h: number,
            css: string,
            anchor: "left" | "right",
          ) => {
            if (w <= 0 || h <= 0) return;
            if (db.gradient !== false) {
              const x0 = anchor === "left" ? x : x + w;
              const x1 = anchor === "left" ? x + w : x;
              const grad = ctx.createLinearGradient(x0, y, x1, y);
              grad.addColorStop(0, withAlpha(css, 1.0));
              grad.addColorStop(0.7, withAlpha(css, 0.8));
              grad.addColorStop(1, withAlpha(css, 0.05));
              ctx.fillStyle = grad;
            } else {
              ctx.fillStyle = css;
            }
            ctx.fillRect(x, y, w, h);
          };

          if (straddles) {
            const axisX = bx + bw * axisFrac;
            if (v >= 0) {
              const t = Math.min(1, v / maxVal);
              const len = bw * (1 - axisFrac) * (minPct + t * (maxPct - minPct));
              fillBar(axisX, by, len, bh, posCss, "left");
            } else {
              const t = Math.min(1, -v / -minVal);
              const len = bw * axisFrac * (minPct + t * (maxPct - minPct));
              fillBar(axisX - len, by, len, bh, negCss, "right");
            }

            ctx.fillStyle = "#000000";
            ctx.fillRect(Math.round(axisX) - 0.5, by, 1, bh);
          } else {
            const t = Math.max(0, Math.min(1, (v - minVal) / (maxVal - minVal)));
            const len = bw * (minPct + t * (maxPct - minPct));
            fillBar(bx, by, len, bh, posCss, "left");
          }
        }
      }
    }
  }
}

const colorScaleStopsCache = new WeakMap<CfRule, ReturnType<typeof resolveColorScaleStops>>();
const dataBarBoundsCache = new WeakMap<CfRule, { minVal: number; maxVal: number } | null>();

const numericCellMapCache = new WeakMap<Sheet, Map<string, number>>();
export function getNumericCellMap(sheet: Sheet): Map<string, number> {
  const hit = numericCellMapCache.get(sheet);
  if (hit) return hit;
  const cellNumeric = new Map<string, number>();
  iterAllCells(sheet, (cell) => {
    if (cell.value === undefined) return;
    if (cell.type === "n" || cell.type === "f") {
      const n = parseFloat(cell.value);
      if (!Number.isNaN(n)) cellNumeric.set(`${cell.r}:${cell.c}`, n);
    }
  });
  numericCellMapCache.set(sheet, cellNumeric);
  return cellNumeric;
}

export function resolveCfvoValue(
  s: CfvoStop,
  dataMin: number,
  dataMax: number,
  sorted: number[],
  isMin: boolean,
): number {
  switch (s.type) {
    case "min":
    case "automin":
      return Math.min(0, dataMin);
    case "max":
    case "automax":
      return Math.max(0, dataMax);
    case "num":
    case "formula":
      return parseFloat(s.val ?? (isMin ? `${dataMin}` : `${dataMax}`));
    case "percent": {
      const p = parseFloat(s.val ?? (isMin ? "0" : "100")) / 100;
      return dataMin + (dataMax - dataMin) * p;
    }
    case "percentile": {
      const p = parseFloat(s.val ?? (isMin ? "0" : "100")) / 100;
      if (sorted.length === 0) return isMin ? dataMin : dataMax;
      const idx = Math.min(sorted.length - 1, Math.max(0, Math.round(p * (sorted.length - 1))));
      return sorted[idx] ?? (isMin ? dataMin : dataMax);
    }
    default:
      return isMin ? dataMin : dataMax;
  }
}

export { computeCfIconState } from "./cfIconState.js";

export function computeCfTextSuppress(sheet: Sheet, locks?: Map<string, number>): Set<string> {
  const out = new Set<string>();
  const cfs = sheet.conditionalFormats;
  if (!cfs || cfs.length === 0) return out;
  for (const cf of cfs) {
    const rule = cf.rules
      .filter((r) => r.kind === "dataBar" && r.dataBar && r.dataBar.showValue === false)
      .sort((a, b) => a.priority - b.priority)[0];
    if (!rule) continue;
    for (const range of cf.ranges) {
      const r1 = Math.max(1, range.r1);
      const r2 = Math.min(sheet.maxRow, range.r2);
      const c1 = Math.max(1, range.c1);
      const c2 = Math.min(sheet.maxCol, range.c2);
      for (let r = r1; r <= r2; r++) {
        for (let c = c1; c <= c2; c++) {
          const k = `${r}:${c}`;
          if (isCfLocked(locks, k, rule.priority)) continue;
          out.add(k);
        }
      }
    }
  }
  return out;
}
