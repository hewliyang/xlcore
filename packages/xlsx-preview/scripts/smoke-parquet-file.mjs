// Render an arbitrary Parquet path to PNG for ad hoc real-world smoke testing.
import { readFile, writeFile } from "node:fs/promises";
import { basename } from "node:path";
import { loadWorkbookFromParquetWithReport, renderToPng } from "../dist/node.js";

const input = process.argv[2];
const output = process.argv[3] ?? "/tmp/parquet-out.png";
if (!input) {
  console.error("usage: smoke-parquet-file.mjs <in.parquet> [out.png]");
  process.exit(2);
}

const bytes = await readFile(input);
const { layout, report } = await loadWorkbookFromParquetWithReport(bytes, {
  sheetName: basename(input).replace(/\.parquet$/i, ""),
  maxRows: 200,
});
const sheet = layout.sheets[0];

console.log(
  JSON.stringify(
    {
      input,
      rows: sheet.maxRow,
      cols: sheet.maxCol,
      cellCount: sheet.cells.count,
      widths: sheet.cols.map((c) => Math.round(c.widthPx)),
      headers: sheet.valuePool.slice(0, sheet.maxCol),
      sampleValues: sheet.valuePool.slice(sheet.maxCol, sheet.maxCol + 20),
      warnings: report.warnings,
    },
    null,
    2,
  ),
);

const png = await renderToPng(layout, { scale: 2 });
await writeFile(output, png);
console.log("wrote", output, png.length, "bytes");
