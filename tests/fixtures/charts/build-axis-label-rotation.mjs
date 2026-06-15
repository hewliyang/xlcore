import { dirname, resolve } from "node:path";
import { writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const pkg = resolve(here, "../../../packages/xlsx-preview");
const { Workbook } = await import(resolve(pkg, "dist/node.js"));

const wb = await Workbook.create();
const s = wb.sheet("Sheet1");
s.cell({ row: 1, column: 1 }).setValue("Region");
s.cell({ row: 1, column: 2 }).setValue("Revenue");
const cats = [
  "North America",
  "Latin America",
  "Western Europe",
  "Eastern Europe",
  "Middle East",
  "Asia Pacific",
];
const vals = [128, 64, 96, 42, 38, 110];
for (let i = 0; i < cats.length; i++) {
  s.cell({ row: i + 2, column: 1 }).setValue(cats[i]);
  s.cell({ row: i + 2, column: 2 }).setValue(vals[i]);
}

s.charts.set({
  kind: "column",
  title: "Revenue by Region",
  anchor: { fromColumn: 4, fromRow: 1, toColumn: 13, toRow: 20 },
  categoriesRef: "Sheet1!$A$2:$A$7",
  legendPosition: "bottom",
  categoryAxis: { labelRotation: -45 },
  series: [
    {
      name: "Revenue",
      valuesRef: "Sheet1!$B$2:$B$7",
      color: "4472C4",
    },
  ],
});

const bytes = wb.save();
const out = resolve(here, "chart-axis-label-rotation.xlsx");
writeFileSync(out, bytes);
console.log(out);
