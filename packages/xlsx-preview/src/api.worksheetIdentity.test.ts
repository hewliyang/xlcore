import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { readFileSync } from "node:fs";
import { describe, expect, test } from "vitest";

const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");

type ApiModule = typeof import("./api.js");

async function loadApi(): Promise<ApiModule> {
  return (await import(resolve(packageRoot, "dist/api.js"))) as ApiModule;
}

function wasmBytes(): Uint8Array {
  return readFileSync(resolve(packageRoot, "dist/xlcore_wasm_bg.wasm"));
}

describe("worksheet identity", () => {
  test("sheet() returns the same object per stable id", async () => {
    const { Workbook } = await loadApi();
    const wb = await Workbook.create({ wasmBinaryUrl: wasmBytes() });
    try {
      const a = wb.sheet("Sheet1");
      const b = wb.sheet("Sheet1");
      expect(a).toBe(b);
      expect(wb.worksheets()[0]).toBe(a);
      expect(wb.activeSheet()).toBe(a);
    } finally {
      wb.dispose();
    }
  });

  test("rename on one handle is visible through every other", async () => {
    const { Workbook } = await loadApi();
    const wb = await Workbook.create({ wasmBinaryUrl: wasmBytes() });
    try {
      const first = wb.sheet("Sheet1");
      const second = wb.sheet("Sheet1");
      first.cell("A1").setValue(1);

      first.rename("Renamed");

      expect(second.name).toBe("Renamed");
      expect(first.name).toBe("Renamed");
      expect(wb.sheet("Renamed")).toBe(first);
      expect(() => wb.sheet("Sheet1")).toThrow();

      second.cell("A2").setValue(2);
      expect(first.cell("A2").info().value).toEqual({ type: "number", value: 2 });
    } finally {
      wb.dispose();
    }
  });

  test("addSheet caches and removeSheet evicts", async () => {
    const { Workbook } = await loadApi();
    const wb = await Workbook.create({ wasmBinaryUrl: wasmBytes() });
    try {
      const added = wb.addSheet("Extra");
      expect(wb.sheet("Extra")).toBe(added);
      wb.removeSheet("Extra");
      expect(() => wb.sheet("Extra")).toThrow();
    } finally {
      wb.dispose();
    }
  });
});
