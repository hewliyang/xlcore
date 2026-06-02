#!/usr/bin/env node
import { readFileSync } from "node:fs";
import { NumberFormat, Workbook } from "../dist/api.js";

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

const baselineStyleIndex = s1.cell("Z99").info().styleIndex;
s1.setStyles({
  A2: { font: { italic: true } },
  "B2:C2": { font: { color: "#FF0000" } },
});
const styledA2 = s1.cell("A2").info();
const styledB2 = s1.cell("B2").info();
const styledC2 = s1.cell("C2").info();
if (styledA2.styleIndex === undefined || styledA2.styleIndex === baselineStyleIndex) {
  throw new Error("setStyles did not apply style to A2: " + JSON.stringify(styledA2));
}
if (
  styledB2.styleIndex === undefined ||
  styledB2.styleIndex === baselineStyleIndex ||
  styledB2.styleIndex !== styledC2.styleIndex
) {
  throw new Error("setStyles did not apply style to B2:C2: " + JSON.stringify({ styledB2, styledC2 }));
}
if (styledA2.styleIndex === styledB2.styleIndex) {
  throw new Error("setStyles unexpectedly merged distinct patches: " + JSON.stringify({ styledA2, styledB2 }));
}
let setStylesErr;
try {
  s1.setStyles({ "": { font: { bold: true } } });
} catch (err) {
  setStylesErr = err?.message ?? String(err);
}
if (!setStylesErr?.includes("non-empty string")) {
  throw new Error("setStyles did not reject empty ref: " + setStylesErr);
}

if (NumberFormat.Percent2 !== "0.00%" || NumberFormat.Scientific2 !== "0.00E+00") {
  throw new Error("NumberFormat enum drift: " + JSON.stringify(NumberFormat));
}
const nfBaseline = s1.cell("D2").info().styleIndex;
s1.cell("D2").setValue(0.125);
s1.cell("D2").setStyle({ numberFormat: NumberFormat.Percent2 });
const nfStyled = s1.cell("D2").info().styleIndex;
if (nfStyled === undefined || nfStyled === nfBaseline) {
  throw new Error("NumberFormat.Percent2 did not apply: " + JSON.stringify({ nfBaseline, nfStyled }));
}
s1.cell("D3").setValue(1234567);
s1.cell("D3").setStyle({ numberFormat: NumberFormat.Scientific2 });

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

wb.definedNames.set({ name: "TaxRate", reference: "Sheet1!$B$1" });

const inputs = wb.addSheet("Inputs");
const outputs = wb.addSheet("Outputs");
wb.definedNames.set({ name: "LocalRange", formula: "$A$1:$B$5", scope: "Inputs" });
const _aliasCheck = wb.definedNames.list().find((d) => d.name === "LocalRange");
if (!_aliasCheck || _aliasCheck.reference !== "$A$1:$B$5") {
  throw new Error("definedNames.set should accept legacy `formula` alias for `reference`");
}

outputs
  .range("A1:B3")
  .setValues([
    ["Region", "Units"],
    ["North", 10],
    ["South", 20],
  ]);

const table = outputs.tables.set({
  name: "Sales",
  reference: "A1:B3",
  style: { name: "TableStyleMedium2", showRowStripes: true },
});
if (table.sheet !== "Outputs") {
  throw new Error("unqualified ref did not resolve to scoped sheet: " + JSON.stringify(table));
}
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

outputs.autoFilter.set("A1:B3");
const afValues = outputs.autoFilter.setColumnValues(0, ["North"], { blank: false });
if (afValues.criteria.kind !== "values" || afValues.criteria.values[0] !== "North") {
  throw new Error("setColumnValues round-trip: " + JSON.stringify(afValues));
}
const afTop = outputs.autoFilter.setColumnTop10(1, 5, { percent: true });
if (afTop.criteria.kind !== "top10" || afTop.criteria.val !== 5 || afTop.criteria.percent !== true) {
  throw new Error("setColumnTop10 round-trip: " + JSON.stringify(afTop));
}
const afCustom = outputs.autoFilter.setColumnCustom(
  1,
  [{ operator: "greaterThan", value: "5" }],
  { logicalAnd: true },
);
if (afCustom.criteria.kind !== "custom" || afCustom.criteria.criteria[0].value !== "5") {
  throw new Error("setColumnCustom round-trip: " + JSON.stringify(afCustom));
}
let badKindErr = null;
try {
  outputs.autoFilter.setColumn({ columnOffset: 0, criteria: { values: ["x"] } });
} catch (e) {
  badKindErr = e;
}
if (!badKindErr || !String(badKindErr.message).includes("patch.criteria.kind")) {
  throw new Error("missing-kind error not clearer: " + badKindErr);
}
outputs.autoFilter.remove();

const chart = outputs.charts.set({
  sheet: "Outputs",
  name: "Sales Chart",
  kind: "column",
  title: "Sales",
  categoriesRef: "Outputs!$A$2:$A$3",
  series: [
    { name: "Units", valuesRef: "Outputs!$B$2:$B$3", color: "4472C4" },
  ],
  anchor: { fromColumn: 3, fromRow: 0, toColumn: 8, toRow: 12 },
});
if (chart.title !== "Sales") throw new Error("unexpected chart: " + JSON.stringify(chart));
const updated = outputs.charts.update(chart.id, {
  title: "Sales (Updated)",
  legendPosition: "bottom",
});
if (updated.title !== "Sales (Updated)" || updated.legendPosition !== "bottom") {
  throw new Error("charts.update did not merge: " + JSON.stringify(updated));
}
if (updated.series.length !== 1 || updated.series[0].valuesRef !== "Outputs!$B$2:$B$3") {
  throw new Error("charts.update lost series: " + JSON.stringify(updated));
}
if (updated.kind !== "column" || updated.name !== "Sales Chart") {
  throw new Error("charts.update lost kind/name: " + JSON.stringify(updated));
}
let missingChartErr = null;
try {
  outputs.charts.update("nope-rid", { title: "x" });
} catch (e) {
  missingChartErr = e;
}
if (!missingChartErr || !String(missingChartErr.message).includes("chart not found")) {
  throw new Error("charts.update did not error on missing id: " + missingChartErr);
}
if (outputs.charts.list().length !== 1) {
  throw new Error("charts.update should not duplicate: " + outputs.charts.list().length);
}

const pivotSrc = wb.addSheet("PivotSrc");
pivotSrc.range("A1:C7").setValues([
  ["Region", "Product", "Amount"],
  ["North", "Widget", 100],
  ["North", "Gadget", 50],
  ["South", "Widget", 75],
  ["South", "Gadget", 25],
  ["North", "Widget", 30],
  ["South", "Gadget", 60],
]);
const pivotSheet = wb.addSheet("PivotOut");
const pivot = pivotSheet.pivots.set({
  anchorCell: "PivotOut!A1",
  sourceRef: "PivotSrc!A1:C7",
  name: "SmokePivot",
  rowFields: ["Region"],
  columnFields: ["Product"],
  dataFields: [{ field: "Amount", aggregation: "sum", numberFormat: "$#,##0.00" }],
});
if (pivot.name !== "SmokePivot" || pivot.rowFields[0] !== "Region") {
  throw new Error("unexpected pivot: " + JSON.stringify(pivot));
}
if (pivotSheet.pivots.list().length !== 1) {
  throw new Error("expected one pivot on PivotOut");
}
if (wb.allPivots.list().length !== 1) {
  throw new Error("expected one workbook pivot");
}

const preview = pivotSheet.pivots.preview({
  anchorCell: "PivotOut!H1",
  sourceRef: "PivotSrc!A1:C7",
  rowFields: ["Region"],
  dataFields: [{ field: "Amount", aggregation: "sum", name: "Sum of Amount" }],
});
const previewAt = (row, col) => preview.cells.find((c) => c.row === row && c.col === col);
if (preview.rows !== 4 || preview.cols !== 2) {
  throw new Error("unexpected preview dims: " + JSON.stringify(preview));
}
if (previewAt(1, 1)?.value !== "180" || previewAt(3, 1)?.value !== "340") {
  throw new Error("unexpected preview values: " + JSON.stringify(preview.cells));
}
if (previewAt(0, 0)?.role !== "header" || previewAt(3, 0)?.role !== "total_label") {
  throw new Error("unexpected preview roles: " + JSON.stringify(preview.cells));
}
if (pivotSheet.pivots.list().length !== 1) {
  throw new Error("preview should not author a pivot");
}

const pivotUpdated = pivotSheet.pivots.update(pivot.id, {
  dataFields: [{ field: "Amount", aggregation: "count", numberFormat: "$#,##0.00" }],
});
if (pivotUpdated.dataFields[0].aggregation !== "count") {
  throw new Error("pivot update did not change aggregation: " + JSON.stringify(pivotUpdated));
}
if (pivotSheet.pivots.list().length !== 1) {
  throw new Error("pivot update should not duplicate the pivot");
}
if (pivotUpdated.anchorCell !== "A1") {
  throw new Error("pivot update moved the anchor: " + JSON.stringify(pivotUpdated));
}

const noteAdded = s1.threadedNotes.add("A1", { text: "hello", author: "smoke" });
if (!noteAdded.id) throw new Error("threadedNotes.add returned no id");
s1.cell("A2").setValue("post-note");

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
const roPivots = reopened.sheet("PivotOut").pivots.list();
if (roPivots.length !== 1 || roPivots[0].name !== "SmokePivot") {
  throw new Error("reopened pivot missing: " + JSON.stringify(roPivots));
}
if (roPivots[0].dataFields[0].field !== "Amount") {
  throw new Error("reopened pivot data field lost: " + JSON.stringify(roPivots[0]));
}
if (roPivots[0].dataFields[0].numberFormat !== "$#,##0.00") {
  throw new Error("reopened pivot numberFormat lost: " + JSON.stringify(roPivots[0]));
}

{
  const defaultWb = await Workbook.create();
  const ds = defaultWb.sheet("Sheet1");
  ds.cell("A1").setValue(7);
  ds.cell("B1").setFormula("=A1*6");
  defaultWb.recalculate();
  const dv = ds.cell("B1").info().value;
  if (dv.type !== "number" || dv.value !== 42) {
    throw new Error("default-wasm Node bootstrap failed: " + JSON.stringify(dv));
  }
  defaultWb.dispose();
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
