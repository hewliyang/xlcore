import { dirname, resolve } from "node:path";
import { writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const pkg = resolve(here, "../../../packages/xlsx-preview");
const { Workbook } = await import(resolve(pkg, "dist/node.js"));

const wb = await Workbook.create();
const s = wb.sheet("Sheet1");
s.cell({ row: 1, column: 1 }).setValue("Channel");
s.cell({ row: 1, column: 2 }).setValue("Revenue");
const rows = [
  ["Retail", 48],
  ["Online", 35],
  ["Wholesale", 27],
  ["Phone", 8],
  ["Mail", 5],
];
for (let i = 0; i < rows.length; i++) {
  s.cell({ row: i + 2, column: 1 }).setValue(rows[i][0]);
  s.cell({ row: i + 2, column: 2 }).setValue(rows[i][1]);
}

s.charts.set({
  kind: "pieofpie",
  title: "Pie of Pie",
  anchor: { fromColumn: 3, fromRow: 0, toColumn: 12, toRow: 18 },
  categoriesRef: "Sheet1!$A$2:$A$6",
  legendPosition: "bottom",
  splitType: "pos",
  splitPos: 2,
  secondPieSize: 75,
  seriesLines: true,
  series: [{ name: "Revenue", valuesRef: "Sheet1!$B$2:$B$6", showValue: true }],
});

s.charts.set({
  kind: "barofpie",
  title: "Bar of Pie",
  anchor: { fromColumn: 13, fromRow: 0, toColumn: 22, toRow: 18 },
  categoriesRef: "Sheet1!$A$2:$A$6",
  legendPosition: "bottom",
  splitType: "pos",
  splitPos: 2,
  secondPieSize: 75,
  seriesLines: true,
  series: [{ name: "Revenue", valuesRef: "Sheet1!$B$2:$B$6", showValue: true }],
});

const bytes = wb.save();
const out = resolve(here, "chart-ofpie.xlsx");
writeFileSync(out, bytes);
console.log(out);
