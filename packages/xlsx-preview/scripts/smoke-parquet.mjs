// Smoke test: load a committed Parquet fixture, render it to PNG, and print layout stats.
import { writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { loadWorkbookFromParquetWithReport, renderToPng } from "../dist/node.js";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../../..");
const output = process.argv[2] ?? "/tmp/parquet-smoke.png";
const input = resolve(root, "tests/fixtures/parquet/primitives.parquet");

const { layout, report } = await loadWorkbookFromParquetWithReport(input, {
  sheetName: "primitives",
});
const sheet = layout.sheets[0];

console.log(
  JSON.stringify(
    {
      input,
      sheets: layout.sheets.length,
      maxRow: sheet.maxRow,
      maxCol: sheet.maxCol,
      cols: sheet.cols.map((c) => Math.round(c.widthPx)),
      cells: sheet.cells.count,
      warnings: report.warnings,
    },
    null,
    2,
  ),
);

const png = await renderToPng(layout, { scale: 2 });
await writeFile(output, png);
console.log("wrote", output, png.length, "bytes");
