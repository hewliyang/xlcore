import type { Section } from "./numfmt.js";

export function renderScientific(value: number, sec: Section): string {
  if (value === 0) {
    const mantissa = (0).toFixed(sec.fracPlaces);
    const e = "0".padStart(sec.expDigits, "0");
    const sign = sec.expSign === "+" ? "+" : "";
    return mantissa + (sec.expUpper ? "E" : "e") + sign + e;
  }
  const sign = value < 0 ? "-" : "";
  const v = Math.abs(value);

  const rawExp = Math.floor(Math.log10(v));
  const exp = rawExp - (Math.max(1, sec.intPlaces) - 1);
  const mant = v / Math.pow(10, exp);

  const mantStr = mant.toFixed(sec.fracPlaces);
  const expStr = Math.abs(exp).toString().padStart(sec.expDigits, "0");
  const expSign = exp < 0 ? "-" : sec.expSign === "+" ? "+" : "";
  return sign + mantStr + (sec.expUpper ? "E" : "e") + expSign + expStr;
}
