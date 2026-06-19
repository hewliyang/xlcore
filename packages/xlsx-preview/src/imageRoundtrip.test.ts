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

const PNG_1X1 = new Uint8Array([
  0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
  0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f, 0x15, 0xc4,
  0x89, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0x00, 0x01, 0x00, 0x00,
  0x05, 0x00, 0x01, 0x0d, 0x0a, 0x2d, 0xb4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae,
  0x42, 0x60, 0x82,
]);

describe("image anchor round-trip", () => {
  test("images.update persists a moved anchor through save/open", async () => {
    const { Workbook } = await loadApi();
    const wb = await Workbook.create({ wasmBinaryUrl: wasmBytes() });
    try {
      const ws = wb.worksheets()[0]!;
      const created = ws.images.set({ name: "Logo", anchor: "B2:F11", bytes: PNG_1X1 });
      const updated = ws.images.update(created.id, { name: "Moved", anchor: "E7:I16" });
      expect(updated.anchor.fromColumn).toBe(4);
      expect(updated.anchor.toRow).toBe(16);

      const wb2 = await Workbook.open(wb.save(), { wasmBinaryUrl: wasmBytes() });
      try {
        const img = wb2.worksheets()[0]!.images.list()[0]!;
        expect(img.name).toBe("Moved");
        expect(img.anchor.fromColumn).toBe(4);
        expect(img.anchor.fromRow).toBe(6);
        expect(img.anchor.toColumn).toBe(9);
        expect(img.anchor.toRow).toBe(16);
      } finally {
        wb2.dispose();
      }
    } finally {
      wb.dispose();
    }
  });
});
