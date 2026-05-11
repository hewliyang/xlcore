import { expect, test } from "vitest";
import { applyTint } from "./render";

// Reference values cross-checked against Excel's color picker.
// Accent1 = #4472C4 (Office 2016 default).
//
// Excel reports the following hexes when you tint #4472C4:
//   tint  +0.80  →  #D9E2F3   (lighter 80% / "Accent1, Lighter 80%")
//   tint  +0.60  →  #B4C7E7
//   tint  +0.40  →  #8FAADC
//   tint  -0.25  →  #2F5497   ("Accent1, Darker 25%")
//   tint  -0.50  →  #203864   ("Accent1, Darker 50%")
//
// We accept ±2 per channel for floating-point + integer-rounding wobble
// (Excel itself rounds via 240-step HSL "HLSMAX" while we work in [0,1]).

function close(a: string, b: string, tol = 2): boolean {
  const ar = parseInt(a.slice(1, 3), 16),
    ag = parseInt(a.slice(3, 5), 16),
    ab = parseInt(a.slice(5, 7), 16);
  const br = parseInt(b.slice(1, 3), 16),
    bg = parseInt(b.slice(3, 5), 16),
    bb = parseInt(b.slice(5, 7), 16);
  return Math.abs(ar - br) <= tol && Math.abs(ag - bg) <= tol && Math.abs(ab - bb) <= tol;
}

test("HLS tint preserves hue when lightening accent1 by +0.8", () => {
  const got = applyTint("#4472C4", 0.8);
  expect(close(got, "#d9e2f3")).toBe(true);
});

test("HLS tint +0.6", () => {
  const got = applyTint("#4472C4", 0.6);
  expect(close(got, "#b4c7e7")).toBe(true);
});

test("HLS tint +0.4", () => {
  const got = applyTint("#4472C4", 0.4);
  expect(close(got, "#8faadc")).toBe(true);
});

test("HLS tint -0.25 (darken)", () => {
  const got = applyTint("#4472C4", -0.25);
  expect(close(got, "#2f5497", 2)).toBe(true);
});

test("HLS tint -0.5 (darken more)", () => {
  const got = applyTint("#4472C4", -0.5);
  expect(close(got, "#203864", 2)).toBe(true);
});

test("tint of pure gray stays gray (no hue shift)", () => {
  const got = applyTint("#808080", 0.5);
  // pure gray: S=0, hue is undefined; result must still be gray
  expect(got.slice(1, 3)).toBe(got.slice(3, 5));
  expect(got.slice(3, 5)).toBe(got.slice(5, 7));
});

test("tint of pure red stays on the red hue line", () => {
  const got = applyTint("#FF0000", 0.5);
  // lightening red toward white in HSL: stays a pink, not a desaturated gray.
  // Per-channel linear would give #FF7F7F. HSL lighten-to-L=0.75 gives #FF8080.
  expect(close(got, "#ff8080", 2)).toBe(true);
});

test("zero tint is identity", () => {
  expect(applyTint("#4472C4", 0)).toBe("#4472c4");
});
