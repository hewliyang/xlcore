import { dirname, resolve } from "node:path";
import { writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const pkg = resolve(here, "../../../packages/xlsx-preview");
const { Workbook } = await import(resolve(pkg, "dist/node.js"));

const wb = await Workbook.create();
const s = wb.sheet("Sheet1");
s.cell({ row: 1, column: 1 }).setValue("Region");
s.cell({ row: 1, column: 2 }).setValue("Sales");
const labels = ["North", "South", "East", "West"];
const sales = [24, 38, 31, 45];
for (let i = 0; i < sales.length; i++) {
  s.cell({ row: i + 2, column: 1 }).setValue(labels[i]);
  s.cell({ row: i + 2, column: 2 }).setValue(sales[i]);
}

s.charts.set({
  kind: "column",
  title: "Per-point fills",
  anchor: { fromColumn: 3, fromRow: 0, toColumn: 12, toRow: 19 },
  categoriesRef: "Sheet1!$A$2:$A$5",
  legendPosition: "bottom",
  series: [
    {
      name: "Sales",
      valuesRef: "Sheet1!$B$2:$B$5",
      color: "4472C4",
      dataPoints: [
        {
          index: 0,
          gradientFill: {
            angle: 90,
            stops: [
              { position: 0, color: "FFC000" },
              { position: 100, color: "C00000" },
            ],
          },
        },
        {
          index: 2,
          patternFill: { preset: "ltUpDiag", foreground: "70AD47", background: "FFFFFF" },
        },
      ],
    },
  ],
});

const bytes = wb.save();
const out = resolve(here, "chart-point-fills.xlsx");
writeFileSync(out, bytes);
console.log(out);
