import { dirname, resolve } from "node:path";
import { writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const pkg = resolve(here, "../../../packages/xlsx-preview");
const { Workbook } = await import(resolve(pkg, "dist/node.js"));

const wb = await Workbook.create();
const s = wb.sheet("Sheet1");

const cats = ["C1", "C2", "C3", "C4", "C5", "C6"];
const seriesNames = ["R1", "R2", "R3", "R4"];
s.cell({ row: 1, column: 1 }).setValue("X");
for (let c = 0; c < cats.length; c++) {
  s.cell({ row: 1, column: c + 2 }).setValue(cats[c]);
}
for (let r = 0; r < seriesNames.length; r++) {
  s.cell({ row: r + 2, column: 1 }).setValue(seriesNames[r]);
  for (let c = 0; c < cats.length; c++) {
    const x = (c / (cats.length - 1)) * 2 - 1;
    const y = (r / (seriesNames.length - 1)) * 2 - 1;
    const z = Math.round((Math.exp(-(x * x + y * y) * 1.4) * 100 + 5) * 10) / 10;
    s.cell({ row: r + 2, column: c + 2 }).setValue(z);
  }
}

const cat = "Sheet1!$B$1:$G$1";
const seriesSpec = seriesNames.map((name, r) => ({
  name,
  valuesRef: `Sheet1!$B$${r + 2}:$G$${r + 2}`,
}));

s.charts.set({
  kind: "surface3d",
  title: "Surface (filled bands)",
  anchor: { fromColumn: 8, fromRow: 1, toColumn: 17, toRow: 20 },
  categoriesRef: cat,
  legendPosition: "bottom",
  view3d: { rotX: 15, rotY: 20, rightAngleAxes: false, depthPercent: 100 },
  floor: { fill: "D9D9D9" },
  sideWall: { fill: "EFEFEF" },
  backWall: { fill: "F4F4F4" },
  series: seriesSpec,
});

s.charts.set({
  kind: "surface3d",
  title: "Surface (wireframe)",
  anchor: { fromColumn: 8, fromRow: 22, toColumn: 17, toRow: 41 },
  categoriesRef: cat,
  legendPosition: "bottom",
  wireframe: true,
  view3d: { rotX: 15, rotY: 20, rightAngleAxes: false, depthPercent: 100 },
  series: seriesSpec,
});

const bytes = wb.save();
const out = resolve(here, "chart-surface.xlsx");
writeFileSync(out, bytes);
console.log(out);
