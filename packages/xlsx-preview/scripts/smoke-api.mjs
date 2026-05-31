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

const saved = workbook.save();
const reopened = await Workbook.open(saved, { wasmBinaryUrl: wasm });
const c1 = reopened.getCell("Sheet1!C1");
if (c1.value.type !== "number" || c1.value.value !== 30 || c1.formula !== "B1*3") {
  throw new Error("unexpected reopened cell: " + JSON.stringify(c1));
}

console.log(JSON.stringify({ ok: true, sheets: reopened.sheets().length, c1 }));
