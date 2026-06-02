import type { Workbook } from "./api.js";

export function parseRef(ref: string): { sheet?: string; a1: string } {
  const bang = ref.lastIndexOf("!");
  if (bang < 0) return { a1: ref };
  let sheet = ref.slice(0, bang);
  if (sheet.startsWith("'") && sheet.endsWith("'")) {
    sheet = sheet.slice(1, -1).replace(/''/g, "'");
  }
  return { sheet, a1: ref.slice(bang + 1) };
}

export function headerRange(a1: string): string {
  const [start, end] = a1.split(":");
  if (!start || !end) return a1;
  const col = (s: string) => s.replace(/\d+/g, "");
  const rowNum = (s: string) => Number.parseInt(s.replace(/\D+/g, ""), 10) || 1;
  return `${col(start)}${rowNum(start)}:${col(end)}${rowNum(start)}`;
}

export function distinctValuesFor(workbook: Workbook, sourceRef: string, field: string): string[] {
  const { sheet: srcSheet, a1 } = parseRef(sourceRef);
  let headers: string[];
  try {
    const ws = srcSheet ? workbook.sheet(srcSheet) : workbook.activeSheet();
    const row = ws.range(headerRange(a1)).values()[0] ?? [];
    headers = row.map((c, i) => (c.type === "blank" ? `Column ${i + 1}` : String(c.value)));
  } catch {
    return [];
  }
  const idx = headers.indexOf(field);
  if (idx < 0) return [];
  try {
    const ws = srcSheet ? workbook.sheet(srcSheet) : workbook.activeSheet();
    const rows = ws.range(a1).values();
    const seen = new Set<string>();
    const out: string[] = [];
    for (let r = 1; r < rows.length; r++) {
      const c = rows[r]?.[idx];
      if (!c || c.type === "blank") continue;
      const label = c.type === "boolean" ? (c.value ? "TRUE" : "FALSE") : String(c.value);
      if (!seen.has(label)) {
        seen.add(label);
        out.push(label);
      }
    }
    return out;
  } catch {
    return [];
  }
}
