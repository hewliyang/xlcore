import type { Section } from "./numfmt.js";

export function renderFraction(value: number, sec: Section): string {
  const sign = value < 0 ? "-" : "";
  const av = Math.abs(value);
  const intPart = sec.fractionIntPlaces > 0 ? Math.floor(av) : 0;
  const fracPart = sec.fractionIntPlaces > 0 ? av - intPart : av;

  let num = 0,
    den = 1;
  if (sec.fractionDenom > 0) {
    den = sec.fractionDenom;
    num = Math.round(fracPart * den);
    if (num === den) {
      // round-up: bump int part, zero out fraction
      if (sec.fractionIntPlaces > 0) {
        return formatFractionFinal(sign, intPart + 1, 0, den, sec);
      } else {
        return formatFractionFinal(sign, 0, den, den, sec);
      }
    }
  } else {
    // Variable denominator with fractionDenomQs digits of precision.
    const maxDen = Math.pow(10, Math.max(1, sec.fractionDenomQs)) - 1;
    [num, den] = bestFraction(fracPart, maxDen);
  }

  return formatFractionFinal(sign, intPart, num, den, sec);
}

function formatFractionFinal(
  sign: string,
  intPart: number,
  num: number,
  den: number,
  sec: Section,
): string {
  if (sec.fractionIntPlaces > 0) {
    // `#` placeholders hide a zero integer ("5/8" not "0 5/8");
    // `0` / `?` placeholders keep it.
    const hideInt = intPart === 0 && sec.fractionHideZeroInt;
    if (num === 0) return hideInt ? sign + "0" : sign + String(intPart);
    if (hideInt) return sign + String(num) + "/" + String(den);
    return sign + String(intPart) + " " + String(num) + "/" + String(den);
  }
  return sign + String(num) + "/" + String(den);
}

/** Best p/q approximation of `x` in [0,1) with q <= maxDen.
 *  Stern–Brocot walk; returns [num, den]. */
function bestFraction(x: number, maxDen: number): [number, number] {
  if (x === 0) return [0, 1];
  let lo: [number, number] = [0, 1];
  let hi: [number, number] = [1, 1];
  let best: [number, number] = [0, 1];
  let bestErr = Math.abs(x);
  for (let i = 0; i < 100; i++) {
    const mn = lo[0] + hi[0];
    const md = lo[1] + hi[1];
    if (md > maxDen) break;
    const m = mn / md;
    const err = Math.abs(x - m);
    if (err < bestErr) {
      best = [mn, md];
      bestErr = err;
    }
    if (m < x) lo = [mn, md];
    else if (m > x) hi = [mn, md];
    else return [mn, md];
  }
  return best;
}
