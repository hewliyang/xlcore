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

describe("parseFormulaReferences", () => {
  test("parses references from an uncommitted formula string", async () => {
    const { Workbook } = await loadApi();
    const wb = await Workbook.create({ wasmBinaryUrl: wasmBytes() });
    try {
      const refs = wb.parseFormulaReferences("Sheet1", "A1", "=B1+SUM(C1:C3)");
      const byRef = refs.map((r) => r.reference).sort();
      expect(byRef).toEqual(["B1", "C1:C3"]);
    } finally {
      wb.dispose();
    }
  });
});

describe("functionNames", () => {
  test("returns a sorted uppercase catalog including common functions", async () => {
    const { Workbook } = await loadApi();
    const wb = await Workbook.create({ wasmBinaryUrl: wasmBytes() });
    try {
      const names = wb.functionNames();
      expect(names.length).toBeGreaterThan(300);
      expect(names).toContain("SUM");
      expect(names).toContain("IF");
      expect(names).toContain("XLOOKUP");
      for (const name of names) expect(name).toBe(name.toUpperCase());
      const sorted = [...names].sort();
      expect(names).toEqual(sorted);
    } finally {
      wb.dispose();
    }
  });
});
