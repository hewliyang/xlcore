import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, test } from "vitest";

// Exercises the built artifact (like cli.test.ts): src/node.ts pulls in
// skia-canvas + wasm, which only resolve cleanly post-build.
const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const repoRoot = resolve(packageRoot, "../..");
const fixture = resolve(repoRoot, "tests/fixtures/shapes/basic-autoshapes.xlsx");

type NodeModule = typeof import("./node.js");

async function loadNode(): Promise<NodeModule> {
  return (await import(resolve(packageRoot, "dist/node.js"))) as NodeModule;
}

describe("renderToCanvas viewport", () => {
  test("explicit width/height yields an exact-size canvas (headers on)", async () => {
    const { loadWorkbookFromXlsx, renderToCanvas } = await loadNode();
    const layout = await loadWorkbookFromXlsx(fixture);
    const canvas = renderToCanvas(layout, { width: 620, height: 420 });
    expect({ w: canvas.width, h: canvas.height }).toEqual({ w: 620, h: 420 });
  });

  test("explicit width/height with renderHeaders:false crops the header band", async () => {
    const { loadWorkbookFromXlsx, renderToCanvas } = await loadNode();
    const layout = await loadWorkbookFromXlsx(fixture);
    const canvas = renderToCanvas(layout, { renderHeaders: false, width: 620, height: 420 });
    expect({ w: canvas.width, h: canvas.height }).toEqual({ w: 620, h: 420 });
    // Top-left must be cell content, not the white band where headers were:
    // A1 hosts a filled rect shape in this fixture, so probe the gridline at y=0.
    const ctx = canvas.getContext("2d");
    const row0 = ctx.getImageData(0, 0, canvas.width, 1).data;
    let nonWhite = 0;
    for (let i = 0; i < row0.length; i += 4) {
      if (row0[i] !== 255 || row0[i + 1] !== 255 || row0[i + 2] !== 255) nonWhite++;
    }
    expect(nonWhite).toBeGreaterThan(0);
  });

  test("scale multiplies the explicit dimensions", async () => {
    const { loadWorkbookFromXlsx, renderToCanvas } = await loadNode();
    const layout = await loadWorkbookFromXlsx(fixture);
    const canvas = renderToCanvas(layout, {
      renderHeaders: false,
      width: 300,
      height: 200,
      scale: 2,
    });
    expect({ w: canvas.width, h: canvas.height }).toEqual({ w: 600, h: 400 });
  });

  test("default viewport grows to include drawings past 1244×822", async () => {
    const { loadWorkbookFromXlsx, renderToCanvas } = await loadNode();
    const layout = await loadWorkbookFromXlsx(fixture);
    const sheet = layout.sheets[0]!;
    // Re-anchor one shape far beyond the historical default viewport.
    const drawing = sheet.drawings![0]!;
    drawing.anchor = {
      ...drawing.anchor,
      fromCol: 30,
      fromRow: 60,
      toCol: 33,
      toRow: 65,
      fromColOffEmu: 0,
      fromRowOffEmu: 0,
      toColOffEmu: 0,
      toRowOffEmu: 0,
      extEmuCx: undefined,
      extEmuCy: undefined,
    };
    const warnings: string[] = [];
    const canvas = renderToCanvas(layout, { onWarning: (m) => warnings.push(m) });
    // Default grid: 64px cols / 20px rows + headers; col 33 ends ≈ 44+33*64 = 2156.
    expect(canvas.width).toBeGreaterThan(2000);
    expect(canvas.height).toBeGreaterThan(1300);
    expect(warnings).toEqual([]);
  });

  test("warns when drawings exceed the 4096px auto-viewport cap", async () => {
    const { loadWorkbookFromXlsx, renderToCanvas } = await loadNode();
    const layout = await loadWorkbookFromXlsx(fixture);
    const sheet = layout.sheets[0]!;
    const drawing = sheet.drawings![0]!;
    drawing.anchor = {
      ...drawing.anchor,
      fromCol: 70,
      fromRow: 1,
      toCol: 75,
      toRow: 6,
      fromColOffEmu: 0,
      fromRowOffEmu: 0,
      toColOffEmu: 0,
      toRowOffEmu: 0,
      extEmuCx: undefined,
      extEmuCy: undefined,
    };
    const warnings: string[] = [];
    const canvas = renderToCanvas(layout, { onWarning: (m) => warnings.push(m) });
    expect(canvas.width).toBeLessThanOrEqual(4096);
    expect(warnings).toHaveLength(1);
    expect(warnings[0]).toContain("auto-viewport cap");
    expect(warnings[0]).toContain("--width/--height");
  });

  test("renderGridLines:false suppresses gridlines", async () => {
    const { loadWorkbookFromXlsx, renderToCanvas } = await loadNode();
    const layout = await loadWorkbookFromXlsx(fixture);
    const on = renderToCanvas(layout, { renderHeaders: false, width: 620, height: 420 });
    const off = renderToCanvas(layout, {
      renderHeaders: false,
      renderGridLines: false,
      width: 620,
      height: 420,
    });
    const count = (canvas: typeof on) => {
      const ctx = canvas.getContext("2d");
      const data = ctx.getImageData(0, 0, canvas.width, canvas.height).data;
      let gridPx = 0;
      // GRID_COLOR #d9d9d9
      for (let i = 0; i < data.length; i += 4) {
        if (data[i] === 0xd9 && data[i + 1] === 0xd9 && data[i + 2] === 0xd9) gridPx++;
      }
      return gridPx;
    };
    expect(count(on)).toBeGreaterThan(1000);
    expect(count(off)).toBe(0);
  });
});
