#!/usr/bin/env node
import { readFileSync } from "node:fs";
import { Workbook } from "../dist/api.js";

const wasm = readFileSync(new URL("../dist/xlcore_wasm_bg.wasm", import.meta.url));
const wb = await Workbook.create({ wasmBinaryUrl: wasm });

const s1 = wb.sheet("Sheet1");

s1.cell("A1").setValue("Units");
s1.cell({ row: 1, column: 2 }).setValue(10);
s1.cell("C1").setFormula("=B1*3");

const c1 = s1.cell("C1");
const deps = c1.dependencies();
if (deps.precedents[0]?.reference !== "B1" || deps.precedents[0]?.sheet !== "Sheet1") {
  throw new Error("unexpected precedents: " + JSON.stringify(deps));
}
const dependents = s1.cell("B1").dependents();
if (dependents[0]?.reference !== "C1") {
  throw new Error("unexpected dependents: " + JSON.stringify(dependents));
}

wb.recalculate();
const c1Info = c1.info();
if (c1Info.value.type !== "number" || c1Info.value.value !== 30) {
  throw new Error("unexpected c1 value: " + JSON.stringify(c1Info));
}

s1.range({ row: 3, column: 1, rowCount: 2, columnCount: 2 }).setValues([
  ["North", 10],
  ["South", 20],
]);
s1.range("C3:C4").setFormulas([["=B3*2"], ["=B4*2"]]);

const rangeInfo = s1.range("A3:C4").info();
if (
  rangeInfo.rows !== 2 ||
  rangeInfo.columns !== 3 ||
  rangeInfo.reference !== "A3:C4" ||
  rangeInfo.values[0][0]?.type !== "string" ||
  rangeInfo.values[0][0].value !== "North" ||
  rangeInfo.formulas[0][2] !== "B3*2"
) {
  throw new Error("unexpected range info: " + JSON.stringify(rangeInfo));
}

s1.range("A1:C1").setStyle({ font: { bold: true } });

s1.range("E1").setValues([[5]]);
s1.cell("E2").setFormula("=E1*5");
wb.recalculate();
s1.cell("E2").clear("formulas");
const e2 = s1.cell("E2").info();
if (e2.formula !== undefined) {
  throw new Error("clear formulas did not drop formula: " + JSON.stringify(e2));
}

s1.range("H1").setValues([[1]]);
s1.cell("I1").setFormula("=H1*2");
const filled = s1.range("H1:I1").fillTo("H1:I3");
if (filled.info().formulas[2][1] !== "H3*2") {
  throw new Error("unexpected fillRange: " + JSON.stringify(filled.info()));
}

s1.range("A10:B11").merge();
const merges = s1.merges.list();
if (merges.length !== 1 || merges[0].reference !== "A10:B11") {
  throw new Error("unexpected merges: " + JSON.stringify(merges));
}
s1.merges.remove("A10");
if (s1.merges.list().length !== 0) throw new Error("merges not cleared");

s1.setRowHeight(2, 33);
s1.setColumnWidth(2, 24.5);
const freeze = s1.freeze.set(1, 2);
if (freeze.frozenRows !== 1 || freeze.frozenColumns !== 2) {
  throw new Error("unexpected freeze: " + JSON.stringify(freeze));
}

wb.definedNames.set({ name: "TaxRate", formula: "Sheet1!$B$1" });

const inputs = wb.addSheet("Inputs");
const outputs = wb.addSheet("Outputs");
wb.definedNames.set({ name: "LocalRange", formula: "$A$1:$B$5", scope: "Inputs" });

outputs
  .range("A1:B3")
  .setValues([
    ["Region", "Units"],
    ["North", 10],
    ["South", 20],
  ]);

const table = outputs.tables.set({
  name: "Sales",
  reference: "Outputs!A1:B3",
  style: { name: "TableStyleMedium2", showRowStripes: true },
});
if (table.columns.length !== 2 || table.columns[0].name !== "Region") {
  throw new Error("unexpected table: " + JSON.stringify(table));
}
if (outputs.tables.list().length !== 1) throw new Error("expected one table on Outputs");
if (wb.allTables.list().length !== 1) throw new Error("expected one workbook table");

const names = wb.definedNames.list();
if (names.length !== 2 || !names.some((n) => n.name === "LocalRange" && n.scope === "Inputs")) {
  throw new Error("unexpected definedNames: " + JSON.stringify(names));
}

outputs.activate();
const active = wb.activeSheet();
if (active.name !== "Outputs") throw new Error("activeSheet not Outputs: " + active.name);

inputs.setVisibility("hidden");

outputs.moveTo(0);
const worksheets = wb.worksheets();
if (worksheets[0].name !== "Outputs") throw new Error("moveTo did not work");

const extra = wb.addSheet("Scratch");
extra.cell({ row: 1, column: 1 }).setValue("x");
extra.rename("Junk");
if (extra.name !== "Junk") throw new Error("rename did not update name");
if (extra.cell("A1").value().type !== "string") {
  throw new Error("rename broke cell access via the same Worksheet handle");
}
extra.remove();
if (wb.worksheets().some((w) => w.name === "Junk")) {
  throw new Error("Worksheet.remove did not delete the sheet");
}

wb.properties.set({ title: "Smoke Plan", creator: "smoke-api" });
wb.calcProperties.set({ calcMode: "manual", iterate: true, iterateCount: 12 });

const saved = wb.save();
const reopened = await Workbook.open(saved, { wasmBinaryUrl: wasm });
const ro = reopened.sheet("Sheet1");
const roFreeze = ro.freeze.get();
if (roFreeze.frozenRows !== 1 || roFreeze.frozenColumns !== 2) {
  throw new Error("unexpected reopened freeze: " + JSON.stringify(roFreeze));
}
const roC1 = ro.cell("C1").info();
if (roC1.value.type !== "number" || roC1.value.value !== 30 || roC1.formula !== "B1*3") {
  throw new Error("unexpected reopened C1: " + JSON.stringify(roC1));
}
const roActive = reopened.activeSheet();
if (roActive.name !== "Outputs") throw new Error("reopened active not Outputs");
if (reopened.sheet("Inputs").visibility !== "hidden") {
  throw new Error("reopened Inputs visibility not hidden");
}
if (reopened.properties.get().title !== "Smoke Plan") {
  throw new Error("reopened properties.title not Smoke Plan");
}
if (reopened.calcProperties.get().calcMode !== "manual") {
  throw new Error("reopened calcProperties.calcMode not manual");
}

console.log(
  JSON.stringify({
    ok: true,
    sheets: reopened.worksheets().map((w) => w.name),
    c1: roC1,
    active: roActive.name,
    definedNames: reopened.definedNames.list().length,
    tables: reopened.allTables.list().length,
  }),
);
