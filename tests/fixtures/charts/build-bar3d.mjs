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
const north = [18, 27, 23, 35];
const south = [12, 19, 31, 28];
for (let i = 0; i < north.length; i++) {
  s.cell({ row: i + 2, column: 1 }).setValue(`Q${i + 1}`);
  s.cell({ row: i + 2, column: 2 }).setValue(north[i]);
  s.cell({ row: i + 2, column: 3 }).setValue(south[i]);
}

s.charts.set({
  kind: "column3d",
  title: "3D Columns",
  anchor: { fromColumn: 4, fromRow: 1, toColumn: 13, toRow: 20 },
  categoriesRef: "Sheet1!$A$2:$A$5",
  legendPosition: "bottom",
  view3d: { rotX: 15, rotY: 20, rightAngleAxes: true, depthPercent: 100 },
  gapDepth: 150,
  floor: { fill: "D9D9D9" },
  sideWall: { fill: "EFEFEF" },
  backWall: { fill: "F4F4F4" },
  series: [
    { name: "North", valuesRef: "Sheet1!$B$2:$B$5", color: "4472C4" },
    { name: "South", valuesRef: "Sheet1!$C$2:$C$5", color: "ED7D31" },
  ],
});

const bytes = wb.save();
const out = resolve(here, "chart-bar3d.xlsx");
writeFileSync(out, bytes);
console.log(out);
