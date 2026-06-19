import { resolveCellText, resolveCellXf } from "./cellText.js";
import { findCell } from "./columnar.js";
import { colLabel } from "./grid.js";
import type { Selection } from "./interact.js";
import type { Sheet, WorkbookLayout } from "./types.js";

export interface SerializedRange {
  tsv: string;
  html: string;
}

interface XlcorePayload {
  source: "xlcore";
  sheet: string;
  range: string;
  values: string[][];
  formulas: (string | null)[][];
}

export interface ParsedClipboard {
  values: string[][];
  formulas?: (string | null)[][];
  source: "internal" | "external";
  sourceSheet?: string;
  sourceRange?: string;
}

function normalize(sel: Selection): { r1: number; c1: number; r2: number; c2: number } {
  return {
    r1: Math.min(sel.r1, sel.r2),
    c1: Math.min(sel.c1, sel.c2),
    r2: Math.max(sel.r1, sel.r2),
    c2: Math.max(sel.c1, sel.c2),
  };
}

function cellDisplay(sheet: Sheet, layout: WorkbookLayout, r: number, c: number): string {
  const cell = findCell(sheet, r, c);
  if (!cell) return "";
  const xf = resolveCellXf(cell, sheet, layout);
  return resolveCellText(cell, layout, xf).text;
}

function cellFormula(sheet: Sheet, r: number, c: number): string | null {
  const cell = findCell(sheet, r, c);
  if (!cell || cell.formula === undefined) return null;
  return cell.formula.startsWith("=") ? cell.formula : `=${cell.formula}`;
}

function rangeRef(n: { r1: number; c1: number; r2: number; c2: number }): string {
  const tl = `${colLabel(n.c1)}${n.r1}`;
  const br = `${colLabel(n.c2)}${n.r2}`;
  return tl === br ? tl : `${tl}:${br}`;
}

function quoteTsvField(field: string): string {
  if (/[\t\n\r"]/.test(field)) {
    return `"${field.replace(/"/g, '""')}"`;
  }
  return field;
}

function toTsv(values: string[][]): string {
  return values.map((row) => row.map(quoteTsvField).join("\t")).join("\n");
}

function escapeHtml(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}

function escapeAttr(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

function decodeEntities(s: string): string {
  return s
    .replace(/&quot;/g, '"')
    .replace(/&#39;/g, "'")
    .replace(/&apos;/g, "'")
    .replace(/&lt;/g, "<")
    .replace(/&gt;/g, ">")
    .replace(/&#(\d+);/g, (_, d: string) => String.fromCodePoint(Number(d)))
    .replace(/&amp;/g, "&");
}

export function serializeRange(
  layout: WorkbookLayout,
  sheetName: string,
  selection: Selection,
): SerializedRange {
  const sheet = layout.sheets.find((s) => s.name === sheetName) as Sheet | undefined;
  const n = normalize(selection);
  const values: string[][] = [];
  const formulas: (string | null)[][] = [];
  if (sheet) {
    for (let r = n.r1; r <= n.r2; r++) {
      const rowVals: string[] = [];
      const rowFms: (string | null)[] = [];
      for (let c = n.c1; c <= n.c2; c++) {
        rowVals.push(cellDisplay(sheet, layout, r, c));
        rowFms.push(cellFormula(sheet, r, c));
      }
      values.push(rowVals);
      formulas.push(rowFms);
    }
  }
  const payload: XlcorePayload = {
    source: "xlcore",
    sheet: sheetName,
    range: rangeRef(n),
    values,
    formulas,
  };
  const rows = values
    .map((row) => `<tr>${row.map((v) => `<td>${escapeHtml(v)}</td>`).join("")}</tr>`)
    .join("");
  const html = `<table data-xlcore="${escapeAttr(JSON.stringify(payload))}"><tbody>${rows}</tbody></table>`;
  return { tsv: toTsv(values), html };
}

function extractPayload(html: string): XlcorePayload | null {
  const m = /data-xlcore="([^"]*)"/.exec(html);
  if (!m || m[1] === undefined) return null;
  try {
    const parsed = JSON.parse(decodeEntities(m[1])) as XlcorePayload;
    if (parsed && parsed.source === "xlcore" && Array.isArray(parsed.values)) return parsed;
  } catch {
    return null;
  }
  return null;
}

function parseHtmlTable(html: string): string[][] | null {
  const tableMatch = /<table[\s\S]*?<\/table>/i.exec(html);
  if (!tableMatch) return null;
  const table = tableMatch[0];
  const values: string[][] = [];
  const rowRe = /<tr[^>]*>([\s\S]*?)<\/tr>/gi;
  let rowM: RegExpExecArray | null;
  while ((rowM = rowRe.exec(table)) !== null) {
    const rowHtml = rowM[1] ?? "";
    const cells: string[] = [];
    const cellRe = /<t[hd][^>]*>([\s\S]*?)<\/t[hd]>/gi;
    let cellM: RegExpExecArray | null;
    while ((cellM = cellRe.exec(rowHtml)) !== null) {
      const inner = (cellM[1] ?? "")
        .replace(/<br\s*\/?>/gi, "\n")
        .replace(/<[^>]+>/g, "");
      cells.push(decodeEntities(inner));
    }
    values.push(cells);
  }
  return values.length > 0 ? values : null;
}

function parseTsv(tsv: string): string[][] {
  const values: string[][] = [];
  let row: string[] = [];
  let field = "";
  let inQuotes = false;
  let started = false;
  const pushField = () => {
    row.push(field);
    field = "";
  };
  const pushRow = () => {
    pushField();
    values.push(row);
    row = [];
  };
  for (let i = 0; i < tsv.length; i++) {
    const ch = tsv[i];
    started = true;
    if (inQuotes) {
      if (ch === '"') {
        if (tsv[i + 1] === '"') {
          field += '"';
          i++;
        } else {
          inQuotes = false;
        }
      } else {
        field += ch;
      }
      continue;
    }
    if (ch === '"' && field === "") {
      inQuotes = true;
    } else if (ch === "\t") {
      pushField();
    } else if (ch === "\n") {
      pushRow();
    } else if (ch === "\r") {
      if (tsv[i + 1] === "\n") i++;
      pushRow();
    } else {
      field += ch;
    }
  }
  if (started) pushRow();
  return values;
}

export function parseClipboard(input: { html?: string; tsv?: string }): ParsedClipboard {
  if (input.html) {
    const payload = extractPayload(input.html);
    if (payload) {
      return {
        values: payload.values,
        formulas: payload.formulas,
        source: "internal",
        sourceSheet: payload.sheet,
        sourceRange: payload.range,
      };
    }
    const table = parseHtmlTable(input.html);
    if (table) return { values: table, source: "external" };
  }
  return { values: parseTsv(input.tsv ?? ""), source: "external" };
}
