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

const saved = workbook.save();
const reopened = await Workbook.open(saved, { wasmBinaryUrl: wasm });
const c1 = reopened.getCell("Sheet1!C1");
if (c1.value.type !== "number" || c1.value.value !== 30 || c1.formula !== "B1*3") {
  throw new Error("unexpected reopened cell: " + JSON.stringify(c1));
}

console.log(JSON.stringify({ ok: true, sheets: reopened.sheets().length, c1 }));
