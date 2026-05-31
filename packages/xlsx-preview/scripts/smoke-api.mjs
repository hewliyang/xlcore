#!/usr/bin/env node
import { readFileSync } from "node:fs";
import { Workbook } from "../dist/api.js";

const wasm = readFileSync(new URL("../dist/xlcore_wasm_bg.wasm", import.meta.url));
const workbook = await Workbook.create({ wasmBinaryUrl: wasm });

workbook.setValue("Sheet1!A1", "Units");
workbook.setValue("Sheet1!B1", 10);
workbook.setFormula("Sheet1!C1", "=B1*3");

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

workbook.setRowHeight("Sheet1", 2, 33);
workbook.setRowVisible("Sheet1", 3, false);
workbook.setColumnWidth("Sheet1", 2, 24.5);
workbook.setColumnVisible("Sheet1", 4, false);
const freeze = workbook.setFreeze("Sheet1", 1, 2);
if (freeze.frozenRows !== 1 || freeze.frozenColumns !== 2) {
  throw new Error("unexpected setFreeze result: " + JSON.stringify(freeze));
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

console.log(
  JSON.stringify({
    ok: true,
    sheets: reopenedSheets.length,
    c1,
    merges: reopenedMerges.length,
    active: reopenedActive?.name,
  }),
);
