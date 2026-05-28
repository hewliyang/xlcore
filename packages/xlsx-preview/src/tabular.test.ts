import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";
import { describe, expect, test } from "vitest";

import { XlsxLoadError } from "./errors.js";
import { loadWorkbookFromCsvWithReport, loadWorkbookFromParquetWithReport } from "./node.js";
import type { Sheet } from "./schema/Sheet.js";
import type { WorkbookLayout } from "./schema/WorkbookLayout.js";

const FIXTURES = resolve(dirname(fileURLToPath(import.meta.url)), "../../../tests/fixtures");

function cellAt(
  sheet: Sheet,
  layout: WorkbookLayout,
  r: number,
  c: number,
): { kind: string; value: string } | null {
  const cells = sheet.cells;
  if (!cells || !cells.count) return null;
  const r32 = base64ToU32(cells.r);
  const c32 = base64ToU32(cells.c);
  const kinds = base64ToU8(cells.kind);
  const valueIdx = base64ToI32(cells.valueIdx);
  for (let i = 0; i < cells.count; i++) {
    if (r32[i] === r && c32[i] === c) {
      const kind = ["n", "s", "inline", "b", "e", "str", "f"][kinds[i] ?? 0] ?? "n";
      const vi = valueIdx[i] ?? -1;
      const valFromPool = vi >= 0 ? sheet.valuePool?.[vi] : undefined;
      const value =
        kind === "s"
          ? (layout.sharedStrings?.[Number(valFromPool ?? "0")] ?? "")
          : (valFromPool ?? "");
      return { kind, value };
    }
  }
  return null;
}

function base64ToU8(s: string): Uint8Array {
  return Uint8Array.from(Buffer.from(s, "base64"));
}
function base64ToU32(s: string): Uint32Array {
  const b = base64ToU8(s);
  return new Uint32Array(b.buffer, b.byteOffset, b.byteLength / 4);
}
function base64ToI32(s: string): Int32Array {
  const b = base64ToU8(s);
  return new Int32Array(b.buffer, b.byteOffset, b.byteLength / 4);
}

describe("csv fixtures", () => {
  test("basic: mixed types, header in row 1, numbers right-aligned, bools as 'b'", async () => {
    const { layout, report } = await loadWorkbookFromCsvWithReport(
      await readFile(`${FIXTURES}/csv/basic.csv`),
      { sheetName: "basic" },
    );
    expect(report.warnings).toEqual([]);
    expect(layout.sheets).toHaveLength(1);
    const sheet = layout.sheets[0]!;
    expect(sheet.name).toBe("basic");
    expect(sheet.maxRow).toBe(5);
    expect(sheet.maxCol).toBe(4);
    expect(cellAt(sheet, layout, 1, 1)).toEqual({ kind: "str", value: "name" });
    expect(cellAt(sheet, layout, 2, 1)).toEqual({ kind: "str", value: "Ada" });
    expect(cellAt(sheet, layout, 2, 2)).toEqual({ kind: "n", value: "36" });
    expect(cellAt(sheet, layout, 2, 3)).toEqual({ kind: "n", value: "0.95" });
    expect(cellAt(sheet, layout, 2, 4)).toEqual({ kind: "b", value: "1" });
    expect(cellAt(sheet, layout, 5, 3)).toEqual({ kind: "n", value: "1" });
    expect(cellAt(sheet, layout, 3, 4)).toEqual({ kind: "b", value: "0" });
  });

  test("semicolon: delimiter sniffer picks ';'", async () => {
    const { layout } = await loadWorkbookFromCsvWithReport(
      await readFile(`${FIXTURES}/csv/semicolon.csv`),
    );
    const sheet = layout.sheets[0]!;
    expect(sheet.maxCol).toBe(3);
    expect(cellAt(sheet, layout, 1, 1)).toEqual({ kind: "str", value: "city" });
    expect(cellAt(sheet, layout, 2, 1)).toEqual({ kind: "str", value: "Paris" });
    expect(cellAt(sheet, layout, 2, 2)).toEqual({ kind: "n", value: "2161000" });
  });

  test("leading-zeros: identifier-shaped tokens stay strings (ZIPs, phones)", async () => {
    const { layout } = await loadWorkbookFromCsvWithReport(
      await readFile(`${FIXTURES}/csv/leading-zeros.csv`),
    );
    const sheet = layout.sheets[0]!;
    expect(cellAt(sheet, layout, 2, 2)).toEqual({ kind: "str", value: "00123" });
    expect(cellAt(sheet, layout, 2, 3)).toEqual({ kind: "n", value: "5550199" });
  });

  test("ragged: max_col extends to widest row", async () => {
    const { layout } = await loadWorkbookFromCsvWithReport(
      await readFile(`${FIXTURES}/csv/ragged.csv`),
    );
    const sheet = layout.sheets[0]!;
    expect(sheet.maxCol).toBe(4);
    expect(cellAt(sheet, layout, 3, 3)).toEqual({ kind: "str", value: "z" });
    expect(cellAt(sheet, layout, 4, 4)).toEqual({ kind: "n", value: "4" });
  });

  test("max_rows truncation appends a single warning", async () => {
    const { layout, report } = await loadWorkbookFromCsvWithReport(
      await readFile(`${FIXTURES}/csv/basic.csv`),
      { maxRows: 2 },
    );
    expect(layout.sheets[0]!.maxRow).toBe(2);
    expect(report.warnings).toHaveLength(1);
    expect(report.warnings[0]).toMatch(/truncated.*2 of 5/);
  });

  test("invalid options throw XlsxLoadError", async () => {
    await expect(
      loadWorkbookFromCsvWithReport(await readFile(`${FIXTURES}/csv/basic.csv`), {
        delimiter: "::",
      }),
    ).rejects.toBeInstanceOf(XlsxLoadError);
  });
});

describe("parquet fixtures", () => {
  test("primitives: utf8 / int64 / float64 / boolean with one null", async () => {
    const { layout, report } = await loadWorkbookFromParquetWithReport(
      await readFile(`${FIXTURES}/parquet/primitives.parquet`),
      { sheetName: "primitives" },
    );
    expect(report.warnings).toEqual([]);
    const sheet = layout.sheets[0]!;
    expect(sheet.name).toBe("primitives");
    expect(sheet.maxRow).toBe(4);
    expect(sheet.maxCol).toBe(4);
    expect(cellAt(sheet, layout, 1, 1)).toEqual({ kind: "str", value: "name" });
    expect(cellAt(sheet, layout, 2, 2)).toEqual({ kind: "n", value: "36" });
    expect(cellAt(sheet, layout, 2, 3)).toEqual({ kind: "n", value: "0.95" });
    expect(cellAt(sheet, layout, 3, 3)).toBeNull();
    expect(cellAt(sheet, layout, 4, 4)).toEqual({ kind: "b", value: "1" });
  });

  test("temporal: Date32 / Timestamp(ms) / Time64(us) render as ISO strings", async () => {
    const { layout } = await loadWorkbookFromParquetWithReport(
      await readFile(`${FIXTURES}/parquet/temporal.parquet`),
    );
    const sheet = layout.sheets[0]!;
    expect(sheet.maxRow).toBe(4);
    expect(cellAt(sheet, layout, 2, 1)).toEqual({ kind: "str", value: "2024-01-15" });
    expect(cellAt(sheet, layout, 2, 2)).toEqual({
      kind: "str",
      value: "2023-11-14 22:13:20",
    });
    expect(cellAt(sheet, layout, 2, 3)).toEqual({ kind: "str", value: "09:00:00" });
    expect(cellAt(sheet, layout, 3, 2)).toBeNull();
    expect(cellAt(sheet, layout, 3, 3)).toEqual({ kind: "str", value: "14:30:15" });
  });

  test("nested: list / struct / map collapse to one-line ArrayFormatter output", async () => {
    const { layout } = await loadWorkbookFromParquetWithReport(
      await readFile(`${FIXTURES}/parquet/nested.parquet`),
    );
    const sheet = layout.sheets[0]!;
    expect(sheet.maxRow).toBe(4);
    expect(sheet.maxCol).toBe(3);
    expect(cellAt(sheet, layout, 2, 1)).toEqual({ kind: "str", value: "[1, 2, 3]" });
    expect(cellAt(sheet, layout, 3, 1)).toEqual({ kind: "str", value: "[]" });
    expect(cellAt(sheet, layout, 2, 2)).toEqual({
      kind: "str",
      value: "{name: Ada, age: 36}",
    });
    expect(cellAt(sheet, layout, 3, 2)).toEqual({
      kind: "str",
      value: "{name: Grace, age: }",
    });
    expect(cellAt(sheet, layout, 2, 3)).toEqual({
      kind: "str",
      value: "{k1: 1, k2: 2}",
    });
    expect(cellAt(sheet, layout, 3, 3)).toEqual({ kind: "str", value: "{}" });
  });

  test("maxRows truncation counts the synthetic header row", async () => {
    const { layout, report } = await loadWorkbookFromParquetWithReport(
      await readFile(`${FIXTURES}/parquet/primitives.parquet`),
      { maxRows: 2 },
    );
    const sheet = layout.sheets[0]!;
    expect(sheet.maxRow).toBe(2);
    expect(report.warnings).toHaveLength(1);
    expect(report.warnings[0]).toMatch(/truncated.*2 of 4 rows/);
    expect(cellAt(sheet, layout, 1, 1)).toEqual({ kind: "str", value: "name" });
    expect(cellAt(sheet, layout, 2, 1)).toEqual({ kind: "str", value: "Ada" });
    expect(cellAt(sheet, layout, 3, 1)).toBeNull();
  });

  test("invalid bytes throw XlsxLoadError", async () => {
    await expect(
      loadWorkbookFromParquetWithReport(new TextEncoder().encode("not parquet")),
    ).rejects.toBeInstanceOf(XlsxLoadError);
  });
});
