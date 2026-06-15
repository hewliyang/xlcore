#!/usr/bin/env node
import { readFileSync, writeFileSync } from "node:fs";
import { Workbook } from "../dist/node.js";

const wasm = readFileSync(new URL("../dist/xlcore_wasm_bg.wasm", import.meta.url));
const wb = await Workbook.create({ wasmBinaryUrl: wasm });

const s = wb.addSheet("Random");
wb.removeSheet("Sheet1");

s.cell("A1").setValue("Recalc demo — click Recalc to re-roll volatile formulas (watch the conditional colors)");
s.cell("A1").setStyle({ font: { bold: true } });

s.cell("A3").setValue("Label");
s.cell("B3").setValue("Formula");
s.cell("C3").setValue("Value");
for (const ref of ["A3", "B3", "C3"]) s.cell(ref).setStyle({ font: { bold: true } });

const rows = [
  ["RAND()", "=RAND()"],
  ["RANDBETWEEN(1,1000)", "=RANDBETWEEN(1,1000)"],
  ["RANDBETWEEN(1,1000)", "=RANDBETWEEN(1,1000)"],
  ["RANDBETWEEN(1,1000)", "=RANDBETWEEN(1,1000)"],
  ["100*RAND()", "=ROUND(100*RAND(),2)"],
  ["NOW()", "=NOW()"],
];

let r = 4;
for (const [label, formula] of rows) {
  s.cell(`A${r}`).setValue(label);
  s.cell(`B${r}`).setValue(formula.slice(1));
  s.cell(`C${r}`).setFormula(formula);
  r++;
}

s.cell(`A${r}`).setValue("SUM(C4:C9)");
s.cell(`C${r}`).setFormula(`=SUM(C4:C${r - 1})`);
s.cell(`A${r}`).setStyle({ font: { bold: true } });
s.cell(`C${r}`).setStyle({ font: { bold: true } });

s.cell("C9").setStyle({ numberFormat: "yyyy-mm-dd hh:mm:ss" });

s.conditionalFormats.set("C5:C7", {
  kind: "colorScale",
  colorScale: {
    values: [
      { kind: "min" },
      { kind: "percentile", value: "50" },
      { kind: "max" },
    ],
    colors: ["FFF8696B", "FFFFEB84", "FF63BE7B"],
  },
});

s.conditionalFormats.set("C8", {
  kind: "cellIs",
  operator: "greaterThanOrEqual",
  formula1: "50",
  dxf: { fill: { background: "FFC6EFCE" }, font: { color: "FF006100" } },
});
s.conditionalFormats.set("C8", {
  kind: "cellIs",
  operator: "lessThan",
  formula1: "50",
  dxf: { fill: { background: "FFFFC7CE" }, font: { color: "FF9C0006" } },
});

s.conditionalFormats.set("C4", {
  kind: "dataBar",
  dataBar: { min: { kind: "number", value: "0" }, max: { kind: "number", value: "1" }, color: "FF638EC6" },
});
s.setColumnWidth(1, 28);
s.setColumnWidth(2, 24);
s.setColumnWidth(3, 22);

wb.recalculate();

const out = new URL("../examples/recalc-demo.xlsx", import.meta.url);
writeFileSync(out, wb.save());
console.log(`wrote ${out.pathname}`);
