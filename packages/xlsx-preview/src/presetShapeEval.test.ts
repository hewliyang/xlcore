import { expect, test } from "vitest";

import { PRESET_SHAPES } from "./presetShapeData.generated.js";
import { hasPresetGeometry, presetTextRect, tracePresetIntoPath } from "./presetShapeEval.js";

type Call =
  | ["M", number, number]
  | ["L", number, number]
  | ["Q", number, number, number, number]
  | ["C", number, number, number, number, number, number]
  | ["E", number, number, number, number, number, number, number, boolean]
  | ["Z"];

function fakeCtx(): { calls: Call[]; ctx: CanvasRenderingContext2D } {
  const calls: Call[] = [];
  const ctx = {
    moveTo: (x: number, y: number) => calls.push(["M", x, y]),
    lineTo: (x: number, y: number) => calls.push(["L", x, y]),
    quadraticCurveTo: (cx: number, cy: number, x: number, y: number) =>
      calls.push(["Q", cx, cy, x, y]),
    bezierCurveTo: (c1x: number, c1y: number, c2x: number, c2y: number, x: number, y: number) =>
      calls.push(["C", c1x, c1y, c2x, c2y, x, y]),
    ellipse: (
      cx: number,
      cy: number,
      rx: number,
      ry: number,
      rot: number,
      sa: number,
      ea: number,
      ccw: boolean,
    ) => calls.push(["E", cx, cy, rx, ry, rot, sa, ea, ccw]),
    closePath: () => calls.push(["Z"]),
  } as unknown as CanvasRenderingContext2D;
  return { calls, ctx };
}

test("preset table covers the documented long-tail shapes", () => {
  for (const name of [
    "rect",
    "ellipse",
    "roundRect",
    "triangle",
    "diamond",
    "leftArrow",
    "rightArrow",
    "chevron",
    "pentagon",
    "hexagon",
    "octagon",
    "star5",
    "leftBrace",
    "leftBracket",
    "leftRightArrow",
    "donut",
    "cloud",
    "heart",
    "lightningBolt",
    "smileyFace",
    "sun",
    "moon",
    "arc",
    "blockArc",
    "callout1",
    "wedgeRoundRectCallout",
    "actionButtonHome",
    "flowChartDecision",
    "flowChartProcess",
    "flowChartTerminator",
    "mathPlus",
    "mathMinus",
    "mathMultiply",
    "mathDivide",
    "mathEqual",
    "mathNotEqual",
    "ribbon",
    "ribbon2",
    "wave",
    "doubleWave",
    "noSmoking",
    "plaque",
    "bevel",
    "can",
    "cube",
    "foldedCorner",
    "frame",
    "halfFrame",
    "parallelogram",
    "trapezoid",
    "teardrop",
    "irregularSeal1",
    "irregularSeal2",
    "star4",
    "star6",
    "star7",
    "star8",
    "star10",
    "star12",
    "star16",
    "star24",
    "star32",
  ]) {
    expect(hasPresetGeometry(name)).toBe(true);
  }
});

test("tracePresetIntoPath emits canvas calls for rect (sanity check)", () => {
  const { calls, ctx } = fakeCtx();
  const ok = tracePresetIntoPath(ctx, "rect", 10, 20, 100, 50);
  expect(ok).toBe(true);
  expect(calls[0]).toEqual(["M", 10, 20]);
  expect(calls[1]).toEqual(["L", 110, 20]);
  expect(calls[2]).toEqual(["L", 110, 70]);
  expect(calls[3]).toEqual(["L", 10, 70]);
  expect(calls[calls.length - 1]).toEqual(["Z"]);
});

test("tracePresetIntoPath honors avLst defaults (roundRect uses ss * 16667/100000)", () => {
  const { calls, ctx } = fakeCtx();
  tracePresetIntoPath(ctx, "roundRect", 0, 0, 100, 200);
  const first = calls.find((c) => c[0] === "M") as Call;
  expect(first[0]).toBe("M");
  expect(first[1] as number).toBeCloseTo(0, 6);
  expect(first[2] as number).toBeCloseTo(16.667, 2);
});

test("tracePresetIntoPath honors caller-supplied adjusts (roundRect adj=50000 = pill)", () => {
  const { calls, ctx } = fakeCtx();
  tracePresetIntoPath(ctx, "roundRect", 0, 0, 100, 200, [50000]);
  const first = calls.find((c) => c[0] === "M") as Call;
  expect(first[2] as number).toBeCloseTo(50, 6);
});

test("ellipse preset emits an arc command", () => {
  const { calls, ctx } = fakeCtx();
  tracePresetIntoPath(ctx, "ellipse", 0, 0, 100, 60);
  expect(calls.some((c) => c[0] === "E")).toBe(true);
});

test("presetTextRect returns reasonable bounds for plain rect", () => {
  const r = presetTextRect("rect", 100, 50);
  expect(r).toEqual({ l: 0, t: 0, r: 100, b: 50 });
});

test("every preset in the table evaluates without throwing", () => {
  const { ctx } = fakeCtx();
  let traced = 0;
  for (const name of Object.keys(PRESET_SHAPES)) {
    const ok = tracePresetIntoPath(ctx, name, 0, 0, 100, 80);
    if (ok) traced++;
  }
  expect(traced).toBe(Object.keys(PRESET_SHAPES).length);
});
