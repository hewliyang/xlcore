import { dirname, resolve } from "node:path";
import { writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const pkg = resolve(here, "../../../packages/xlsx-preview");
const { Workbook } = await import(resolve(pkg, "dist/node.js"));

const wb = await Workbook.create();
const s = wb.sheet("Sheet1");
s.cell({ row: 1, column: 1 }).setValue("Quarter");
s.cell({ row: 1, column: 2 }).setValue("Sales");
s.cell({ row: 1, column: 3 }).setValue("Costs");
const cats = ["Q1", "Q2", "Q3", "Q4"];
const sales = [120, 145, 98, 167];
const costs = [80, 92, 75, 101];
for (let i = 0; i < cats.length; i++) {
  s.cell({ row: i + 2, column: 1 }).setValue(cats[i]);
  s.cell({ row: i + 2, column: 2 }).setValue(sales[i]);
  s.cell({ row: i + 2, column: 3 }).setValue(costs[i]);
}

s.charts.set({
  kind: "column",
  title: "Sales vs Costs",
  anchor: { fromColumn: 4, fromRow: 1, toColumn: 13, toRow: 22 },
  categoriesRef: "Sheet1!$A$2:$A$5",
  legendPosition: "bottom",
  dataTable: {
    showKeys: true,
    showHorzBorder: true,
    showVertBorder: true,
    showOutline: true,
  },
  series: [
    {
      name: "Sales",
      valuesRef: "Sheet1!$B$2:$B$5",
      color: "4472C4",
    },
    {
      name: "Costs",
      valuesRef: "Sheet1!$C$2:$C$5",
      color: "ED7D31",
    },
  ],
});

const bytes = wb.save();
const out = resolve(here, "chart-data-table.xlsx");
writeFileSync(out, bytes);
console.log(out);
