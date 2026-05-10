// Excel/OOXML number-format evaluator.
//
// Replaces the stub regex-based formatter that landed in v0. Targets the
// four fixtures in `tests/fixtures/numfmt/`:
//   - date / time (built-in 14..22 + custom y/m/d/h/s tokens, [h], am/pm)
//   - currency / accounting ($, [$€-407], _("$"* …), padding tokens)
//   - multi-section pos;neg;zero;text + [color][cond] gates
//   - fractions # ?/?, # ??/??, # ?/8, and scientific 0.00E+00, ##0.0E+0
//
// Spec references:
//   - ECMA-376 part 1, §18.8.31 ("numFmt") — token grammar
//   - LibreOffice sc/source/core/tool/zformat.cxx — battle-tested runtime
//
// Out of scope (tracked in PARITY.md):
//   - Locale-aware separator selection (we always use "." and ",")
//   - Real font-metric padding for `_x` and `*x` (we emit a single space)
//   - Asian / lunar calendar tokens (`g`, `e`, `b1`, `b2`, etc.)

import { renderDate } from "./numfmtDate.js";
import { renderFraction } from "./numfmtFraction.js";
import { renderScientific } from "./numfmtScientific.js";

// ---------- public API ----------

export interface FormatResult {
  text: string;
  /** CSS color from `[Red]` / `[Color12]` etc., if the matched section
   *  carried one. The renderer uses it as a font-color override. */
  color?: string;
}

const FORMAT_CACHE = new Map<string, Section[]>();

/** Format a numeric value through an OOXML format code.
 *
 *  `fmt` is the raw format code as it appears in `<numFmt formatCode="…"/>`
 *  or one of the built-in IDs (resolved by the caller before this hits).
 *
 *  Returns `{ text }` on the happy path; falls back to `formatGeneral` if
 *  the format is missing, "General", or the parser bails. Never throws. */
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
  const sec = pickSection(sections, value);
  if (!sec) return { text: formatGeneral(value) };
  try {
    return { text: renderSection(value, sec), color: sec.color };
  } catch {
    return { text: formatGeneral(value) };
  }
}

/** "General" rendering — used when no format is set or as a fallback. */
export function formatGeneral(v: number): string {
  if (!isFinite(v)) return String(v);
  if (Number.isInteger(v) && Math.abs(v) < 1e15) return v.toString();
  return parseFloat(v.toPrecision(11)).toString();
}

// ---------- token model ----------

// `,` and `/` stay as literals (`lit`) — they are context-sensitive
// (grouping vs date separator vs fraction divider) and we resolve them
// during section classification, not tokenization.
export type Tok =
  | { kind: "lit"; s: string }
  | { kind: "digit"; ch: "0" | "#" | "?" }
  | { kind: "dot" }
  | { kind: "percent" }
  | { kind: "exp"; sign: "+" | "-" | ""; upper: boolean }
  | { kind: "date"; field: string } // "y","yy","yyyy","m","mm","mmm","mmmm","mmmmm","d","dd","ddd","dddd","h","hh","s","ss"
  | { kind: "elapsed"; field: "h" | "m" | "s"; width: number } // [h], [hh], [mm]:..., etc.
  | { kind: "ampm"; upper: boolean; abbreviated: boolean }
  | { kind: "text" }; // @

export interface Section {
  tokens: Tok[];
  color?: string;
  condition?: { op: ">" | "<" | ">=" | "<=" | "=" | "<>"; value: number };
  /** kind of section, picked from the token mix */
  flavor: "number" | "date" | "fraction" | "scientific" | "text" | "literal";
  // number-flavor pre-computed bits
  intPlaces: number; // digit placeholders before "."
  fracPlaces: number; // digit placeholders after "."
  hasGrouping: boolean;
  scale: number; // 1, 100 (percent), 1e-3 (per trailing comma)
  // fraction-flavor
  fractionDenom: number; // 0 = variable, else fixed (e.g. 8 for "?/8")
  fractionDenomQs: number; // count of "?" in denominator placeholder run
  fractionIntPlaces: number; // placeholders in integer part before " "
  fractionHideZeroInt: boolean; // true if every int placeholder is `#` — hide leading 0
  // scientific-flavor
  expSign: "+" | "-" | "";
  expDigits: number; // digit placeholders after E
  expUpper: boolean;
}

// ---------- parsing ----------

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
      // consume quoted literal
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

// Excel `[Color1]` .. `[Color56]` map onto the legacy indexed palette
// (ECMA-376 §18.8.27 default `indexedColors`), 1-based. Indices here
// are shifted by +1 vs render.ts's 0-based INDEXED_PALETTE: Excel's
// `[Color1]` is the first user-visible palette slot (black), which is
// stored as indexedColors[0] in styles.xml.
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
  // Strip leading [color]/[cond]/[$cur-loc] tags off the front; embed
  // currency/literal tags as `lit` tokens; everything else flows through
  // a per-character tokenizer.
  let s = raw;
  let color: string | undefined;
  let condition: Section["condition"];

  // Eat leading [..] runs that are color or condition. Currency ([$..])
  // can appear anywhere, so we handle it inside the main loop.
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
    // Not a leading meta-tag (e.g. `[$€-407]`, `[h]`, `[hh]`, `[mm]`).
    break;
  }

  const tokens = tokenize(s);

  // Classify and pre-compute counts.
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

  // Helpers to find structural indices.
  const dotIdx = tokens.findIndex((t) => t.kind === "dot");
  // Fraction marker: a `/` literal flanked by digit placeholders or by
  // run-of-digits literals (for fixed denoms like `?/8`).
  const slashIdx = findFractionSlash(tokens);
  const expIdx = tokens.findIndex((t) => t.kind === "exp");
  const hasDate = tokens.some(
    (t) => t.kind === "date" || t.kind === "elapsed" || t.kind === "ampm",
  );
  const hasText = tokens.some((t) => t.kind === "text");
  const hasDigit = tokens.some((t) => t.kind === "digit");

  if (hasDate) flavor = "date";
  else if (slashIdx >= 0 && hasDigit) flavor = "fraction";
  else if (expIdx >= 0 && hasDigit) flavor = "scientific";
  else if (hasDigit) flavor = "number";
  else if (hasText) flavor = "text";
  else flavor = "literal";

  if (flavor === "number") {
    // intPlaces = digits before dot; fracPlaces = digits after.
    const before = dotIdx < 0 ? tokens : tokens.slice(0, dotIdx);
    const after = dotIdx < 0 ? [] : tokens.slice(dotIdx + 1);
    intPlaces = before.filter((t) => t.kind === "digit").length;
    fracPlaces = after.filter((t) => t.kind === "digit").length;
    // Grouping: a `,` literal sits between two digit placeholders.
    hasGrouping = hasGroupingComma(before);
    // Trailing commas (in literal tokens) after the last digit-placeholder
    // each scale the value by 1/1000. We also strip those `,` chars from
    // the literals so they don't render as text. Stops at the first non-`,`
    // character within a lit (any text after, like `K`, stays as-is).
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
            // Mutate the token — we own it, parseFormat just made it.
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
    // Layout: [int-digits] [space] num-digits "/" denom-digits-or-fixed.
    // The slash is a `lit` token whose `.s` contains "/". If that lit
    // has trailing chars (fixed denom like `?/8` → lit "/8"), peel them
    // off into a synthetic lit for the after-slash side.
    const slashTok = tokens[slashIdx] as Extract<Tok, { kind: "lit" }>;
    const slashPos = slashTok.s.indexOf("/");
    const beforeSlashStr = slashTok.s.slice(0, slashPos);
    const afterSlashStr = slashTok.s.slice(slashPos + 1);
    const before: Tok[] = tokens.slice(0, slashIdx);
    if (beforeSlashStr) before.push({ kind: "lit", s: beforeSlashStr });
    const after: Tok[] = [];
    if (afterSlashStr) after.push({ kind: "lit", s: afterSlashStr });
    after.push(...tokens.slice(slashIdx + 1));
    // Integer placeholders sit before the last whitespace-bearing lit in
    // `before`; if no whitespace, there's no integer part.
    let lastSpaceIdx = -1;
    for (let i = 0; i < before.length; i++) {
      const t = before[i]!;
      if (t.kind === "lit" && /\s/.test(t.s)) lastSpaceIdx = i;
    }
    if (lastSpaceIdx >= 0) {
      fractionIntPlaces = before.slice(0, lastSpaceIdx).filter((t) => t.kind === "digit").length;
      // Track whether every int placeholder is `#` (hide zeros) vs `0`/`?`.
      const intPHs = before.slice(0, lastSpaceIdx).filter((t) => t.kind === "digit") as Extract<
        Tok,
        { kind: "digit" }
      >[];
      fractionHideZeroInt = intPHs.length > 0 && intPHs.every((t) => t.ch === "#");
    }
    // Denominator: either a fixed number (digit chars in a lit) or `?` count.
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
    // intPlaces / fracPlaces come from the mantissa portion.
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

/** True if any `lit` between two digit placeholders contains `,`.
 *  Trailing commas (after the last digit) are scale markers, not grouping. */
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

/** Find the index of a `lit` token whose `.s` contains `/` and which is
 *  flanked (skipping other lits) by a digit placeholder on the left and a
 *  digit placeholder OR a digit-bearing lit on the right. -1 if none. */
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
    // Right side: digits can live (a) inside this same lit (chars after
    // the `/`, e.g. `/8`), (b) in a later digit placeholder, or (c) in a
    // later digit-bearing lit.
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
    // Quoted literal
    if (c === '"') {
      let lit = "";
      i++;
      while (i < s.length && s[i] !== '"') {
        lit += s[i];
        i++;
      }
      if (i < s.length) i++; // closing quote
      if (lit) out.push({ kind: "lit", s: lit });
      continue;
    }
    // Backslash escape: next char is literal
    if (c === "\\") {
      if (i + 1 < s.length) {
        out.push({ kind: "lit", s: s[i + 1]! });
        i += 2;
      } else i++;
      continue;
    }
    // _x — render width of x (we emit a space; close enough for v0)
    if (c === "_") {
      i += i + 1 < s.length ? 2 : 1;
      out.push({ kind: "lit", s: " " });
      continue;
    }
    // *x — fill char (we emit nothing; we don't know the cell width here)
    if (c === "*") {
      i += i + 1 < s.length ? 2 : 1;
      continue;
    }
    // Bracketed run: currency, elapsed-time, or stray (unhandled) tag
    if (c === "[") {
      let inner = "";
      i++;
      while (i < s.length && s[i] !== "]") {
        inner += s[i];
        i++;
      }
      if (i < s.length) i++; // closing
      // [$sym-locale]  →  literal sym
      if (inner.startsWith("$")) {
        const sym = inner.slice(1).split("-")[0]!;
        if (sym) out.push({ kind: "lit", s: sym });
        continue;
      }
      // [h], [hh], [m], [mm], [s], [ss] → elapsed time
      const em = /^([hms])\1*$/i.exec(inner);
      if (em) {
        const field = inner[0]!.toLowerCase() as "h" | "m" | "s";
        out.push({ kind: "elapsed", field, width: inner.length });
        continue;
      }
      // Otherwise: ignore (unhandled — locale tags etc.)
      continue;
    }
    // Date / time runs: yyyy/yy/y, mmmm/mmm/mm/m, dddd/ddd/dd/d, hh/h, ss/s
    const dm = /^(yyyy|yyy|yy|y|mmmmm|mmmm|mmm|mm|m|dddd|ddd|dd|d|hh|h|ss|s)/i.exec(s.slice(i));
    if (dm) {
      const tok = dm[0]!.toLowerCase();
      out.push({ kind: "date", field: tok });
      i += tok.length;
      continue;
    }
    // AM/PM  (also A/P)
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
    // `,` and `/` are literals; semantics resolved per-section.
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
    // Anything else is a literal char.
    out.push({ kind: "lit", s: c });
    i++;
  }
  // Coalesce adjacent literals — makes downstream walkers tidier.
  const merged: Tok[] = [];
  for (const t of out) {
    const prev = merged[merged.length - 1];
    if (t.kind === "lit" && prev && prev.kind === "lit") prev.s += t.s;
    else merged.push(t);
  }
  return merged;
}

// ---------- section selection ----------

function pickSection(sections: Section[], value: number): Section | undefined {
  if (sections.length === 0) return undefined;
  // If any section has an explicit [cond], interpret per OOXML:
  // section[0]'s cond, section[1]'s cond, section[2] = "everything else".
  const hasExplicitConds = sections.some((s) => s.condition);
  if (hasExplicitConds) {
    for (let i = 0; i < Math.min(2, sections.length); i++) {
      const s = sections[i]!;
      if (!s.condition) continue;
      if (matchesCond(value, s.condition)) return s;
    }
    return sections[2] ?? sections[sections.length - 1];
  }
  // Sign-based: pos / neg / zero / text (we don't get text values here).
  if (sections.length === 1) return sections[0];
  if (value > 0) return sections[0];
  if (value < 0) return sections[1] ?? sections[0];
  // value == 0
  return sections[2] ?? sections[0];
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

// ---------- rendering ----------

function renderSection(value: number, sec: Section): string {
  switch (sec.flavor) {
    case "literal":
      return sec.tokens.map((t) => (t.kind === "lit" ? t.s : "")).join("");
    case "text":
      // No string value to substitute on the numeric path; emit the literal scaffolding.
      return sec.tokens.map((t) => (t.kind === "lit" ? t.s : "")).join("");
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

// ----- number -----

function renderNumber(value: number, sec: Section): string {
  // Whether the section itself represents the absolute value (negative
  // section already encodes the sign via parens or a leading "-").
  // OOXML rule: when only ONE section is given, negatives auto-prefix "-".
  // When the section is index 1 (the "neg" slot), the value is rendered
  // as |value| because the section contains the formatting for negatives.
  const sign = value < 0 ? "-" : "";
  const v = value * sec.scale;
  // Round to fracPlaces.
  const absStr = Math.abs(v).toFixed(sec.fracPlaces);
  const dotPos = absStr.indexOf(".");
  const intDigits = dotPos < 0 ? absStr : absStr.slice(0, dotPos);
  const fracDigits = dotPos < 0 ? "" : absStr.slice(dotPos + 1);

  // Walk tokens: integer side right-to-left, fractional side left-to-right.
  const dotIdx = sec.tokens.findIndex((t) => t.kind === "dot");
  const beforeDot = dotIdx < 0 ? sec.tokens : sec.tokens.slice(0, dotIdx);
  const afterDot = dotIdx < 0 ? [] : sec.tokens.slice(dotIdx + 1);

  const intRendered = renderIntegerTokens(beforeDot, intDigits, sec.hasGrouping);
  const fracRendered = renderFractionalTokens(afterDot, fracDigits);
  void sec.intPlaces; // currently unused; kept for future shrink-to-fit

  // Determine if section already encodes sign (parens or leading "-").
  const sectionEncodesNeg = sec.tokens.some(
    (t) => t.kind === "lit" && (t.s.includes("(") || t.s.includes("-")),
  );
  // A section with a leading "-" in its literals (or wrapped in parens)
  // is "the negative formatter" — emit |value| through it.
  // We approximate: if the parser is rendering this section as part of a
  // multi-section format (caller picked it because value < 0), the sign is
  // already encoded by literals. We can't tell here whether this is the
  // neg-slot or the only-slot; so we suppress our auto-sign whenever the
  // section contains literals that look sign-bearing.
  const finalSign = sectionEncodesNeg ? "" : sign;

  let out = "";
  if (dotIdx < 0) out = intRendered;
  else out = intRendered + "." + fracRendered;
  return finalSign + out;
}

import { renderFractionalTokens, renderIntegerTokens } from "./numfmtNumberParts.js";
