import { dirname, resolve } from "node:path";
import { writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const pkg = resolve(here, "../../../packages/xlsx-preview");
const { Workbook } = await import(resolve(pkg, "dist/node.js"));

const wb = await Workbook.create();
const s = wb.sheet("Sheet1");
s.cell({ row: 1, column: 1 }).setValue("Month");
s.cell({ row: 1, column: 2 }).setValue("Sales");
s.cell({ row: 1, column: 3 }).setValue("Cost");
const sales = [12, 19, 17, 28, 33, 31, 44, 51, 49, 62];
const cost = [40, 35, 33, 28, 26, 22, 21, 18, 16, 14];
for (let i = 0; i < sales.length; i++) {
  s.cell({ row: i + 2, column: 1 }).setValue(`M${i + 1}`);
  s.cell({ row: i + 2, column: 2 }).setValue(sales[i]);
  s.cell({ row: i + 2, column: 3 }).setValue(cost[i]);
}

s.charts.set({
  kind: "line",
  title: "Trendlines",
  anchor: { fromColumn: 4, fromRow: 1, toColumn: 13, toRow: 20 },
  categoriesRef: "Sheet1!$A$2:$A$11",
  legendPosition: "bottom",
  series: [
    {
      name: "Sales",
      valuesRef: "Sheet1!$B$2:$B$11",
      color: "4472C4",
      trendline: { type: "linear", forward: 1 },
    },
    {
      name: "Cost",
      valuesRef: "Sheet1!$C$2:$C$11",
      color: "ED7D31",
      trendline: { type: "movingAvg", period: 3 },
    },
  ],
});

const bytes = wb.save();
const out = resolve(here, "chart-trendline.xlsx");
writeFileSync(out, bytes);
console.log(out);
