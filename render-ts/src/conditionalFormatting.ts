import type {
  Cell,
  CfColorScale,
  CfvoStop,
  CfRule,
  Color,
  Dxf,
  Merge,
  Sheet,
  WorkbookLayout,
} from "./types.js";
import { cellNumericValue, cellTextValue } from "./cellText.js";
import { colorToCss } from "./color.js";
import { withAlpha } from "./chartUtils.js";
import { iterAllCells } from "./columnar.js";
import type { Grid } from "./grid.js";
import { buildMergeMaps, cellRect, mergedRect } from "./geometry.js";
import type { Visible } from "./renderTypes.js";

// ---------- conditional formatting ----------

// Kinds whose match set is value-driven (the rule only "applies" to
// cells whose value satisfies the predicate). All other kinds
// (colorScale / dataBar / iconSet / expression) either always apply
// across their full sqref (the visual passes paint a no-op for cells
// whose value isn't numeric) or need a formula engine.
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

/// Walk every CF rule across the whole sheet in priority order and
/// collect the cells "locked" by a `stopIfTrue` rule. Returns a map
/// from cellKey (`"r:c"`) to the priority value at which that cell is
/// locked — any subsequent rule with priority strictly greater than
/// the recorded value must be masked.
///
/// Cross-kind masking is the whole point: in Excel, a higher-priority
/// `cellIs` rule with stopIfTrue=true will suppress a lower-priority
/// colorScale on the same cell, and vice-versa. We treat colorScale /
/// dataBar / iconSet rules as matching every cell in their `sqref`
/// (Excel's UI doesn't let you set stopIfTrue on these, but the OOXML
/// schema allows it and writers in the wild do produce it).
/// `expression` rules need recalc to evaluate so they don't lock
/// anything today (better to under-mask than over-mask).
export function computeCfStopLocks(sheet: Sheet, layout: WorkbookLayout): Map<string, number> {
  const locks = new Map<string, number>();
  const cfs = sheet.conditionalFormats;
  if (!cfs || cfs.length === 0) return locks;

  const cellByKey = new Map<string, Cell>();
  iterAllCells(sheet, (cell) => {
    cellByKey.set(`${cell.r}:${cell.c}`, cell);
  });

  // Flatten rules across all CF blocks with their parent ranges, then
  // sort globally by priority (low = high precedence).
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
        for (let r = range.r1; r <= range.r2; r++) {
          for (let c = range.c1; c <= range.c2; c++) all.push(`${r}:${c}`);
        }
      }
      matched = all;
    } else {
      // expression / unknown: skip without locking.
      continue;
    }
    for (const k of matched) {
      const cur = locks.get(k);
      if (cur === undefined || rule.priority < cur) locks.set(k, rule.priority);
    }
  }
  return locks;
}

/// `true` if `rulePriority` should be masked at `cellKey` by some
/// higher-priority stopIfTrue rule.
export function isCfLocked(
  locks: Map<string, number> | undefined,
  cellKey: string,
  rulePriority: number,
): boolean {
  if (!locks) return false;
  const at = locks.get(cellKey);
  return at !== undefined && at < rulePriority;
}

/// Walk every CF rule once and build the merged dxf overlay per cell.
/// Today: only `cellIs` (with literal-numeric/literal-string operands) is
/// evaluated; `expression` and friends need a formula engine and are
/// skipped. Honors rule priority (lower number wins) and `stopIfTrue`,
/// including cross-kind masking via `locks` (e.g. a higher-priority
/// stopIfTrue colorScale will suppress a lower-priority dxf overlay).
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

  // In-pass dxf-overlay locks layered on top of the cross-kind locks.
  // The cross-kind `locks` only contains stopIfTrue rules; an in-pass
  // match by a non-stopping higher-priority rule still wins per-field
  // against same-cell overlaps via mergeDxf.
  const locallyLocked = new Set<string>();

  for (const cf of cfs) {
    // Walk this block's rules in priority order (low number = high prio).
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

/// Walk every cell covered by `ranges` and return the keys (`"r:c"`)
/// that satisfy the rule. Caller handles priority + stopIfTrue.
function computeRuleMatchSet(
  rule: CfRule,
  ranges: Merge[],
  cellByKey: Map<string, Cell>,
  layout: WorkbookLayout,
): Set<string> {
  const out = new Set<string>();

  // Collect every (key, cell) pair this rule covers, once.
  const covered: { k: string; cell: Cell | undefined }[] = [];
  for (const range of ranges) {
    for (let r = range.r1; r <= range.r2; r++) {
      for (let c = range.c1; c <= range.c2; c++) {
        const k = `${r}:${c}`;
        covered.push({ k, cell: cellByKey.get(k) });
      }
    }
  }

  switch (rule.kind) {
    case "cellIs": {
      for (const { k, cell } of covered) {
        if (evaluateCellIs(cell, rule.operator, rule.operands, layout)) out.add(k);
      }
      break;
    }
    case "top10": {
      // Rank numeric cells; non-numerics never match.
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
        // Excel: ceil(count * pct/100), clamped to [1, count].
        const pct = Math.max(0, Math.min(100, rankRaw));
        n = Math.max(1, Math.min(nums.length, Math.ceil((nums.length * pct) / 100)));
      } else {
        n = Math.max(1, Math.min(nums.length, rankRaw));
      }
      // Sort: bottom=true → ascending (smallest N), else descending.
      nums.sort((a, b) => (rule.bottom ? a.v - b.v : b.v - a.v));
      // Tie at the cutoff value also matches (Excel includes ties).
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
        // Population stdev (Excel uses N, not N-1, for CF aboveAverage).
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
      // Bucket on a normalized value (text vs number kept distinct so
      // "1" and 1 don't collide; mirrors Excel behavior).
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
      // Excel does case-insensitive substring/prefix/suffix match on the
      // displayed text. Empty needle ⇒ never matches (Excel won't let you
      // create such a rule, but be defensive).
      const needle = (rule.text ?? "").toLowerCase();
      if (needle.length === 0) break;
      for (const { k, cell } of covered) {
        if (!cell) {
          // Empty cells: notContainsText matches them; others don't.
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
      // Match against today (real wall-clock at render time).
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

/// Convert an Excel serial date (days since 1900-01-00, with the
/// Lotus 1900-leap-year bug) to a JS Date. Returns null for negative
/// or NaN inputs. Mirrors the simple formula used elsewhere in the
/// renderer; we only need day-precision for `timePeriod`.
function excelSerialToDate(serial: number): Date | null {
  if (!isFinite(serial) || serial < 0) return null;
  // Excel epoch: 1899-12-30 (accounts for the fictitious 1900-02-29).
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
      // Excel weeks: Sunday–Saturday.
      const tow = today.getDay(); // 0=Sun
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

/// Field-by-field merge: `base` (higher-priority) wins; `overlay` fills
/// any gaps. Used when multiple non-stopping rules cover the same cell.
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

/// Evaluate one `cellIs` rule against a cell. Operands are CF formulas;
/// without a recalc engine we only handle the cases that don't need one:
/// literal numbers (`5`, `-1.5`) and double-quoted strings (`"foo"`).
/// Anything else (e.g. `A1`, `SUM(B:B)`) returns false — better to skip
/// the highlight than guess wrong.
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

  // Excel ordering quirk: when comparing text against a numeric operand,
  // text always sorts "greater than" any number. So `notEqual 50` matches
  // a "foo" cell, `greaterThan 50` matches "foo", `lessThan 50` does not,
  // `between 10 100` does not include "foo", `notBetween` does.
  const cellIsText = cellNum === null && cellStr.length > 0;
  const aNum = typeof a === "number" ? a : NaN;
  const bNum = b !== undefined && typeof b === "number" ? b : NaN;

  // For numeric operators against a text cell vs numeric operand, treat
  // the text as +Infinity (Excel's text > number rule). String operands
  // against text cells use lexicographic compare.
  const cmp = (
    lhsNum: number | null,
    lhsStr: string,
    op: string,
    rhsNum: number,
    rhsIsStr: boolean,
    rhsStr: string,
  ): boolean | null => {
    if (rhsIsStr) {
      // Operand is a string — only equal/notEqual make semantic sense; for
      // ordered ops fall through using lexicographic compare on text cells.
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
    // Numeric operand. Text cells get the +Infinity treatment.
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
      // Both bounds must parse to numbers for between to make sense.
      if (typeof a !== "number" || typeof b !== "number") return false;
      const lo = Math.min(aNum, bNum),
        hi = Math.max(aNum, bNum);
      // Text cells are +Infinity — outside any numeric range.
      if (cellIsText) return operator === "notBetween";
      if (cellNum === null) return false;
      const inside = cellNum >= lo && cellNum <= hi;
      return operator === "between" ? inside : !inside;
    }
  }
  return false;
}

/// CF operands come in as raw formula text. Recognize:
///   `5`, `-1.5`, `1e3` → number
///   `"foo"`            → string (un-escape `""` per OOXML)
/// Everything else (cell refs, function calls) returns `null`.
function parseCfOperand(s: string): number | string | null {
  const t = s.trim();
  if (t.length === 0) return null;
  if (t.startsWith('"') && t.endsWith('"')) {
    return t.slice(1, -1).replace(/""/g, '"');
  }
  // Strip a leading `=` some writers emit.
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
  // First pass: paint dxf fill rects for cellIs / expression matches.
  // (Color-scale fills paint below from their own min/max plumbing.)
  if (cfDxfs.size > 0) {
    const { covered, topLeftOf } = buildMergeMaps(sheet);
    for (const [k, dxf] of cfDxfs) {
      if (!dxf.fillColor) continue;
      if (covered.has(k)) continue;
      const [rs, cs] = k.split(":");
      const r = parseInt(rs!, 10),
        c = parseInt(cs!, 10);
      if (r < vis.firstRow || r > vis.lastRow) continue;
      if (c < vis.firstCol || c > vis.lastCol) continue;
      const rect = topLeftOf.has(k) ? mergedRect(g, topLeftOf.get(k)!) : cellRect(g, r, c);
      ctx.fillStyle = colorToCss(dxf.fillColor, "#ffffff");
      ctx.fillRect(rect.x, rect.y, rect.w, rect.h);
    }
  }

  const cfs = sheet.conditionalFormats;
  if (!cfs || cfs.length === 0) return;
  const { covered, topLeftOf } = buildMergeMaps(sheet);

  // Index numeric values so each color-scale rule only computes its range
  // bounds once.
  const cellNumeric = new Map<string, number>();
  iterAllCells(sheet, (cell) => {
    if (cell.value === undefined) return;
    if (cell.type === "n" || cell.type === "f") {
      const n = parseFloat(cell.value);
      if (!Number.isNaN(n)) cellNumeric.set(`${cell.r}:${cell.c}`, n);
    }
  });

  for (const cf of cfs) {
    // Highest-priority color-scale rule wins (lower number = higher priority).
    const rule = cf.rules
      .filter((r) => r.kind === "colorScale" && r.colorScale)
      .sort((a, b) => a.priority - b.priority)[0];
    if (!rule || !rule.colorScale) continue;

    // Gather all numeric values inside this CF's ranges to compute min/max.
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

    const stops = resolveColorScaleStops(rule.colorScale, values);
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

  // Data-bar pass. Paints horizontal bars proportional to value within
  // each rule's CFVO-derived [min,max] window. When the data range
  // straddles zero the axis sits at |min|/(|min|+max) and negatives
  // paint left of the axis in red.
  for (const cf of cfs) {
    const rule = cf.rules
      .filter((r) => r.kind === "dataBar" && r.dataBar)
      .sort((a, b) => a.priority - b.priority)[0];
    if (!rule || !rule.dataBar) continue;
    const db = rule.dataBar;

    // Numeric values inside the rule's ranges drive the auto min/max.
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
    const minVal = resolveCfvoValue(db.min, dataMin, dataMax, sorted, /*isMin*/ true);
    const maxVal = resolveCfvoValue(db.max, dataMin, dataMax, sorted, /*isMin*/ false);
    if (!isFinite(minVal) || !isFinite(maxVal) || maxVal <= minVal) continue;

    const minPct = (db.minLengthPct ?? 10) / 100;
    const maxPct = (db.maxLengthPct ?? 90) / 100;
    const posCss = colorToCss(db.color, "#638EC6");
    const negCss = db.negativeColor ? colorToCss(db.negativeColor, "#FF0000") : "#FF0000";

    // Split-axis when range straddles zero. axisFrac is where the zero
    // line sits as a fraction of the bar's length (0 = leftmost, 1 =
    // rightmost). We confine the bar to the cell's available width and
    // place the axis at axisFrac inside that.
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
          // Inset 1px so the bar sits inside grid lines.
          const inset = 1;
          const bx = rect.x + inset;
          const by = rect.y + inset;
          const bw = Math.max(0, rect.w - inset * 2);
          const bh = Math.max(0, rect.h - inset * 2);
          if (bw <= 0 || bh <= 0) continue;

          // `gradient` (Excel 2010+ default) paints a `linear-gradient(
          // color, color->transparent)` from the bar's anchor edge to
          // its outer tip; solid mode paints a flat fill of the same
          // color across the whole bar. Excel's gradient stops aren't
          // documented; visually-matched approximation: full-opacity at
          // the anchor end, ~5% opacity at the tip, with the curve held
          // mostly flat (~80% opacity at 70% of the bar) so the bar
          // still reads as solid color from a distance.
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
            // Thin axis tick (Excel paints a 1px black line at zero).
            ctx.fillStyle = "#000000";
            ctx.fillRect(Math.round(axisX) - 0.5, by, 1, bh);
          } else {
            // Single-direction bar from the left edge.
            const t = Math.max(0, Math.min(1, (v - minVal) / (maxVal - minVal)));
            const len = bw * (minPct + t * (maxPct - minPct));
            fillBar(bx, by, len, bh, posCss, "left");
          }
        }
      }
    }
  }
}

/// Resolve a `<cfvo>` stop to a numeric threshold against the data range.
/// `min`/`max` map to "Lowest Value"/"Highest Value" in Excel's UI; the
/// canonical x14 extension records these as `automin`/`automax`, which
/// per ECMA-376 anchor at zero (a positive-only range starts the bar from
/// 0, not from the actual data min). We don't parse x14 yet, so we apply
/// the same zero-clamp to the legacy `min`/`max` types: a strict reading
/// of the spec would skip this clamp, but every Excel/SpreadJS-authored
/// file we've seen uses x14, so this matches what users actually see.
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

/// CF iconSet pre-pass. For each cell covered by an iconSet rule:
///   - decide which icon (0..N-1) to draw, based on the value vs the
///     resolved CFVO thresholds;
///   - reserve `ICON_RESERVE_PX` at the cell's left for the glyph;
///   - if `showValue=false`, mark the cell text-suppressed.
///
/// Multiple iconSet rules on the same cell are resolved by `priority`
/// (lower = higher precedence), matching `dataBar` / `colorScale`.

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
      for (let r = range.r1; r <= range.r2; r++) {
        for (let c = range.c1; c <= range.c2; c++) {
          const k = `${r}:${c}`;
          if (isCfLocked(locks, k, rule.priority)) continue;
          out.add(k);
        }
      }
    }
  }
  return out;
}

interface ResolvedStop {
  value: number;
  rgb: [number, number, number];
}

function resolveColorScaleStops(cs: CfColorScale, values: number[]): ResolvedStop[] {
  const min = Math.min(...values);
  const max = Math.max(...values);
  const sorted = [...values].sort((a, b) => a - b);
  return cs.stops
    .map((s) => {
      let v: number;
      switch (s.type) {
        case "min":
          v = min;
          break;
        case "max":
          v = max;
          break;
        case "num":
          v = parseFloat(s.val ?? "0");
          break;
        case "percent": {
          const p = parseFloat(s.val ?? "0") / 100;
          v = min + (max - min) * p;
          break;
        }
        case "percentile": {
          const p = parseFloat(s.val ?? "0") / 100;
          const idx = Math.min(sorted.length - 1, Math.max(0, Math.round(p * (sorted.length - 1))));
          v = sorted[idx] ?? min;
          break;
        }
        // formula CFVOs need an evaluator; punt for v0.
        default:
          v = min;
      }
      return { value: v, rgb: rgbTriple(s.color) };
    })
    .sort((a, b) => a.value - b.value);
}

function rgbTriple(c: Color): [number, number, number] {
  const css = colorToCss(c, "#ffffff");
  return [
    parseInt(css.slice(1, 3), 16),
    parseInt(css.slice(3, 5), 16),
    parseInt(css.slice(5, 7), 16),
  ];
}

function interpolateStops(stops: ResolvedStop[], value: number): string | null {
  if (stops.length === 0) return null;
  const first = stops[0]!;
  const last = stops[stops.length - 1]!;
  if (value <= first.value) return rgbToCss(first.rgb);
  if (value >= last.value) return rgbToCss(last.rgb);
  for (let i = 0; i < stops.length - 1; i++) {
    const a = stops[i]!;
    const b = stops[i + 1]!;
    if (value >= a.value && value <= b.value) {
      const span = b.value - a.value;
      const t = span === 0 ? 0 : (value - a.value) / span;
      const r = Math.round(a.rgb[0] + (b.rgb[0] - a.rgb[0]) * t);
      const gg = Math.round(a.rgb[1] + (b.rgb[1] - a.rgb[1]) * t);
      const bb = Math.round(a.rgb[2] + (b.rgb[2] - a.rgb[2]) * t);
      return `rgb(${r}, ${gg}, ${bb})`;
    }
  }
  return null;
}

function rgbToCss(rgb: [number, number, number]): string {
  return `rgb(${rgb[0]}, ${rgb[1]}, ${rgb[2]})`;
}
