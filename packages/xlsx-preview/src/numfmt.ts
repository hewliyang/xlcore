import { renderDate } from "./numfmtDate.js";
import { renderFraction } from "./numfmtFraction.js";
import { renderScientific } from "./numfmtScientific.js";

export interface FormatResult {
  text: string;

  color?: string;

  fills?: string[];
}

export const FILL_SENTINEL = "\u0001";

const FORMAT_CACHE = new Map<string, Section[]>();

export function formatValue(value: number, fmt: string | undefined): FormatResult {
  const f = (fmt ?? "").trim();
  if (!f || f.toLowerCase() === "general") return { text: formatGeneral(value) };
  let sections: Section[];
  try {
    sections = FORMAT_CACHE.get(f) ?? parseFormat(f);
    FORMAT_CACHE.set(f, sections);
  } catch {
    return { text: formatGeneral(value) };
  }
  const picked = pickSection(sections, value);
  if (!picked) return { text: formatGeneral(value) };
  const { sec, isNegSlot } = picked;
  try {
    const renderValue = isNegSlot ? Math.abs(value) : value;
    const text = renderSection(renderValue, sec);
    const fills = sec.tokens.flatMap((t) => (t.kind === "fill" ? [t.ch] : []));
    const out: FormatResult = { text, color: sec.color };
    if (fills.length > 0) out.fills = fills;
    return out;
  } catch {
    return { text: formatGeneral(value) };
  }
}

export function formatGeneral(v: number): string {
  if (!isFinite(v)) return String(v);
  if (Number.isInteger(v) && Math.abs(v) < 1e15) return v.toString();
  return parseFloat(v.toPrecision(11)).toString();
}

export type Tok =
  | { kind: "lit"; s: string }
  | { kind: "digit"; ch: "0" | "#" | "?" }
  | { kind: "dot" }
  | { kind: "percent" }
  | { kind: "exp"; sign: "+" | "-" | ""; upper: boolean }
  | { kind: "date"; field: string }
  | { kind: "elapsed"; field: "h" | "m" | "s"; width: number }
  | { kind: "ampm"; upper: boolean; abbreviated: boolean }
  | { kind: "fill"; ch: string }
  | { kind: "general" }
  | { kind: "text" };

export interface Section {
  tokens: Tok[];
  color?: string;
  condition?: { op: ">" | "<" | ">=" | "<=" | "=" | "<>"; value: number };

  flavor: "number" | "date" | "fraction" | "scientific" | "text" | "literal";

  intPlaces: number;
  fracPlaces: number;
  hasGrouping: boolean;
  scale: number;

  fractionDenom: number;
  fractionDenomQs: number;
  fractionIntPlaces: number;
  fractionHideZeroInt: boolean;

  expSign: "+" | "-" | "";
  expDigits: number;
  expUpper: boolean;
}

function parseFormat(fmt: string): Section[] {
  const rawSections = splitTopLevel(fmt, ";");
  return rawSections.map(parseSection);
}

function splitTopLevel(s: string, sep: string): string[] {
  const out: string[] = [];
  let cur = "";
  let i = 0;
  while (i < s.length) {
    const c = s[i]!;
    if (c === '"') {
      cur += c;
      i++;
      while (i < s.length && s[i] !== '"') {
        cur += s[i];
        i++;
      }
      if (i < s.length) {
        cur += s[i];
        i++;
      }
      continue;
    }
    if (c === "[") {
      cur += c;
      i++;
      while (i < s.length && s[i] !== "]") {
        cur += s[i];
        i++;
      }
      if (i < s.length) {
        cur += s[i];
        i++;
      }
      continue;
    }
    if (c === "\\") {
      cur += c;
      if (i + 1 < s.length) {
        cur += s[i + 1];
        i += 2;
      } else i++;
      continue;
    }
    if (c === sep) {
      out.push(cur);
      cur = "";
      i++;
      continue;
    }
    cur += c;
    i++;
  }
  out.push(cur);
  return out;
}

const COLOR_NAMES: Record<string, string> = {
  black: "#000000",
  white: "#ffffff",
  red: "#ff0000",
  green: "#008000",
  blue: "#0000ff",
  yellow: "#ffff00",
  magenta: "#ff00ff",
  cyan: "#00ffff",
};

const COLOR_BY_INDEX: Record<number, string> = {
  1: "#000000",
  2: "#ffffff",
  3: "#ff0000",
  4: "#00ff00",
  5: "#0000ff",
  6: "#ffff00",
  7: "#ff00ff",
  8: "#00ffff",
  9: "#800000",
  10: "#008000",
  11: "#000080",
  12: "#808000",
  13: "#800080",
  14: "#008080",
  15: "#c0c0c0",
  16: "#808080",
  17: "#9999ff",
  18: "#993366",
  19: "#ffffcc",
  20: "#ccffff",
  21: "#660066",
  22: "#ff8080",
  23: "#0066cc",
  24: "#ccccff",
  25: "#000080",
  26: "#ff00ff",
  27: "#ffff00",
  28: "#00ffff",
  29: "#800080",
  30: "#800000",
  31: "#008080",
  32: "#0000ff",
  33: "#00ccff",
  34: "#ccffff",
  35: "#ccffcc",
  36: "#ffff99",
  37: "#99ccff",
  38: "#ff99cc",
  39: "#cc99ff",
  40: "#ffcc99",
  41: "#3366ff",
  42: "#33cccc",
  43: "#99cc00",
  44: "#ffcc00",
  45: "#ff9900",
  46: "#ff6600",
  47: "#666699",
  48: "#969696",
  49: "#003366",
  50: "#339966",
  51: "#003300",
  52: "#333300",
  53: "#993300",
  54: "#993366",
  55: "#333399",
  56: "#333333",
};

function parseSection(raw: string): Section {
  let s = raw;
  let color: string | undefined;
  let condition: Section["condition"];

  while (true) {
    const m = /^\[([^\]]+)\]/.exec(s);
    if (!m) break;
    const inner = m[1]!;
    const lower = inner.toLowerCase();
    if (COLOR_NAMES[lower] !== undefined) {
      color = COLOR_NAMES[lower];
      s = s.slice(m[0].length);
      continue;
    }
    const cm = /^color(\d{1,2})$/i.exec(inner);
    if (cm) {
      const n = parseInt(cm[1]!, 10);
      color = COLOR_BY_INDEX[n] ?? "#000000";
      s = s.slice(m[0].length);
      continue;
    }
    const cond = /^(<=|>=|<>|=|<|>)\s*(-?\d+(?:\.\d+)?)$/.exec(inner);
    if (cond) {
      condition = {
        op: cond[1] as NonNullable<Section["condition"]>["op"],
        value: parseFloat(cond[2]!),
      };
      s = s.slice(m[0].length);
      continue;
    }

    break;
  }

  const tokens = tokenize(s);

  let flavor: Section["flavor"] = "literal";
  let intPlaces = 0,
    fracPlaces = 0;
  let hasGrouping = false;
  let scale = 1;
  let fractionDenom = 0,
    fractionDenomQs = 0,
    fractionIntPlaces = 0;
  let fractionHideZeroInt = false;
  let expSign: Section["expSign"] = "";
  let expDigits = 0;
  let expUpper = true;

  const dotIdx = tokens.findIndex((t) => t.kind === "dot");

  const slashIdx = findFractionSlash(tokens);
  const expIdx = tokens.findIndex((t) => t.kind === "exp");
  const hasDate = tokens.some(
    (t) => t.kind === "date" || t.kind === "elapsed" || t.kind === "ampm",
  );
  const hasText = tokens.some((t) => t.kind === "text");
  const hasDigit = tokens.some((t) => t.kind === "digit");
  const hasGeneral = tokens.some((t) => t.kind === "general");

  if (hasDate) flavor = "date";
  else if (slashIdx >= 0 && hasDigit) flavor = "fraction";
  else if (expIdx >= 0 && hasDigit) flavor = "scientific";
  else if (hasDigit) flavor = "number";
  else if (hasGeneral) flavor = "literal";
  else if (hasText) flavor = "text";
  else flavor = "literal";

  if (flavor === "number") {
    const before = dotIdx < 0 ? tokens : tokens.slice(0, dotIdx);
    const after = dotIdx < 0 ? [] : tokens.slice(dotIdx + 1);
    intPlaces = before.filter((t) => t.kind === "digit").length;
    fracPlaces = after.filter((t) => t.kind === "digit").length;

    hasGrouping = hasGroupingComma(before);

    const lastDigitIdx = lastIndexWhere(tokens, (t) => t.kind === "digit");
    if (lastDigitIdx >= 0) {
      let commaScale = 0;
      for (let i = lastDigitIdx + 1; i < tokens.length; i++) {
        const t = tokens[i]!;
        if (t.kind === "lit") {
          let stripped = 0;
          while (stripped < t.s.length && t.s[stripped] === ",") stripped++;
          if (stripped > 0) {
            commaScale += stripped;

            t.s = t.s.slice(stripped);
          }
          if (t.s.length > 0) break;
        } else if (t.kind === "percent" || t.kind === "dot") continue;
        else break;
      }
      scale *= Math.pow(0.001, commaScale);
    }
    if (tokens.some((t) => t.kind === "percent")) scale *= 100;
  } else if (flavor === "fraction") {
    const slashTok = tokens[slashIdx] as Extract<Tok, { kind: "lit" }>;
    const slashPos = slashTok.s.indexOf("/");
    const beforeSlashStr = slashTok.s.slice(0, slashPos);
    const afterSlashStr = slashTok.s.slice(slashPos + 1);
    const before: Tok[] = tokens.slice(0, slashIdx);
    if (beforeSlashStr) before.push({ kind: "lit", s: beforeSlashStr });
    const after: Tok[] = [];
    if (afterSlashStr) after.push({ kind: "lit", s: afterSlashStr });
    after.push(...tokens.slice(slashIdx + 1));

    let lastSpaceIdx = -1;
    for (let i = 0; i < before.length; i++) {
      const t = before[i]!;
      if (t.kind === "lit" && /\s/.test(t.s)) lastSpaceIdx = i;
    }
    if (lastSpaceIdx >= 0) {
      fractionIntPlaces = before.slice(0, lastSpaceIdx).filter((t) => t.kind === "digit").length;

      const intPHs = before.slice(0, lastSpaceIdx).filter((t) => t.kind === "digit") as Extract<
        Tok,
        { kind: "digit" }
      >[];
      fractionHideZeroInt = intPHs.length > 0 && intPHs.every((t) => t.ch === "#");
    }

    let fixedNum = "";
    let qCount = 0;
    for (const t of after) {
      if (t.kind === "digit") {
        if (t.ch === "?") qCount++;
      } else if (t.kind === "lit") {
        const m = /^([0-9]+)/.exec(t.s);
        if (m) fixedNum += m[1]!;
      }
    }
    if (fixedNum) fractionDenom = parseInt(fixedNum, 10);
    fractionDenomQs = qCount;
  } else if (flavor === "scientific") {
    const expTok = tokens[expIdx] as Extract<Tok, { kind: "exp" }>;
    expSign = expTok.sign;
    expUpper = expTok.upper;

    const before =
      dotIdx < 0 || dotIdx > expIdx ? tokens.slice(0, expIdx) : tokens.slice(0, dotIdx);
    const after = dotIdx >= 0 && dotIdx < expIdx ? tokens.slice(dotIdx + 1, expIdx) : [];
    intPlaces = before.filter((t) => t.kind === "digit").length;
    fracPlaces = after.filter((t) => t.kind === "digit").length;
    expDigits = tokens.slice(expIdx + 1).filter((t) => t.kind === "digit").length;
  }

  return {
    tokens,
    color,
    condition,
    flavor,
    intPlaces,
    fracPlaces,
    hasGrouping,
    scale,
    fractionDenom,
    fractionDenomQs,
    fractionIntPlaces,
    fractionHideZeroInt,
    expSign,
    expDigits,
    expUpper,
  };
}

function lastIndexWhere<T>(arr: T[], pred: (t: T) => boolean): number {
  for (let i = arr.length - 1; i >= 0; i--) if (pred(arr[i]!)) return i;
  return -1;
}

function hasGroupingComma(toks: Tok[]): boolean {
  let firstDigit = -1,
    lastDigit = -1;
  for (let i = 0; i < toks.length; i++) {
    if (toks[i]!.kind === "digit") {
      if (firstDigit < 0) firstDigit = i;
      lastDigit = i;
    }
  }
  if (firstDigit < 0 || firstDigit === lastDigit) return false;
  for (let i = firstDigit + 1; i < lastDigit; i++) {
    const t = toks[i]!;
    if (t.kind === "lit" && t.s.includes(",")) return true;
  }
  return false;
}

function findFractionSlash(toks: Tok[]): number {
  for (let i = 0; i < toks.length; i++) {
    const t = toks[i]!;
    if (t.kind !== "lit" || !t.s.includes("/")) continue;
    let leftOk = false;
    for (let j = i - 1; j >= 0; j--) {
      const u = toks[j]!;
      if (u.kind === "digit") {
        leftOk = true;
        break;
      }
      if (u.kind === "lit") continue;
      break;
    }
    if (!leftOk) continue;

    let rightOk = false;
    const afterSlash = t.s.slice(t.s.indexOf("/") + 1);
    if (/^[0-9]/.test(afterSlash)) rightOk = true;
    if (!rightOk) {
      for (let j = i + 1; j < toks.length; j++) {
        const u = toks[j]!;
        if (u.kind === "digit") {
          rightOk = true;
          break;
        }
        if (u.kind === "lit") {
          if (/^[0-9]/.test(u.s)) {
            rightOk = true;
            break;
          }
          continue;
        }
        break;
      }
    }
    if (rightOk) return i;
  }
  return -1;
}

function tokenize(s: string): Tok[] {
  const out: Tok[] = [];
  let i = 0;
  while (i < s.length) {
    const c = s[i]!;

    if (c === '"') {
      let lit = "";
      i++;
      while (i < s.length && s[i] !== '"') {
        lit += s[i];
        i++;
      }
      if (i < s.length) i++;
      if (lit) out.push({ kind: "lit", s: lit });
      continue;
    }

    if (c === "\\") {
      if (i + 1 < s.length) {
        out.push({ kind: "lit", s: s[i + 1]! });
        i += 2;
      } else i++;
      continue;
    }

    if (c === "_") {
      i += i + 1 < s.length ? 2 : 1;
      out.push({ kind: "lit", s: " " });
      continue;
    }

    if (c === "*") {
      const ch = i + 1 < s.length ? s[i + 1]! : " ";
      i += i + 1 < s.length ? 2 : 1;
      out.push({ kind: "fill", ch });
      continue;
    }

    if (c === "[") {
      let inner = "";
      i++;
      while (i < s.length && s[i] !== "]") {
        inner += s[i];
        i++;
      }
      if (i < s.length) i++;

      if (inner.startsWith("$")) {
        const sym = inner.slice(1).split("-")[0]!;
        if (sym) out.push({ kind: "lit", s: sym });
        continue;
      }

      const em = /^([hms])\1*$/i.exec(inner);
      if (em) {
        const field = inner[0]!.toLowerCase() as "h" | "m" | "s";
        out.push({ kind: "elapsed", field, width: inner.length });
        continue;
      }

      continue;
    }

    const dm = /^(yyyy|yyy|yy|y|mmmmm|mmmm|mmm|mm|m|dddd|ddd|dd|d|hh|h|ss|s)/i.exec(s.slice(i));
    if (dm) {
      const tok = dm[0]!.toLowerCase();
      out.push({ kind: "date", field: tok });
      i += tok.length;
      continue;
    }

    if (/^am\/pm/i.test(s.slice(i))) {
      out.push({ kind: "ampm", upper: s[i] === "A", abbreviated: false });
      i += 5;
      continue;
    }
    if (/^a\/p/i.test(s.slice(i))) {
      out.push({ kind: "ampm", upper: s[i] === "A", abbreviated: true });
      i += 3;
      continue;
    }

    if ((c === "G" || c === "g") && /^general/i.test(s.slice(i))) {
      out.push({ kind: "general" });
      i += 7;
      continue;
    }
    if (c === "0" || c === "#" || c === "?") {
      out.push({ kind: "digit", ch: c });
      i++;
      continue;
    }
    if (c === ".") {
      out.push({ kind: "dot" });
      i++;
      continue;
    }
    if (c === "%") {
      out.push({ kind: "percent" });
      i++;
      continue;
    }

    if (c === "," || c === "/") {
      out.push({ kind: "lit", s: c });
      i++;
      continue;
    }
    if ((c === "E" || c === "e") && i + 1 < s.length) {
      const next = s[i + 1]!;
      if (next === "+" || next === "-") {
        out.push({ kind: "exp", sign: next as "+" | "-", upper: c === "E" });
        i += 2;
        continue;
      }
    }
    if (c === "@") {
      out.push({ kind: "text" });
      i++;
      continue;
    }

    out.push({ kind: "lit", s: c });
    i++;
  }

  const merged: Tok[] = [];
  for (const t of out) {
    const prev = merged[merged.length - 1];
    if (t.kind === "lit" && prev && prev.kind === "lit") prev.s += t.s;
    else merged.push(t);
  }
  return merged;
}

function pickSection(
  sections: Section[],
  value: number,
): { sec: Section; isNegSlot: boolean } | undefined {
  if (sections.length === 0) return undefined;

  const hasExplicitConds = sections.some((s) => s.condition);
  if (hasExplicitConds) {
    for (let i = 0; i < Math.min(2, sections.length); i++) {
      const s = sections[i]!;
      if (!s.condition) continue;
      if (matchesCond(value, s.condition)) return { sec: s, isNegSlot: false };
    }
    const fallback = sections[2] ?? sections[sections.length - 1]!;
    return { sec: fallback, isNegSlot: false };
  }

  if (sections.length === 1) return { sec: sections[0]!, isNegSlot: false };
  if (value > 0) return { sec: sections[0]!, isNegSlot: false };
  if (value < 0) {
    const neg = sections[1];
    if (neg) return { sec: neg, isNegSlot: true };
    return { sec: sections[0]!, isNegSlot: false };
  }

  return { sec: sections[2] ?? sections[0]!, isNegSlot: false };
}

function matchesCond(v: number, c: NonNullable<Section["condition"]>): boolean {
  switch (c.op) {
    case ">":
      return v > c.value;
    case "<":
      return v < c.value;
    case ">=":
      return v >= c.value;
    case "<=":
      return v <= c.value;
    case "=":
      return v === c.value;
    case "<>":
      return v !== c.value;
  }
}

function renderSection(value: number, sec: Section): string {
  const litOrFill = (t: Tok): string =>
    t.kind === "lit"
      ? t.s
      : t.kind === "fill"
        ? FILL_SENTINEL
        : t.kind === "general"
          ? formatGeneral(value)
          : "";
  switch (sec.flavor) {
    case "literal":
      return sec.tokens.map(litOrFill).join("");
    case "text":
      return sec.tokens
        .map((t) => (t.kind === "text" ? formatGeneral(value) : litOrFill(t)))
        .join("");
    case "number":
      return renderNumber(value, sec);
    case "date":
      return renderDate(value, sec);
    case "fraction":
      return renderFraction(value, sec);
    case "scientific":
      return renderScientific(value, sec);
  }
}

function renderNumber(value: number, sec: Section): string {
  const sign = value < 0 ? "-" : "";
  const v = value * sec.scale;

  const absStr = Math.abs(v).toFixed(sec.fracPlaces);
  const dotPos = absStr.indexOf(".");
  const intDigits = dotPos < 0 ? absStr : absStr.slice(0, dotPos);
  const fracDigits = dotPos < 0 ? "" : absStr.slice(dotPos + 1);

  const dotIdx = sec.tokens.findIndex((t) => t.kind === "dot");
  const beforeDot = dotIdx < 0 ? sec.tokens : sec.tokens.slice(0, dotIdx);
  const afterDot = dotIdx < 0 ? [] : sec.tokens.slice(dotIdx + 1);

  const intRendered = renderIntegerTokens(beforeDot, intDigits, sec.hasGrouping);
  const fracRendered = renderFractionalTokens(afterDot, fracDigits);
  void sec.intPlaces;

  const sectionEncodesNeg = sec.tokens.some(
    (t) => t.kind === "lit" && (t.s.includes("(") || t.s.includes("-")),
  );

  const finalSign = sectionEncodesNeg ? "" : sign;

  let out = "";
  if (dotIdx < 0) out = intRendered;
  else out = intRendered + "." + fracRendered;
  return finalSign + out;
}

import { renderFractionalTokens, renderIntegerTokens } from "./numfmtNumberParts.js";
