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

describe("rich text runs in cells", () => {
  test("setRichText round-trips runs, flat value, and reload", async () => {
    const { Workbook } = await loadApi();
    const wb = await Workbook.create({ wasmBinaryUrl: wasmBytes() });
    try {
      const sheet = wb.sheet("Sheet1");
      sheet
        .cell("A1")
        .setRichText([
          { text: "Plain " },
          { text: "bold", font: { bold: true } },
          { text: " " },
          { text: "red", font: { italic: true, color: "#CC0000" } },
        ]);

      const info = sheet.cell("A1").info();
      expect(info.value).toEqual({ type: "string", value: "Plain bold red" });
      const runs = info.richText?.runs ?? [];
      expect(runs.length).toBe(4);
      expect(runs[0]?.font).toBeUndefined();
      expect(runs[1]).toEqual({ text: "bold", font: { bold: true } });
      expect(runs[3]?.font?.color).toBe("#FFCC0000");

      const bytes = wb.save();
      const wb2 = await Workbook.open(bytes, { wasmBinaryUrl: wasmBytes() });
      try {
        const back = wb2.sheet("Sheet1").cell("A1").richText()?.runs ?? [];
        expect(back.map((r) => r.text).join("")).toBe("Plain bold red");
        expect(back[1]?.font?.bold).toBe(true);
      } finally {
        wb2.dispose();
      }
    } finally {
      wb.dispose();
    }
  });

  test("setValue after setRichText drops the runs", async () => {
    const { Workbook } = await loadApi();
    const wb = await Workbook.create({ wasmBinaryUrl: wasmBytes() });
    try {
      const cell = wb.sheet("Sheet1").cell("B2");
      cell.setRichText([{ text: "a", font: { bold: true } }, { text: "b" }]);
      expect(cell.richText()?.runs.length).toBe(2);
      cell.setValue("plain");
      expect(cell.richText()).toBeUndefined();
      expect(cell.value()).toEqual({ type: "string", value: "plain" });
    } finally {
      wb.dispose();
    }
  });
});
