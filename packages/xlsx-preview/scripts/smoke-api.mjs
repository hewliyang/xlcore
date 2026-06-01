#!/usr/bin/env node
import { readFileSync } from "node:fs";
import { Workbook } from "../dist/api.js";

const wasm = readFileSync(new URL("../dist/xlcore_wasm_bg.wasm", import.meta.url));
const workbook = await Workbook.create({ wasmBinaryUrl: wasm });

workbook.setValue("Sheet1!A1", "Units");
workbook.setValue("Sheet1!B1", 10);
workbook.setFormula("Sheet1!C1", "=B1*3");

const dependencies = workbook.dependencies("Sheet1!C1");
if (dependencies.precedents[0]?.reference !== "B1" || dependencies.precedents[0]?.sheet !== "Sheet1") {
  throw new Error("unexpected dependencies: " + JSON.stringify(dependencies));
}
const dependents = workbook.dependents("Sheet1!B1");
if (dependents[0]?.reference !== "C1" || dependents[0]?.sheet !== "Sheet1") {
  throw new Error("unexpected dependents: " + JSON.stringify(dependents));
}

const recalc = workbook.recalculate();
const c1Result = recalc.sheets[0]?.cells.find((cell) => cell.r === 1 && cell.c === 3);
if (c1Result?.value.type !== "number" || c1Result.value.value !== 30) {
  throw new Error("unexpected recalc result: " + JSON.stringify(recalc));
}

const layout = workbook.layout();
if (layout.sheets[0]?.maxRow !== 1 || layout.sheets[0]?.maxCol !== 3) {
  throw new Error(
    "unexpected layout extent: " +
      JSON.stringify({
        maxRow: layout.sheets[0]?.maxRow,
        maxCol: layout.sheets[0]?.maxCol,
      }),
  );
}

workbook.setRangeValues("Sheet1!A3:B4", [
  ["North", 10],
  ["South", 20],
]);
workbook.setRangeFormulas("Sheet1!C3:C4", [["=B3*2"], ["=B4*2"]]);

const range = workbook.getRange("Sheet1!A3:C4");
if (
  range.rows !== 2 ||
  range.columns !== 3 ||
  range.reference !== "A3:C4" ||
  range.values[0][0]?.type !== "string" ||
  range.values[0][0].value !== "North" ||
  range.values[1][1]?.type !== "number" ||
  range.values[1][1].value !== 20 ||
  range.formulas[0][2] !== "B3*2"
) {
  throw new Error("unexpected range round-trip: " + JSON.stringify(range));
}

const rangeRecalc = workbook.recalculate();
const c4 = rangeRecalc.sheets[0]?.cells.find((cell) => cell.r === 4 && cell.c === 3);
if (c4?.value.type !== "number" || c4.value.value !== 40) {
  throw new Error("unexpected range recalc: " + JSON.stringify(rangeRecalc));
}

let shapeErr;
try {
  workbook.setRangeValues("Sheet1!A6:B7", [[1, 2]]);
} catch (err) {
  shapeErr = err;
}
if (!shapeErr || shapeErr.code !== "shape_mismatch" || shapeErr.reference !== "A6:B7") {
  throw new Error("expected shape_mismatch ApiError, got: " + String(shapeErr));
}

workbook.addMerge("Sheet1!A10:B11");
workbook.addMerge("Sheet1!C10:D11");
const af = workbook.setAutoFilter("Sheet1!A1:C5");
if (af.reference !== "A1:C5") {
  throw new Error("unexpected setAutoFilter result: " + JSON.stringify(af));
}
if (workbook.autoFilter("Sheet1")?.reference !== "A1:C5") {
  throw new Error("unexpected autoFilter read");
}
workbook.removeAutoFilter("Sheet1");
if (workbook.autoFilter("Sheet1") !== null) {
  throw new Error("expected autoFilter null after remove");
}
workbook.setAutoFilter("Sheet1!A1:C5");
let mergeOverlapErr;
try {
  workbook.addMerge("Sheet1!B11:C12");
} catch (err) {
  mergeOverlapErr = err;
}
if (!mergeOverlapErr || mergeOverlapErr.code !== "merge_overlap") {
  throw new Error("expected merge_overlap ApiError, got: " + String(mergeOverlapErr));
}
const mergesBefore = workbook.merges("Sheet1");
if (mergesBefore.length !== 2 || mergesBefore[0].reference !== "A10:B11") {
  throw new Error("unexpected merges: " + JSON.stringify(mergesBefore));
}
const removed = workbook.removeMerge("Sheet1!A10");
if (!removed || removed.reference !== "A10:B11") {
  throw new Error("unexpected removeMerge result: " + JSON.stringify(removed));
}
if (workbook.removeMerge("Sheet1!Z99") !== null) {
  throw new Error("expected null when removing non-existent merge");
}

workbook.createSheet("Inputs");
workbook.createSheet("Outputs");
const moved = workbook.moveSheet("Outputs", 0);
if (moved.index !== 0 || moved.name !== "Outputs") {
  throw new Error("unexpected moveSheet result: " + JSON.stringify(moved));
}
const hidden = workbook.setSheetVisibility("Inputs", "hidden");
if (hidden.state !== "hidden") {
  throw new Error("unexpected setSheetVisibility result: " + JSON.stringify(hidden));
}
const active = workbook.setActiveSheet("Outputs");
if (!active.active) {
  throw new Error("unexpected setActiveSheet result: " + JSON.stringify(active));
}
let hideActiveErr;
try {
  workbook.setActiveSheet("Inputs");
} catch (err) {
  hideActiveErr = err;
}
if (!hideActiveErr || hideActiveErr.code !== "other") {
  throw new Error("expected activate-hidden ApiError, got: " + String(hideActiveErr));
}

workbook.setValue("Sheet1!E1", 7);
workbook.setFormula("Sheet1!E2", "=E1*5");
workbook.recalculate();
const clearedFormula = workbook.clear("Sheet1!E2", "formulas");
if (clearedFormula.formula !== undefined && clearedFormula.formula !== null) {
  throw new Error("expected formula cleared, got: " + JSON.stringify(clearedFormula));
}
workbook.setFormula("Sheet1!E2", "=E1*5");
workbook.recalculate();
const clearedValue = workbook.clear("Sheet1!E2", "values");
if (clearedValue.value.type !== "blank" || clearedValue.formula !== "E1*5") {
  throw new Error("expected value cleared, formula kept: " + JSON.stringify(clearedValue));
}
const rangeCleared = workbook.clearRange("Sheet1!E1:E2", "all");
if (rangeCleared.values.flat().some((v) => v.type !== "blank")) {
  throw new Error("expected range cleared: " + JSON.stringify(rangeCleared));
}

workbook.setValue("Sheet1!G1", 1);
workbook.setFormula("Sheet1!G2", "=G1+10");
const copied = workbook.copyRange("Sheet1!G2", "Sheet1!G3:G5");
if (copied.formulas[0][0] !== "G2+10" || copied.formulas[2][0] !== "G4+10") {
  throw new Error("unexpected copyRange formulas: " + JSON.stringify(copied));
}
workbook.setValue("Sheet1!H1", 1);
workbook.setFormula("Sheet1!I1", "=H1*2");
const filled = workbook.fillRange("Sheet1!H1:I1", "Sheet1!H1:I3");
if (filled.formulas[2][1] !== "H3*2") {
  throw new Error("unexpected fillRange formulas: " + JSON.stringify(filled));
}

workbook.setRowHeight("Sheet1", 2, 33);
workbook.setRowVisible("Sheet1", 3, false);
workbook.setColumnWidth("Sheet1", 2, 24.5);
workbook.setColumnVisible("Sheet1", 4, false);
const freeze = workbook.setFreeze("Sheet1", 1, 2);
if (freeze.frozenRows !== 1 || freeze.frozenColumns !== 2) {
  throw new Error("unexpected setFreeze result: " + JSON.stringify(freeze));
}

workbook.setDefinedName({ name: "TaxRate", formula: "Sheet1!$B$1" });
workbook.setDefinedName({ name: "LocalRange", formula: "$A$1:$B$5", scope: "Inputs" });

workbook.setValue("Outputs!A1", "Region");
workbook.setValue("Outputs!B1", "Units");
workbook.setValue("Outputs!A2", "North");
workbook.setValue("Outputs!B2", 10);
workbook.setValue("Outputs!A3", "South");
workbook.setValue("Outputs!B3", 20);
const createdTable = workbook.setTable({
  name: "Sales",
  reference: "Outputs!A1:B3",
  style: { name: "TableStyleMedium2", showRowStripes: true },
});
if (createdTable.columns.length !== 2 || createdTable.columns[0].name !== "Region") {
  throw new Error("unexpected setTable result: " + JSON.stringify(createdTable));
}
if (workbook.tables("Outputs").length !== 1) {
  throw new Error("expected one table on Outputs");
}
const names = workbook.definedNames();
if (names.length !== 2 || !names.some((n) => n.name === "LocalRange" && n.scope === "Inputs")) {
  throw new Error("unexpected definedNames: " + JSON.stringify(names));
}

const saved = workbook.save();
const reopened = await Workbook.open(saved, { wasmBinaryUrl: wasm });
const reopenedFreeze = reopened.getFreeze("Sheet1");
if (reopenedFreeze.frozenRows !== 1 || reopenedFreeze.frozenColumns !== 2) {
  throw new Error("unexpected reopened freeze: " + JSON.stringify(reopenedFreeze));
}
const c1 = reopened.getCell("Sheet1!C1");
if (c1.value.type !== "number" || c1.value.value !== 30 || c1.formula !== "B1*3") {
  throw new Error("unexpected reopened cell: " + JSON.stringify(c1));
}

const reopenedMerges = reopened.merges("Sheet1");
if (reopenedMerges.length !== 1 || reopenedMerges[0].reference !== "C10:D11") {
  throw new Error("unexpected reopened merges: " + JSON.stringify(reopenedMerges));
}

const reopenedSheets = reopened.sheets();
const reopenedActive = reopenedSheets.find((sheet) => sheet.active);
if (reopenedActive?.name !== "Outputs" || reopenedSheets[0]?.name !== "Outputs") {
  throw new Error("unexpected reopened sheet order/active: " + JSON.stringify(reopenedSheets));
}
const reopenedHidden = reopenedSheets.find((sheet) => sheet.name === "Inputs");
if (reopenedHidden?.state !== "hidden") {
  throw new Error("unexpected reopened hidden state: " + JSON.stringify(reopenedHidden));
}

const reopenedNames = reopened.definedNames();
if (reopenedNames.length !== 2) {
  throw new Error("unexpected reopened definedNames: " + JSON.stringify(reopenedNames));
}
const removedName = reopened.removeDefinedName("LocalRange", "Inputs");
if (removedName?.name !== "LocalRange") {
  throw new Error("unexpected removeDefinedName: " + JSON.stringify(removedName));
}

workbook.setProperties({ title: "Smoke Plan", creator: "smoke-api", keywords: "smoke,test" });
workbook.setCalcProperties({ calcMode: "manual", iterate: true, iterateCount: 12 });
const reBytesProps = workbook.save();
const reopenedProps = await Workbook.open(reBytesProps, { wasmBinaryUrl: wasm });
const props = reopenedProps.properties();
if (props.title !== "Smoke Plan" || props.creator !== "smoke-api") {
  throw new Error("unexpected properties round-trip: " + JSON.stringify(props));
}
const calc = reopenedProps.calcProperties();
if (calc.calcMode !== "manual" || calc.iterate !== true || calc.iterateCount !== 12) {
  throw new Error("unexpected calc properties round-trip: " + JSON.stringify(calc));
}

workbook.setPageSetup("Sheet1", {
  page: { orientation: "landscape", scale: 80, fitToWidth: 1, fitToHeight: 0 },
  margins: { left: 0.5, right: 0.5, top: 0.75, bottom: 0.75, header: 0.3, footer: 0.3 },
  printOptions: { horizontalCentered: true, gridLines: true },
  headerFooter: { oddHeader: "&CSmoke", oddFooter: "&CPage &P of &N" },
});
const reBytesPage = workbook.save();
const reopenedPage = await Workbook.open(reBytesPage, { wasmBinaryUrl: wasm });
const pageSetup = reopenedPage.pageSetup("Sheet1");
if (
  pageSetup.page?.orientation !== "landscape" ||
  pageSetup.page?.scale !== 80 ||
  pageSetup.printOptions?.horizontalCentered !== true ||
  pageSetup.headerFooter?.oddHeader !== "&CSmoke"
) {
  throw new Error("unexpected page setup round-trip: " + JSON.stringify(pageSetup));
}

console.log(
  JSON.stringify({
    ok: true,
    sheets: reopenedSheets.length,
    c1,
    merges: reopenedMerges.length,
    active: reopenedActive?.name,
    definedNames: reopened.definedNames().length,
    tables: reopened.tables(null).length,
  }),
);
