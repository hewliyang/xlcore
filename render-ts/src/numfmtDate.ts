import type { Section, Tok } from "./numfmt.js";

const MONTHS_LONG = [
  "January",
  "February",
  "March",
  "April",
  "May",
  "June",
  "July",
  "August",
  "September",
  "October",
  "November",
  "December",
];
const MONTHS_SHORT = [
  "Jan",
  "Feb",
  "Mar",
  "Apr",
  "May",
  "Jun",
  "Jul",
  "Aug",
  "Sep",
  "Oct",
  "Nov",
  "Dec",
];
const DAYS_LONG = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
const DAYS_SHORT = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

interface DateParts {
  y: number;
  mo: number;
  d: number;
  h: number;
  mi: number;
  s: number;
  ms: number;
  weekday: number; // 0=Sun
  totalHours: number; // for [h]
  totalMinutes: number;
  totalSeconds: number;
  isPM: boolean;
}

function serialToParts(serial: number): DateParts {
  // Excel epoch: 1899-12-30 (handles 1900 bogus-leap-year for serial >= 61).
  // For serial 0..59 the resulting date is off by one day from Excel; we
  // accept the discrepancy (real workbooks rarely store dates that early).
  const totalSeconds = serial * 86400;
  const totalMs = Math.round(serial * 86400 * 1000);
  const date = new Date(Date.UTC(1899, 11, 30) + totalMs);
  const y = date.getUTCFullYear();
  const mo = date.getUTCMonth() + 1;
  const d = date.getUTCDate();
  const h = date.getUTCHours();
  const mi = date.getUTCMinutes();
  const s = date.getUTCSeconds();
  const ms = date.getUTCMilliseconds();
  const weekday = date.getUTCDay();
  return {
    y,
    mo,
    d,
    h,
    mi,
    s,
    ms,
    weekday,
    totalHours: Math.floor(totalSeconds / 3600),
    totalMinutes: Math.floor(totalSeconds / 60),
    totalSeconds: Math.floor(totalSeconds),
    isPM: h >= 12,
  };
}

function pad2(n: number): string {
  return n.toString().padStart(2, "0");
}

export function renderDate(value: number, sec: Section): string {
  const p = serialToParts(value);
  const has12h = sec.tokens.some((t) => t.kind === "ampm");
  let s = "";
  // For context-sensitive `m` vs minutes: a `m`/`mm` token means MINUTES
  // when it follows an h/hh/[h] token (skipping non-date literals) OR is
  // immediately followed by an s/ss token. Otherwise it means month.
  for (let i = 0; i < sec.tokens.length; i++) {
    const t = sec.tokens[i]!;
    if (t.kind === "lit") {
      s += t.s;
      continue;
    }
    if (t.kind === "ampm") {
      const pm = p.isPM;
      if (t.abbreviated) s += pm ? (t.upper ? "P" : "p") : t.upper ? "A" : "a";
      else s += pm ? (t.upper ? "PM" : "pm") : t.upper ? "AM" : "am";
      continue;
    }
    if (t.kind === "elapsed") {
      const v = t.field === "h" ? p.totalHours : t.field === "m" ? p.totalMinutes : p.totalSeconds;
      s += v.toString().padStart(t.width, "0");
      continue;
    }
    if (t.kind === "date") {
      switch (t.field) {
        case "yyyy":
        case "yyy":
          s += p.y.toString().padStart(4, "0");
          break;
        case "yy":
        case "y":
          s += pad2(p.y % 100);
          break;
        case "mmmmm":
          s += (MONTHS_LONG[p.mo - 1] ?? "")[0] ?? "";
          break;
        case "mmmm":
          s += MONTHS_LONG[p.mo - 1] ?? "";
          break;
        case "mmm":
          s += MONTHS_SHORT[p.mo - 1] ?? "";
          break;
        case "mm":
        case "m": {
          // context: minutes if surrounded by h/s tokens
          const isMinutes = isMinuteContext(sec.tokens, i);
          if (isMinutes) s += t.field === "mm" ? pad2(p.mi) : p.mi.toString();
          else s += t.field === "mm" ? pad2(p.mo) : p.mo.toString();
          break;
        }
        case "dddd":
          s += DAYS_LONG[p.weekday] ?? "";
          break;
        case "ddd":
          s += DAYS_SHORT[p.weekday] ?? "";
          break;
        case "dd":
          s += pad2(p.d);
          break;
        case "d":
          s += p.d.toString();
          break;
        case "hh":
        case "h": {
          let hr = p.h;
          if (has12h) {
            hr = hr % 12;
            if (hr === 0) hr = 12;
          }
          s += t.field === "hh" ? pad2(hr) : hr.toString();
          break;
        }
        case "ss":
        case "s": {
          // Sub-second precision via trailing ".0" / ".00" — handled by
          // the literal+digit pair "ss.0" tokenizing as ["ss", ".", "0"].
          // We only emit the integer seconds here; the dot/digits in the
          // surrounding tokens come from the adjacent number-flavor
          // placeholders, which we render in the literal pass below.
          // For now: just integer seconds, plus optional ".x" lookahead.
          let sec1 = t.field === "ss" ? pad2(p.s) : p.s.toString();
          // Look ahead for `.` + digits → sub-second.
          if (sec.tokens[i + 1]?.kind === "dot") {
            const digitToks: Tok[] = [];
            let j = i + 2;
            while (j < sec.tokens.length && sec.tokens[j]!.kind === "digit") {
              digitToks.push(sec.tokens[j]!);
              j++;
            }
            if (digitToks.length > 0) {
              const f = (p.ms / 1000).toFixed(digitToks.length).slice(2);
              sec1 += "." + f;
              i = j - 1; // consume dot + digits
            }
          }
          s += sec1;
          break;
        }
      }
      continue;
    }
    // digit / dot / comma / percent / slash / exp / text shouldn't appear
    // inside a date section at the top level — ignore defensively.
  }
  return s;
}

function isMinuteContext(tokens: Tok[], idx: number): boolean {
  // Scan backwards for the nearest date/elapsed token; if it's h/hh/[h], minutes.
  for (let i = idx - 1; i >= 0; i--) {
    const t = tokens[i]!;
    if (t.kind === "date" && /^h{1,2}$/.test(t.field)) return true;
    if (t.kind === "elapsed" && t.field === "h") return true;
    if (t.kind === "date" || t.kind === "elapsed") break;
  }
  // Scan forward for s/ss
  for (let i = idx + 1; i < tokens.length; i++) {
    const t = tokens[i]!;
    if (t.kind === "date" && /^s{1,2}$/.test(t.field)) return true;
    if (t.kind === "elapsed" && t.field === "s") return true;
    if (t.kind === "date" || t.kind === "elapsed") break;
  }
  return false;
}
