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

describe("worker range ops backing api", () => {
  test("setValues round-trips through save/reopen", async () => {
    const { Workbook } = await loadApi();
    const wb = await Workbook.create({ wasmBinaryUrl: wasmBytes() });
    try {
      wb.sheet("Sheet1").range("A1:B2").setValues([
        [1, 2],
        [3, 4],
      ]);
      const bytes = wb.save();
      const wb2 = await Workbook.open(bytes, { wasmBinaryUrl: wasmBytes() });
      try {
        const values = wb2.sheet("Sheet1").range("A1:B2").values();
        expect(values[0]?.[0]).toEqual({ type: "number", value: 1 });
        expect(values[1]?.[1]).toEqual({ type: "number", value: 4 });
      } finally {
        wb2.dispose();
      }
    } finally {
      wb.dispose();
    }
  });

  test("setFormulas recalc and copyRange preserve formulas", async () => {
    const { Workbook } = await loadApi();
    const wb = await Workbook.create({ wasmBinaryUrl: wasmBytes() });
    try {
      const sheet = wb.sheet("Sheet1");
      sheet.range("A1:A2").setValues([[10], [20]]);
      sheet.range("B1:B2").setFormulas([["=A1*2"], ["=A2*2"]]);
      wb.recalculate();
      expect(sheet.range("B1:B2").values()[0]?.[0]).toEqual({ type: "number", value: 20 });

      sheet.range("B1:B2").copyTo(sheet.range("C1:C2"));
      expect(sheet.cell("C1").formula()).toBe("B1*2");
    } finally {
      wb.dispose();
    }
  });

  test("images.set anchors image and round-trips through save/reopen", async () => {
    const { Workbook } = await loadApi();
    const png = Uint8Array.from([
      0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
      0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f, 0x15, 0xc4,
      0x89, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0x60, 0x00, 0x02, 0x00,
      0x00, 0x05, 0x00, 0x01, 0xe2, 0x26, 0x05, 0x9b, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44,
      0xae, 0x42, 0x60, 0x82,
    ]);
    const wb = await Workbook.create({ wasmBinaryUrl: wasmBytes() });
    try {
      wb.sheet("Sheet1").images.set({ anchor: "C3:E10", bytes: png, format: "png" });
      const bytes = wb.save();
      const wb2 = await Workbook.open(bytes, { wasmBinaryUrl: wasmBytes() });
      try {
        expect(wb2.sheet("Sheet1").images.list().length).toBeGreaterThan(0);
      } finally {
        wb2.dispose();
      }
    } finally {
      wb.dispose();
    }
  });

  test("clearRange empties cells", async () => {
    const { Workbook } = await loadApi();
    const wb = await Workbook.create({ wasmBinaryUrl: wasmBytes() });
    try {
      const sheet = wb.sheet("Sheet1");
      sheet.range("A1:B1").setValues([["x", "y"]]);
      sheet.range("A1:B1").clear();
      const values = sheet.range("A1:B1").values();
      expect(values[0]?.[0]).toEqual({ type: "blank" });
      expect(values[0]?.[1]).toEqual({ type: "blank" });
    } finally {
      wb.dispose();
    }
  });
});
