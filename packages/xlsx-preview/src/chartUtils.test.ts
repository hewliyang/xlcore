import { describe, expect, it } from "vitest";
import {
  axisStraddlesZero,
  computeBarSlotMetrics,
  formatAxisValue,
  isZeroTickInside,
  resolveAxisRange,
  zeroAxisMetrics,
} from "./chartUtils.js";

describe("computeBarSlotMetrics — gapWidth / overlap", () => {
  it("Excel default clustered (gapWidth=150, overlap=0)", () => {
    // 1 series, slot=100. Per spec: barW = 100 / (1 + 0 + 1.5) = 40.
    const m = computeBarSlotMetrics(100, 1, false, 150, 0);
    expect(m.barW).toBeCloseTo(40, 5);
    expect(m.firstBarLeftOffset).toBeCloseTo(30, 5); // (100 - 40) / 2
    expect(m.barShift).toBeCloseTo(40, 5); // shift = barW * (1 - 0) = barW
  });

  it("Excel default stacked (overlap forced to 100, gapWidth=150)", () => {
    // Same denom as 1-series clustered; stacked should match regardless of N.
    const m = computeBarSlotMetrics(100, 3, true, 150, undefined);
    expect(m.barW).toBeCloseTo(40, 5);
    expect(m.firstBarLeftOffset).toBeCloseTo(30, 5);
  });

  it("AGS Chart 19 clustered with gapWidth=219 + overlap=-27", () => {
    // 1 series at slot=100, gw=219, ov=-27 (irrelevant when N=1).
    // barW = 100 / (1 + 0 + 2.19) = 100 / 3.19 ≈ 31.35.
    const m = computeBarSlotMetrics(100, 1, false, 219, -27);
    expect(m.barW).toBeCloseTo(100 / 3.19, 4);
  });

  it("3 clustered series with overlap=-27 (extra space between bars)", () => {
    // shiftFactor = 1 - (-27/100) = 1.27. denom = 1 + 2*1.27 + 1.5 = 5.04.
    const m = computeBarSlotMetrics(100, 3, false, 150, -27);
    expect(m.barW).toBeCloseTo(100 / 5.04, 4);
    expect(m.barShift).toBeCloseTo((100 / 5.04) * 1.27, 4);
  });

  it("clamps gapWidth to spec range 0..500", () => {
    const wide = computeBarSlotMetrics(100, 1, false, 1000, 0);
    expect(wide.barW).toBeCloseTo(100 / (1 + 5), 5);
    const negative = computeBarSlotMetrics(100, 1, false, -50, 0);
    expect(negative.barW).toBeCloseTo(100, 5); // gw=0 → all slot is bar
  });

  it("clamps overlap to spec range -100..100 (clustered only)", () => {
    // overlap=200 clamped to 100 → fully overlapped → behaves like stacked.
    const m = computeBarSlotMetrics(100, 3, false, 150, 200);
    expect(m.barW).toBeCloseTo(40, 5);
    expect(m.barShift).toBeCloseTo(0, 5);
  });

  it("missing values fall back to spec defaults", () => {
    // gw=150 default, ov=0 default for clustered.
    const m = computeBarSlotMetrics(100, 1, false, undefined, undefined);
    expect(m.barW).toBeCloseTo(40, 5);
  });

  it("first bar offset centers the bar group inside the slot", () => {
    const m = computeBarSlotMetrics(100, 4, false, 100, 0);
    // span = 4 bars side-by-side = 4 * barW; barW = 100 / (1+3+1) = 20.
    // total span = 80. offset = (100-80)/2 = 10.
    expect(m.barW).toBeCloseTo(20, 5);
    expect(m.firstBarLeftOffset).toBeCloseTo(10, 5);
  });
});

describe("formatAxisValue — dispUnits divisor", () => {
  it("divides by 1000 for thousands and keeps the format code", () => {
    // ECMA-376 §21.2.2.46: tick label = value / dispUnits, then formatted.
    // 75000 / 1000 = 75 → "75" under General; "75" under "0".
    expect(formatAxisValue(75000, "General", 1000)).toBe("75");
    expect(formatAxisValue(75000, "#,##0", 1000)).toBe("75");
    expect(formatAxisValue(45000, "#,##0", 1000)).toBe("45");
  });

  it("is a no-op when divisor is undefined / null / 0 / NaN", () => {
    expect(formatAxisValue(75000, "#,##0")).toBe("75,000");
    expect(formatAxisValue(75000, "#,##0", null)).toBe("75,000");
    expect(formatAxisValue(75000, "#,##0", 0)).toBe("75,000");
    expect(formatAxisValue(75000, "#,##0", NaN)).toBe("75,000");
  });

  it("supports millions / custom divisors", () => {
    expect(formatAxisValue(2_500_000, "#,##0.0", 1_000_000)).toBe("2.5");
    expect(formatAxisValue(123_456, "0", 123)).toBe("1004");
  });

  it("respects percent format after divisor (rare but legal)", () => {
    // 0.5 / 1 → 0.5 → "50%".
    expect(formatAxisValue(0.5, "0%", undefined)).toBe("50%");
  });
});

describe("zeroAxisMetrics — shared zero-baseline helper", () => {
  const inner = { x: 100, y: 50, w: 400, h: 200 };

  it("reports straddlesZero on a mixed-sign range and projects zero", () => {
    // Chart_Chart_17 case: minV=-250, maxV=750.
    // zeroFrac = (0 - -250) / 1000 = 0.25.
    // zeroY = 50 + (1 - 0.25)*200 = 50 + 150 = 200.
    const z = zeroAxisMetrics(inner, -250, 750);
    expect(z.straddlesZero).toBe(true);
    expect(z.zeroFrac).toBeCloseTo(0.25, 6);
    expect(z.zeroY).toBeCloseTo(200, 6);
    expect(z.zeroX).toBeCloseTo(100 + 0.25 * 400, 6);
  });

  it("clamps zeroFrac on entirely-non-negative ranges to 0 (zeroY → bottom)", () => {
    // minV=0, maxV=100. rawFrac = 0; zeroY = inner.y + inner.h = bottom.
    const z = zeroAxisMetrics(inner, 0, 100);
    expect(z.straddlesZero).toBe(false);
    expect(z.zeroFrac).toBe(0);
    expect(z.zeroY).toBe(inner.y + inner.h);
  });

  it("clamps zeroFrac on entirely-negative ranges to 1 (zeroY → top)", () => {
    // minV=-100, maxV=-10. rawFrac = (0 - -100)/90 = 1.111 → clamped to 1.
    const z = zeroAxisMetrics(inner, -100, -10);
    expect(z.straddlesZero).toBe(false);
    expect(z.zeroFrac).toBe(1);
    expect(z.zeroY).toBe(inner.y);
  });

  it("handles a zero-width range without producing NaN", () => {
    const z = zeroAxisMetrics(inner, 5, 5);
    expect(z.straddlesZero).toBe(false);
    expect(Number.isFinite(z.zeroFrac)).toBe(true);
    expect(Number.isFinite(z.zeroY)).toBe(true);
  });

  it("agrees with axisStraddlesZero / isZeroTickInside on edge cases", () => {
    expect(axisStraddlesZero(-1, 1)).toBe(true);
    expect(axisStraddlesZero(0, 10)).toBe(false);
    expect(axisStraddlesZero(-10, 0)).toBe(false);
    expect(isZeroTickInside(0, -10, 10)).toBe(true);
    expect(isZeroTickInside(0, 0, 10)).toBe(false); // not straddling
    expect(isZeroTickInside(5, -10, 10)).toBe(false);
  });
});

describe("resolveAxisRange — <c:majorUnit> cadence", () => {
  it("AGS NWC: max=45000, majorUnit=9000, positive data → 0/9000/.../45000", () => {
    // ECMA-376 §21.2.2.121 in source units. dispUnits scaling is the
    // tick-label formatter's job, not resolveAxisRange's — we just
    // emit the raw tick positions. NWC line chart's authored
    // `<c:max val="45000"/>` + `<c:majorUnit val="9000"/>` over data
    // ~17869..43118 must produce 0/9000/18000/27000/36000/45000 so
    // formatAxisValue(..., 1000) renders 0/9/18/27/36/45 to match
    // Excel's authored cadence.
    const r = resolveAxisRange(17869, 43118, undefined, 45000, false, 5, 9000);
    expect(r.ticks).toEqual([0, 9000, 18000, 27000, 36000, 45000]);
    expect(r.minV).toBe(0);
    expect(r.maxV).toBe(45000);
  });

  it("respects forcedMin verbatim and anchors step cadence above it", () => {
    // Workbook explicitly pinned min=100; the implicit walk-to-zero
    // path must not override that.
    const r = resolveAxisRange(115, 136, 100, 140, false, 5, 10);
    expect(r.minV).toBe(100);
    expect(r.maxV).toBe(140);
    expect(r.ticks).toEqual([100, 110, 120, 130, 140]);
  });

  it("falls back to niceTicks when majorUnit is absent", () => {
    // No majorUnit + zeroClamp=true (bar default) + positive data.
    // niceTicks for 0..43 with count=5 picks step=10 → 0..50.
    const r = resolveAxisRange(17, 43, undefined, undefined, true, 5);
    expect(r.ticks[0]).toBe(0);
    expect(r.ticks[r.ticks.length - 1]).toBeGreaterThanOrEqual(43);
  });

  it("caps the walk-to-zero extension at 14 ticks to avoid blow-ups", () => {
    // Tiny step (1) over data 100..200 — walking to zero would produce
    // 201 ticks. The guard must fall back to anchoring at dataMin so
    // we don't generate a hundred-tick axis.
    const r = resolveAxisRange(100, 200, undefined, 200, false, 5, 1);
    // Without the cap: 201 ticks starting at 0. With cap: ticks start
    // at or near dataMin (100 or floor-of-100 = 100).
    expect(r.ticks.length).toBeLessThan(120);
    expect(r.minV).toBeGreaterThanOrEqual(100);
  });

  it("handles dataMin straddling zero: walks one step below zero", () => {
    // Negative-positive mixed data with majorUnit=10. Walk-down
    // should descend past zero to bracket the negative tail at a
    // multiple of the step.
    const r = resolveAxisRange(-15, 30, undefined, 30, false, 5, 10);
    expect(r.minV).toBeLessThanOrEqual(-15);
    expect(r.maxV).toBe(30);
    // Cadence stays on multiples of 10.
    for (const t of r.ticks) {
      expect(Math.abs(t / 10 - Math.round(t / 10))).toBeLessThan(1e-9);
    }
  });

  it("rejects invalid majorUnit (zero / negative / non-finite) and uses niceTicks", () => {
    const r1 = resolveAxisRange(0, 100, undefined, undefined, true, 5, 0);
    const r2 = resolveAxisRange(0, 100, undefined, undefined, true, 5, -10);
    const r3 = resolveAxisRange(0, 100, undefined, undefined, true, 5, Infinity);
    // All three should fall through to niceTicks identically.
    expect(r1.ticks).toEqual(r2.ticks);
    expect(r1.ticks).toEqual(r3.ticks);
  });
});
