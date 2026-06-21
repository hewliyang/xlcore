import { findCell } from "./columnar.js";
import type { Sheet } from "./types.js";

export function formatFormulaBar(sheet: Sheet, active: { r: number; c: number }): string {
  const cell = findCell(sheet, active.r, active.c);
  if (!cell) return "";
  if (cell.formula) return cell.formula.startsWith("=") ? cell.formula : `=${cell.formula}`;
  if (cell.value !== undefined) return String(cell.value);
  if (cell.runs && cell.runs.length > 0) return cell.runs.map((run) => run.text).join("");
  return "";
}

export function balanceFormula(text: string): string {
  if (!text.startsWith("=")) return text;
  let depth = 0;
  let inString = false;
  let inQuote = false;
  for (let i = 0; i < text.length; i++) {
    const ch = text[i];
    if (inString) {
      if (ch === '"') {
        if (text[i + 1] === '"') i++;
        else inString = false;
      }
      continue;
    }
    if (inQuote) {
      if (ch === "'") {
        if (text[i + 1] === "'") i++;
        else inQuote = false;
      }
      continue;
    }
    if (ch === '"') inString = true;
    else if (ch === "'") inQuote = true;
    else if (ch === "(") depth++;
    else if (ch === ")" && depth > 0) depth--;
  }
  return depth > 0 ? text + ")".repeat(depth) : text;
}
