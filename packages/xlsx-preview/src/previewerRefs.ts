import { colLabel } from "./grid.js";
import type { Selection } from "./interact.js";
import type { HighlightRange } from "./renderTypes.js";
import type { WorkbookLayout } from "./types.js";

export function colNameToIndex(s: string): number {
  let n = 0;
  for (const ch of s.toUpperCase()) n = n * 26 + (ch.charCodeAt(0) - 64);
  return n;
}

export function findUnquotedBang(s: string): number {
  let quoted = false;
  for (let i = 0; i < s.length; i++) {
    const ch = s[i];
    if (ch === "'") {
      if (quoted && s[i + 1] === "'") i++;
      else quoted = !quoted;
    } else if (ch === "!" && !quoted) return i;
  }
  return -1;
}

export function unquoteSheetName(s: string): string {
  const trimmed = s.trim();
  if (trimmed.startsWith("'") && trimmed.endsWith("'")) {
    return trimmed.slice(1, -1).replace(/''/g, "'");
  }
  return trimmed;
}

export function parseSheetCellLocation(
  raw: string,
  layout: WorkbookLayout,
  fallbackSheetIndex: number,
): { sheetIndex: number; r: number; c: number } | null {
  const ref = raw.trim().replace(/^=/, "");
  const bang = findUnquotedBang(ref);
  let sheetIndex = fallbackSheetIndex;
  let addr = ref;
  if (bang >= 0) {
    const sheetName = unquoteSheetName(ref.slice(0, bang));
    const idx = layout.sheets.findIndex((s) => s.name === sheetName);
    if (idx < 0) return null;
    sheetIndex = idx;
    addr = ref.slice(bang + 1);
  }
  const m = addr.match(/\$?([A-Za-z]{1,3})\$?(\d+)/);
  if (!m) return null;
  return { sheetIndex, r: Number(m[2]), c: colNameToIndex(m[1]!) };
}

export function parseSheetRangeLocation(
  raw: string,
  layout: WorkbookLayout,
  fallbackSheetIndex: number,
): { sheetIndex: number; r1: number; c1: number; r2: number; c2: number } | null {
  const ref = raw.trim().replace(/^=/, "");
  const bang = findUnquotedBang(ref);
  let sheetIndex = fallbackSheetIndex;
  let addr = ref;
  if (bang >= 0) {
    const sheetName = unquoteSheetName(ref.slice(0, bang));
    const idx = layout.sheets.findIndex((s) => s.name === sheetName);
    if (idx < 0) return null;
    sheetIndex = idx;
    addr = ref.slice(bang + 1);
  }
  const parts = addr.split(":");
  const cellRe = /^\$?([A-Za-z]{1,3})\$?(\d+)$/;
  const a = cellRe.exec(parts[0]!.trim());
  if (!a) return null;
  const ra = Number(a[2]);
  const ca = colNameToIndex(a[1]!);
  let rb = ra;
  let cb = ca;
  if (parts.length === 2) {
    const b = cellRe.exec(parts[1]!.trim());
    if (!b) return null;
    rb = Number(b[2]);
    cb = colNameToIndex(b[1]!);
  } else if (parts.length !== 1) {
    return null;
  }
  return {
    sheetIndex,
    r1: Math.min(ra, rb),
    c1: Math.min(ca, cb),
    r2: Math.max(ra, rb),
    c2: Math.max(ca, cb),
  };
}

export function resolveWorkbookLocation(
  layout: WorkbookLayout,
  rawLocation: string,
  activeSheetIndex: number,
): { sheetIndex: number; r: number; c: number } | null {
  const location = rawLocation.trim().replace(/^#/, "");
  const direct = parseSheetCellLocation(location, layout, activeSheetIndex);
  if (direct) return direct;

  const wanted = location.toLocaleLowerCase();
  const names = layout.definedNames ?? [];
  const local = names.find(
    (n) => n.name.toLocaleLowerCase() === wanted && n.localSheetId === activeSheetIndex,
  );
  const global = names.find(
    (n) => n.name.toLocaleLowerCase() === wanted && n.localSheetId === undefined,
  );
  const named = local ?? global;
  if (!named) return null;
  return parseSheetCellLocation(named.formula, layout, named.localSheetId ?? activeSheetIndex);
}

export function matchNamedRange(
  selection: Selection,
  layout: WorkbookLayout,
  activeSheetIndex: number,
): string | null {
  const names = layout.definedNames ?? [];
  for (const n of names) {
    if (n.localSheetId !== undefined && n.localSheetId !== activeSheetIndex) continue;
    const range = parseSheetRangeLocation(n.formula, layout, n.localSheetId ?? activeSheetIndex);
    if (!range) continue;
    if (range.sheetIndex !== activeSheetIndex) continue;
    if (
      range.r1 === selection.r1 &&
      range.c1 === selection.c1 &&
      range.r2 === selection.r2 &&
      range.c2 === selection.c2
    ) {
      return n.name;
    }
  }
  return null;
}

export function formatNameBox(
  active: { r: number; c: number },
  selection: Selection,
  layout: WorkbookLayout,
  activeSheetIndex: number,
): string {
  const named = matchNamedRange(selection, layout, activeSheetIndex);
  if (named) return named;
  if (selection.r1 !== selection.r2 || selection.c1 !== selection.c2) {
    return `${colLabel(active.c)}${active.r}  (${selection.r2 - selection.r1 + 1}R×${selection.c2 - selection.c1 + 1}C)`;
  }
  return colLabel(active.c) + active.r;
}

export function parsePointHighlight(ref: string, color: string): HighlightRange | null {
  const cellRe = /^\$?([A-Za-z]+)\$?(\d+)$/;
  const parts = ref.split(":");
  if (parts.length === 1) {
    const m = cellRe.exec(parts[0]!.trim());
    if (!m) return null;
    const c = colNameToIndex(m[1]!);
    const r = Number(m[2]);
    return { r1: r, c1: c, r2: r, c2: c, color };
  }
  if (parts.length === 2) {
    const a = cellRe.exec(parts[0]!.trim());
    const b = cellRe.exec(parts[1]!.trim());
    if (!a || !b) return null;
    const ca = colNameToIndex(a[1]!);
    const ra = Number(a[2]);
    const cb = colNameToIndex(b[1]!);
    const rb = Number(b[2]);
    return {
      r1: Math.min(ra, rb),
      c1: Math.min(ca, cb),
      r2: Math.max(ra, rb),
      c2: Math.max(ca, cb),
      color,
    };
  }
  return null;
}
