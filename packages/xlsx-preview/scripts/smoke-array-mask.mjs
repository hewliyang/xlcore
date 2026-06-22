import { readFileSync } from "node:fs";
import { Workbook } from "../dist/node.js";

const wasm = readFileSync(new URL("../dist/xlcore_wasm_bg.wasm", import.meta.url));
const wb = await Workbook.create({ wasmBinaryUrl: wasm });
const s = wb.sheet("Sheet1");

s.range("A1:C1").setValues([["Deal", "Year", "Amount"]]);
s.range("A2:C9").setValues([
  ["Acme", 2024, 12000],
  ["Globex", 2023, 8000],
  ["Initech", 2024, 30000],
  ["Umbrella", 2024, 5000],
  ["Stark", 2022, 99000],
  ["Wayne", 2024, 21000],
  ["Hooli", 2023, 4000],
  ["Wonka", 2024, 18000],
]);

s.cell("E1").setFormula("=SUMPRODUCT((B2:B9=2024)*1)");
s.cell("E2").setFormula("=SUMPRODUCT((B2:B9=2024)*C2:C9)");
s.cell("E3").setFormula("=MEDIAN(FILTER(C2:C9, B2:B9=2024))");
s.cell("E4").setFormula("=PERCENTILE.INC(FILTER(C2:C9, B2:B9=2024), 0.9)");
s.cell("G1").setFormula("=FILTER(A2:C9, B2:B9=2024)");

wb.recalculate();

const num = (a1) => s.cell(a1).info().value.value;
const txt = (a1) => {
  const v = s.cell(a1).info().value;
  return v.type === "empty" ? "" : v.value;
};

console.log("2024 deal count       (E1):", num("E1"), "  expect 5");
console.log("2024 revenue          (E2):", num("E2"), "  expect 86000");
console.log("2024 median deal      (E3):", num("E3"), "  expect 18000");
console.log("2024 p90 deal         (E4):", num("E4"), "  expect 26400");

console.log("\nFILTER spill (2024 deals only):");
for (let r = 1; r <= 5; r++) {
  console.log(" ", txt(`G${r}`), txt(`H${r}`), txt(`I${r}`));
}

const assert = (cond, msg) => {
  if (!cond) throw new Error("FAIL: " + msg);
};
assert(num("E1") === 5, "count");
assert(num("E2") === 86000, "revenue");
assert(num("E3") === 18000, "median");
assert(Math.abs(num("E4") - 26400) < 1e-6, "p90");
assert(txt("G1") === "Acme" && txt("G3") === "Umbrella", "filter spill");
console.log("\nALL ASSERTIONS PASSED ✓");
wb.dispose();
