import { FILL_SENTINEL, type Tok } from "./numfmt.js";

export function renderIntegerTokens(tokens: Tok[], intDigits: string, grouping: boolean): string {
  let digits = intDigits.replace(/^0+(?=\d)/, "");
  if (digits === "") digits = "0";

  if (digits === "0") {
    const hasZeroPlaceholder = tokens.some((t) => t.kind === "digit" && t.ch === "0");
    if (!hasZeroPlaceholder) digits = "";
  }

  if (grouping) digits = digits.replace(/\B(?=(\d{3})+(?!\d))/g, ",");

  const placeholders: number[] = [];
  for (let i = 0; i < tokens.length; i++) if (tokens[i]!.kind === "digit") placeholders.push(i);
  const firstDigit = placeholders[0] ?? -1;
  const lastDigit = placeholders[placeholders.length - 1] ?? -1;
  const isGroupingMarker = (idx: number, t: Tok): boolean => {
    if (!grouping || t.kind !== "lit" || t.s !== ",") return false;
    return idx > firstDigit && idx < lastDigit;
  };

  const out: string[] = new Array(tokens.length);
  let di = digits.length - 1;
  for (let pi = placeholders.length - 1; pi >= 0; pi--) {
    const tIdx = placeholders[pi]!;
    const t = tokens[tIdx] as Extract<Tok, { kind: "digit" }>;
    if (pi === 0) {
      const rest = di >= 0 ? digits.slice(0, di + 1) : "";
      if (rest) out[tIdx] = rest;
      else if (t.ch === "0") out[tIdx] = "0";
      else if (t.ch === "?") out[tIdx] = " ";
      else out[tIdx] = "";
      di = -1;
    } else {
      if (di >= 0) {
        let ch = digits[di]!;
        di--;
        while (ch === "," && di >= 0) {
          ch = digits[di]!;
          di--;
        }
        if (ch === ",") ch = "";
        out[tIdx] = ch;
      } else {
        out[tIdx] = t.ch === "0" ? "0" : t.ch === "?" ? " " : "";
      }
    }
  }

  let s = "";
  for (let i = 0; i < tokens.length; i++) {
    const t = tokens[i]!;
    if (t.kind === "digit") s += out[i] ?? "";
    else if (t.kind === "lit") {
      if (isGroupingMarker(i, t)) continue;
      s += t.s;
    } else if (t.kind === "percent") s += "%";
    else if (t.kind === "fill") s += FILL_SENTINEL;
  }
  return s;
}

export function renderFractionalTokens(tokens: Tok[], fracDigits: string): string {
  let s = "";
  let di = 0;
  for (const t of tokens) {
    if (t.kind === "digit") {
      if (di < fracDigits.length) {
        s += fracDigits[di]!;
        di++;
      } else if (t.ch === "0") s += "0";
      else if (t.ch === "?") s += " ";
    } else if (t.kind === "lit") {
      s += t.s;
    } else if (t.kind === "percent") {
      s += "%";
    } else if (t.kind === "fill") {
      s += FILL_SENTINEL;
    }
  }
  return s;
}
