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
  // Excel's rule (matched against hsx): pick `exp` so that the mantissa's
  // integer part is exactly `intPlaces` digits long. `0.00E+00` (intPlaces=1)
  // is classic scientific; `##0.0E+0` (intPlaces=3) shifts the mantissa into
  // [100, 1000) and the exponent isn't necessarily a multiple of 3.
  const rawExp = Math.floor(Math.log10(v));
  const exp = rawExp - (Math.max(1, sec.intPlaces) - 1);
  const mant = v / Math.pow(10, exp);
  // Determine mantissa format: intPlaces digits before "." (use as a max
  // when '#'-based; force-pad zeros for '0').
  const mantStr = mant.toFixed(sec.fracPlaces);
  const expStr = Math.abs(exp).toString().padStart(sec.expDigits, "0");
  const expSign = exp < 0 ? "-" : sec.expSign === "+" ? "+" : "";
  return sign + mantStr + (sec.expUpper ? "E" : "e") + expSign + expStr;
}
