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

const TRICKY = "Q1 'Final' Inputs";

describe("range ops on quoted sheet names", () => {
  test("cell and range ops round-trip through spaces + apostrophes", async () => {
    const { Workbook } = await loadApi();
    const wb = await Workbook.create({ wasmBinaryUrl: wasmBytes() });
    try {
      const sheet = wb.addSheet(TRICKY);

      sheet.cell("B2").setValue(41);
      sheet.cell("C2").setFormula("=B2+1");
      expect(sheet.cell("C2").formula()).toBe("B2+1");
      expect(sheet.cell("B2").info().sheet).toBe(TRICKY);

      sheet.range("A1:B1").setValues([[1, 2]]);
      expect(sheet.range("A1:B1").values()).toEqual([
        [
          { type: "number", value: 1 },
          { type: "number", value: 2 },
        ],
      ]);

      sheet.range("A1").setStyle({ font: { bold: true } });
      sheet.cell("B2").clear();
      expect(sheet.cell("B2").value()).toEqual({ type: "blank" });

      const deps = sheet.cell("C2").precedents();
      expect(deps.map((d) => d.reference)).toEqual(["B2"]);
    } finally {
      wb.dispose();
    }
  });

  test("copyTo / fillTo carry the quoted sheet without TS string parsing", async () => {
    const { Workbook } = await loadApi();
    const wb = await Workbook.create({ wasmBinaryUrl: wasmBytes() });
    try {
      const sheet = wb.addSheet(TRICKY);
      const other = wb.sheet("Sheet1");

      sheet.cell("A1").setValue(5);
      const copied = sheet.range("A1").copyTo(other.range("D4"));
      expect(copied.reference).toBe("D4:D4");
      expect(other.cell("D4").value()).toEqual({ type: "number", value: 5 });

      sheet.range("A1").setValues([[9]]);
      const filled = sheet.range("A1").fillTo("A1:A3");
      expect(filled.sheet).toBe(TRICKY);
      expect(sheet.cell("A3").value()).toEqual({ type: "number", value: 9 });
    } finally {
      wb.dispose();
    }
  });
});
