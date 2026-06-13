import { dirname, resolve } from "node:path";
import { writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const pkg = resolve(here, "../../../packages/xlsx-preview");
const { Workbook } = await import(resolve(pkg, "dist/node.js"));

const wb = await Workbook.create();
const s = wb.sheet("Sheet1");
s.cell({ row: 1, column: 1 }).setValue("Quarter");
s.cell({ row: 1, column: 2 }).setValue("North");
s.cell({ row: 1, column: 3 }).setValue("South");
const north = [38, 52, 47, 61];
const south = [29, 34, 41, 55];
const quarters = ["Q1", "Q2", "Q3", "Q4"];
for (let i = 0; i < quarters.length; i++) {
  s.cell({ row: i + 2, column: 1 }).setValue(quarters[i]);
  s.cell({ row: i + 2, column: 2 }).setValue(north[i]);
  s.cell({ row: i + 2, column: 3 }).setValue(south[i]);
}

s.charts.set({
  kind: "column",
  title: "Plot + Legend Styling",
  anchor: { fromColumn: 4, fromRow: 1, toColumn: 13, toRow: 20 },
  categoriesRef: "Sheet1!$A$2:$A$5",
  legendPosition: "bottom",
  plotArea: {
    fill: "FFF2CC",
    border: { widthEmu: 19050, dash: "dash" },
  },
  legend: {
    fill: "DDEBF7",
    border: { widthEmu: 12700 },
    font: { size: 12, bold: true, italic: true, color: "C0392B", typeface: "Georgia" },
  },
  series: [
    { name: "North", valuesRef: "Sheet1!$B$2:$B$5", color: "4472C4" },
    { name: "South", valuesRef: "Sheet1!$C$2:$C$5", color: "ED7D31" },
  ],
});

const bytes = wb.save();
const out = resolve(here, "chart-plot-legend-style.xlsx");
writeFileSync(out, bytes);
console.log(out);
