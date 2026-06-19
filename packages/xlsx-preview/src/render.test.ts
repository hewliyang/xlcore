import { expect, test, vi } from "vitest";
import { applyTint } from "./render";
import { drawDrawingSelection, drawingHandles } from "./drawingSelection";

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

  expect(got.slice(1, 3)).toBe(got.slice(3, 5));
  expect(got.slice(3, 5)).toBe(got.slice(5, 7));
});

test("tint of pure red stays on the red hue line", () => {
  const got = applyTint("#FF0000", 0.5);

  expect(close(got, "#ff8080", 2)).toBe(true);
});

test("zero tint is identity", () => {
  expect(applyTint("#4472C4", 0)).toBe("#4472c4");
});

test("drawingHandles yields 8 handles at corners and edge midpoints", () => {
  const handles = drawingHandles({ x: 0, y: 0, w: 100, h: 40 });
  expect(handles).toHaveLength(8);
  const centers = handles.map((h) => [h.x + h.w / 2, h.y + h.h / 2]);
  expect(centers).toContainEqual([50, 0]);
  expect(centers).toContainEqual([100, 20]);
  expect(centers).toContainEqual([0, 40]);
});

test("drawDrawingSelection draws box + 8 handles", () => {
  const ctx = {
    save: vi.fn(),
    restore: vi.fn(),
    strokeRect: vi.fn(),
    fillRect: vi.fn(),
    strokeStyle: "",
    fillStyle: "",
    lineWidth: 0,
  } as unknown as CanvasRenderingContext2D;
  drawDrawingSelection(ctx, { x: 10, y: 20, w: 200, h: 80 });
  expect((ctx.strokeRect as ReturnType<typeof vi.fn>).mock.calls).toHaveLength(9);
  expect((ctx.fillRect as ReturnType<typeof vi.fn>).mock.calls).toHaveLength(8);
});
