import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, test } from "vitest";

const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");

type ApiModule = typeof import("./api.js");

async function loadApi(): Promise<ApiModule> {
  return (await import(resolve(packageRoot, "dist/api.js"))) as ApiModule;
}

function wasmBytes(): Uint8Array {
  return readFileSync(resolve(packageRoot, "dist/xlcore_wasm_bg.wasm"));
}

describe("chart anchor round-trip", () => {
  test("charts.update persists a moved anchor through save/open", async () => {
    const { Workbook } = await loadApi();
    const bytes = readFileSync(resolve(packageRoot, "../../tests/fixtures/charts/chart-bar3d.xlsx"));
    const wb = await Workbook.open(bytes, { wasmBinaryUrl: wasmBytes() });
    try {
      const ws = wb.worksheets()[0]!;
      const chart = ws.charts.list()[0]!;
      const before = chart.anchor;
      ws.charts.update(chart.id, {
        anchor: {
          ...before,
          fromColumn: before.fromColumn + 3,
          fromRow: before.fromRow + 5,
          toColumn: before.toColumn + 3,
          toRow: before.toRow + 5,
        },
      });
      const saved = wb.save();
      const wb2 = await Workbook.open(saved, { wasmBinaryUrl: wasmBytes() });
      try {
        const after = wb2.worksheets()[0]!.charts.list()[0]!.anchor;
        expect(after.fromColumn).toBe(before.fromColumn + 3);
        expect(after.fromRow).toBe(before.fromRow + 5);
        expect(after.toColumn).toBe(before.toColumn + 3);
        expect(after.toRow).toBe(before.toRow + 5);
      } finally {
        wb2.dispose();
      }
    } finally {
      wb.dispose();
    }
  });
});
