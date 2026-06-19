import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, test } from "vitest";
import { decodeWorkbookLayout, findCell } from "./columnar.js";
import { patchWorkbookSheet } from "./layoutPatch.js";
import type { Cell, Sheet, WorkbookLayout } from "./types.js";

const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");

type ApiModule = typeof import("./api.js");

async function loadApi(): Promise<ApiModule> {
  return (await import(resolve(packageRoot, "dist/api.js"))) as ApiModule;
}

function wasmBytes(): Uint8Array {
  return readFileSync(resolve(packageRoot, "dist/xlcore_wasm_bg.wasm"));
}

function resolveString(layout: WorkbookLayout, cell: Cell | undefined): string | undefined {
  if (!cell) return undefined;
  if (cell.type === "s") return layout.sharedStrings[Number(cell.value)] ?? "";
  return cell.value === undefined ? undefined : String(cell.value);
}

describe("patchWorkbookSheet", () => {
  test("patches active sheet, leaves others intact, indices resolve", async () => {
    const { Workbook } = await loadApi();

    const wb = await Workbook.create({ wasmBinaryUrl: wasmBytes() });
    try {
      const alpha = wb.sheet("Sheet1");
      alpha.cell("A1").setValue("hello");
      alpha.cell("A2").setValue(1);
      const beta = wb.addSheet("Beta");
      beta.cell("B1").setValue("world");

      const full = decodeWorkbookLayout(wb.layout({}) as WorkbookLayout);
      const alphaIdx = full.sheets.findIndex((s) => s.name === "Sheet1");
      const betaIdx = full.sheets.findIndex((s) => s.name === "Beta");
      expect(alphaIdx).toBeGreaterThanOrEqual(0);
      expect(betaIdx).toBeGreaterThanOrEqual(0);

      const alphaSheet = full.sheets[alphaIdx] as Sheet;
      const betaSheet = full.sheets[betaIdx] as Sheet;
      expect(resolveString(full, findCell(alphaSheet, 1, 1))).toBe("hello");
      expect(resolveString(full, findCell(betaSheet, 1, 2))).toBe("world");

      alpha.cell("A2").setValue(42);
      wb.recalculate();
      const single = wb.layout({ sheetName: "Sheet1" }) as WorkbookLayout;
      expect(single.sheets.length).toBe(1);

      const patched = patchWorkbookSheet(full, single);
      expect(patched).toBe(true);

      const patchedAlpha = full.sheets[alphaIdx] as Sheet;
      const patchedBeta = full.sheets[betaIdx] as Sheet;
      expect(Number(findCell(patchedAlpha, 2, 1)?.value)).toBe(42);
      expect(resolveString(full, findCell(patchedAlpha, 1, 1))).toBe("hello");
      expect(resolveString(full, findCell(patchedBeta, 1, 2))).toBe("world");

      expect(patchWorkbookSheet(full, { ...single, sheets: [] })).toBe(false);
    } finally {
      wb.dispose();
    }
  });
});
