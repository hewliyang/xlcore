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
    const m = computeBarSlotMetrics(100, 1, false, 150, 0);
    expect(m.barW).toBeCloseTo(40, 5);
    expect(m.firstBarLeftOffset).toBeCloseTo(30, 5);
    expect(m.barShift).toBeCloseTo(40, 5);
  });

  it("Excel default stacked (overlap forced to 100, gapWidth=150)", () => {
    const m = computeBarSlotMetrics(100, 3, true, 150, undefined);
    expect(m.barW).toBeCloseTo(40, 5);
    expect(m.firstBarLeftOffset).toBeCloseTo(30, 5);
  });

  it("AGS Chart 19 clustered with gapWidth=219 + overlap=-27", () => {
    const m = computeBarSlotMetrics(100, 1, false, 219, -27);
    expect(m.barW).toBeCloseTo(100 / 3.19, 4);
  });

  it("3 clustered series with overlap=-27 (extra space between bars)", () => {
    const m = computeBarSlotMetrics(100, 3, false, 150, -27);
    expect(m.barW).toBeCloseTo(100 / 5.04, 4);
    expect(m.barShift).toBeCloseTo((100 / 5.04) * 1.27, 4);
  });

  it("clamps gapWidth to spec range 0..500", () => {
    const wide = computeBarSlotMetrics(100, 1, false, 1000, 0);
    expect(wide.barW).toBeCloseTo(100 / (1 + 5), 5);
    const negative = computeBarSlotMetrics(100, 1, false, -50, 0);
    expect(negative.barW).toBeCloseTo(100, 5);
  });

  it("clamps overlap to spec range -100..100 (clustered only)", () => {
    const m = computeBarSlotMetrics(100, 3, false, 150, 200);
    expect(m.barW).toBeCloseTo(40, 5);
    expect(m.barShift).toBeCloseTo(0, 5);
  });

  it("missing values fall back to spec defaults", () => {
    const m = computeBarSlotMetrics(100, 1, false, undefined, undefined);
    expect(m.barW).toBeCloseTo(40, 5);
  });

  it("first bar offset centers the bar group inside the slot", () => {
    const m = computeBarSlotMetrics(100, 4, false, 100, 0);

    expect(m.barW).toBeCloseTo(20, 5);
    expect(m.firstBarLeftOffset).toBeCloseTo(10, 5);
  });
});

describe("formatAxisValue — dispUnits divisor", () => {
  it("divides by 1000 for thousands and keeps the format code", () => {
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
    expect(formatAxisValue(0.5, "0%", undefined)).toBe("50%");
  });
});

describe("zeroAxisMetrics — shared zero-baseline helper", () => {
  const inner = { x: 100, y: 50, w: 400, h: 200 };

  it("reports straddlesZero on a mixed-sign range and projects zero", () => {
    const z = zeroAxisMetrics(inner, -250, 750);
    expect(z.straddlesZero).toBe(true);
    expect(z.zeroFrac).toBeCloseTo(0.25, 6);
    expect(z.zeroY).toBeCloseTo(200, 6);
    expect(z.zeroX).toBeCloseTo(100 + 0.25 * 400, 6);
  });

  it("clamps zeroFrac on entirely-non-negative ranges to 0 (zeroY → bottom)", () => {
    const z = zeroAxisMetrics(inner, 0, 100);
    expect(z.straddlesZero).toBe(false);
    expect(z.zeroFrac).toBe(0);
    expect(z.zeroY).toBe(inner.y + inner.h);
  });

  it("clamps zeroFrac on entirely-negative ranges to 1 (zeroY → top)", () => {
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
    expect(isZeroTickInside(0, 0, 10)).toBe(false);
    expect(isZeroTickInside(5, -10, 10)).toBe(false);
  });
});

describe("resolveAxisRange — <c:majorUnit> cadence", () => {
  it("AGS NWC: max=45000, majorUnit=9000, positive data → 0/9000/.../45000", () => {
    const r = resolveAxisRange(17869, 43118, undefined, 45000, 5, 9000);
    expect(r.ticks).toEqual([0, 9000, 18000, 27000, 36000, 45000]);
    expect(r.minV).toBe(0);
    expect(r.maxV).toBe(45000);
  });

  it("respects forcedMin verbatim and anchors step cadence above it", () => {
    const r = resolveAxisRange(115, 136, 100, 140, 5, 10);
    expect(r.minV).toBe(100);
    expect(r.maxV).toBe(140);
    expect(r.ticks).toEqual([100, 110, 120, 130, 140]);
  });

  it("falls back to niceTicks when majorUnit is absent", () => {
    const r = resolveAxisRange(17, 43, undefined, undefined, 5);
    expect(r.ticks[0]).toBe(0);
    expect(r.ticks[r.ticks.length - 1]).toBeGreaterThanOrEqual(43);
  });

  it("caps the walk-to-zero extension at 14 ticks to avoid blow-ups", () => {
    const r = resolveAxisRange(100, 200, undefined, 200, 5, 1);

    expect(r.ticks.length).toBeLessThan(120);
    expect(r.minV).toBeGreaterThanOrEqual(100);
  });

  it("handles dataMin straddling zero: walks one step below zero", () => {
    const r = resolveAxisRange(-15, 30, undefined, 30, 5, 10);
    expect(r.minV).toBeLessThanOrEqual(-15);
    expect(r.maxV).toBe(30);

    for (const t of r.ticks) {
      expect(Math.abs(t / 10 - Math.round(t / 10))).toBeLessThan(1e-9);
    }
  });

  it("line case: dataMin 2765 / max 5218 → axis min 0 (5/6 rule)", () => {
    const r = resolveAxisRange(2765, 5218, undefined, undefined, 5);
    expect(r.minV).toBe(0);
  });

  it("bar case: dataMin 4224 / max 4980 → non-zero axis min (5/6 rule)", () => {
    const r = resolveAxisRange(4224, 4980, undefined, undefined, 5);
    expect(r.minV).toBeGreaterThan(0);
  });

  it("all-negative: dataMax -10 / min -100 → axis max 0 (5/6 rule)", () => {
    const r = resolveAxisRange(-100, -10, undefined, undefined, 5);
    expect(r.maxV).toBe(0);
  });

  it("all-negative tight: dataMax -90 / min -100 → non-zero axis max", () => {
    const r = resolveAxisRange(-100, -90, undefined, undefined, 5);
    expect(r.maxV).toBeLessThan(0);
  });

  it("rejects invalid majorUnit (zero / negative / non-finite) and uses niceTicks", () => {
    const r1 = resolveAxisRange(0, 100, undefined, undefined, 5, 0);
    const r2 = resolveAxisRange(0, 100, undefined, undefined, 5, -10);
    const r3 = resolveAxisRange(0, 100, undefined, undefined, 5, Infinity);

    expect(r1.ticks).toEqual(r2.ticks);
    expect(r1.ticks).toEqual(r3.ticks);
  });
});
