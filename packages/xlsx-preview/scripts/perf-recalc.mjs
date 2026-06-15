import { readFileSync } from "node:fs";
import { performance } from "node:perf_hooks";
import { Workbook } from "../dist/node.js";

const wasm = readFileSync(new URL("../dist/xlcore_wasm_bg.wasm", import.meta.url));

const ROWS = Number(process.argv[2] ?? 100_000);
const SHEET = "Sheet1";

const wb = await Workbook.create({ wasmBinaryUrl: wasm });
const s1 = wb.sheet(SHEET);

console.log(`building fixture: ${ROWS} formulas...`);
let t = performance.now();

const seeds = [];
const formulas = [];
for (let i = 0; i < ROWS; i++) {
  seeds.push([i + 1]);
  const r = i + 1;
  formulas.push([r === 1 ? "=A1*2" : `=A${r}+C${r - 1}`]);
}

s1.range(`A1:A${ROWS}`).setValues(seeds);
s1.range(`C1:C${ROWS}`).setFormulas(formulas);

console.log(`  build: ${(performance.now() - t).toFixed(0)}ms`);

const runs = Number(process.env.RUNS ?? 3);
for (let i = 0; i < runs; i++) {
  t = performance.now();
  const report = wb.recalculate({ errorsOnly: true });
  const ms = performance.now() - t;
  const errSheets = report.sheets?.length ?? 0;
  console.log(
    `  recalc #${i + 1}: ${ms.toFixed(0)}ms  (${(ROWS / (ms / 1000)).toFixed(0)} formulas/s, errorSheets=${errSheets})`,
  );
}

const last = s1.cell(`C${ROWS}`).info();
console.log(`  C${ROWS} = ${JSON.stringify(last.value)}`);

wb.dispose();
