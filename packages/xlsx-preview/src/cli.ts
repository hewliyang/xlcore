#!/usr/bin/env node

import { mkdir, writeFile } from "node:fs/promises";
import { dirname, extname, join, resolve } from "node:path";
import { type LoadReport, XlsxLoadError, reportIsClean } from "./errors.js";
import { loadWorkbookFromXlsxWithReport, renderToPng } from "./node.js";
import type { WorkbookLayout } from "./types.js";

interface CliOptions {
  input?: string;
  output?: string;
  range?: string;
  sheet?: string;
  sheetIndex?: number;
  scale: number;
  info: boolean;
  all: boolean;
  verbose: boolean;
  strict: boolean;
}

function usage(): string {
  return [
    "Usage:",
    "  xlsx-preview workbook.xlsx --output sheet.png [--range A1:H20] [--sheet Sheet1] [--scale 2]",
    "  xlsx-preview workbook.xlsx --info",
    "  xlsx-preview workbook.xlsx --all --output previews/",
    "",
    "Options:",
    "  --info             Print workbook sheets and used ranges as JSON; no --output needed",
    "  --all              Render every sheet. --output may be a directory or include {sheet}/{index}",
    "  --sheet-index N    Render/select sheet by zero-based index",
    "  --verbose / -v     Print non-fatal load warnings (fixed attributes) to stderr",
    "  --strict           Exit non-zero if the loader had to coerce any invalid attributes",
    "                     (use to catch regressions in CI)",
    "",
    "Examples:",
    "  xlsx-preview model.xlsx --info",
    "  xlsx-preview model.xlsx -o cover.png --range \"'Cover'!B3:E12\" --scale 2",
    "  xlsx-preview model.xlsx -o previews/{index}-{sheet}.png --all",
    "  xlsx-preview model.xlsx -o sheet.png --sheet Cover --range B3:E12",
  ].join("\n");
}

function parseArgs(argv: string[]): CliOptions {
  const options: CliOptions = { scale: 1, info: false, all: false, verbose: false, strict: false };
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i]!;
    if (arg === "--help" || arg === "-h") {
      console.log(usage());
      process.exit(0);
    }
    if (arg === "--info" || arg === "--list") {
      options.info = true;
      continue;
    }
    if (arg === "--all") {
      options.all = true;
      continue;
    }
    if (arg === "--verbose" || arg === "-v") {
      options.verbose = true;
      continue;
    }
    if (arg === "--strict") {
      options.strict = true;
      continue;
    }
    if (!arg.startsWith("-")) {
      if (options.input) throw new Error(`Unexpected positional argument: ${arg}`);
      options.input = arg;
      continue;
    }
    const value = argv[i + 1];
    if (!value || value.startsWith("-")) throw new Error(`Missing value for ${arg}`);
    if (arg === "--input" || arg === "-i") {
      if (options.input) throw new Error("Input specified more than once");
      options.input = value;
    } else if (arg === "--output" || arg === "-o") options.output = value;
    else if (arg === "--range" || arg === "-r") options.range = value;
    else if (arg === "--sheet" || arg === "-s") options.sheet = value;
    else if (arg === "--sheet-index") options.sheetIndex = Number(value);
    else if (arg === "--scale") options.scale = Number(value);
    else throw new Error(`Unknown argument: ${arg}`);
    i++;
  }
  if (!options.input) throw new Error("Missing --input");
  if ((!options.info || options.all) && !options.output) throw new Error("Missing --output");
  if (options.all && (options.sheet || options.sheetIndex !== undefined))
    throw new Error("--all cannot be combined with --sheet or --sheet-index");
  if (!Number.isFinite(options.scale) || options.scale <= 0)
    throw new Error(`Invalid --scale: ${options.scale}`);
  if (
    options.sheetIndex !== undefined &&
    (!Number.isInteger(options.sheetIndex) || options.sheetIndex < 0)
  )
    throw new Error(`Invalid --sheet-index: ${options.sheetIndex}`);
  return options;
}

async function main(): Promise<void> {
  const options = parseArgs(process.argv.slice(2));
  const input = resolve(options.input!);

  if (options.info || options.all) {
    const { layout, report } = await loadWorkbookFromXlsxWithReport(input);
    reportToStderr(report, options);
    enforceStrict(report, options);

    if (options.info) {
      console.log(JSON.stringify(workbookInfo(input, layout, report), null, 2));
      if (!options.all) return;
    }

    const rendered = [];
    for (let i = 0; i < layout.sheets.length; i++) {
      const state = layout.sheets[i]?.state;
      if (state === "hidden" || state === "veryHidden") continue;
      const output = outputForSheet(
        resolve(options.output!),
        layout.sheets[i]?.name ?? `Sheet${i + 1}`,
        i,
      );
      const png = await renderToPng(layout, {
        range: options.range,
        sheetIndex: i,
        scale: options.scale,
      });
      await mkdir(dirname(output), { recursive: true });
      await writeFile(output, png);
      rendered.push({ sheetIndex: i, sheet: layout.sheets[i]?.name, output, bytes: png.length });
    }
    console.log(JSON.stringify({ input, scale: options.scale, rendered }, null, 2));
    return;
  }

  const output = resolve(options.output!);

  const { layout, report } = await loadWorkbookFromXlsxWithReport(input, {
    sheetIndex: options.sheetIndex,
    sheetName: options.sheet,
  });
  reportToStderr(report, options);
  enforceStrict(report, options);
  const png = await renderToPng(layout, {
    range: options.range,
    sheetName: options.sheet,
    sheetIndex: options.sheetIndex,
    scale: options.scale,
  });

  await mkdir(dirname(output), { recursive: true });
  await writeFile(output, png);
  console.log(
    JSON.stringify({
      input,
      output,
      range: options.range,
      sheet: options.sheet,
      sheetIndex: options.sheetIndex,
      scale: options.scale,
      bytes: png.length,
    }),
  );
}

function workbookInfo(input: string, layout: WorkbookLayout, report: LoadReport) {
  return {
    input,
    loadReport: reportIsClean(report) ? null : report,
    activeSheetIndex: layout.activeSheetIndex ?? 0,
    sheets: layout.sheets.map((sheet, index) => ({
      index,
      name: sheet.name,
      maxRow: sheet.maxRow,
      maxCol: sheet.maxCol,
      usedRange:
        sheet.maxRow > 0 && sheet.maxCol > 0 ? `A1:${colName(sheet.maxCol)}${sheet.maxRow}` : null,
      cells: sheet.cells?.count ?? 0,
      tables: sheet.tables?.length ?? 0,
      drawings: sheet.drawings?.length ?? 0,
      comments: sheet.comments?.length ?? 0,
    })),
  };
}

function outputForSheet(pattern: string, sheetName: string, index: number): string {
  const safeSheet = sheetName.replace(/[\\/:*?"<>|]/g, "_");
  if (pattern.includes("{sheet}") || pattern.includes("{index}")) {
    return pattern.replaceAll("{sheet}", safeSheet).replaceAll("{index}", String(index));
  }
  if (!extname(pattern) || pattern.endsWith("/") || pattern.endsWith("\\")) {
    return join(pattern, `${index}-${safeSheet}.png`);
  }
  return pattern.replace(/(\.[^.\\/]*)?$/, `-${index}-${safeSheet}$1`);
}

function colName(col: number): string {
  let n = col;
  let out = "";
  while (n > 0) {
    n--;
    out = String.fromCharCode(65 + (n % 26)) + out;
    n = Math.floor(n / 26);
  }
  return out;
}

function reportToStderr(report: LoadReport, options: CliOptions): void {
  if (!options.verbose || reportIsClean(report)) return;
  const lines: string[] = ["xlsx-preview: load report"];
  for (const fix of report.fixes) {
    const where = fix.part && fix.part !== "*" ? fix.part : "package";
    lines.push(
      `  fixed ${fix.field ?? "?"}=${JSON.stringify(fix.value ?? "")} ×${fix.occurrences} in ${where} (${fix.kind})`,
    );
  }
  for (const w of report.warnings) {
    lines.push(`  warning: ${w}`);
  }
  console.error(lines.join("\n"));
}

function enforceStrict(report: LoadReport, options: CliOptions): void {
  if (!options.strict || reportIsClean(report)) return;
  const fixes = report.fixes.reduce((sum, fix) => sum + fix.occurrences, 0);
  const warnings = report.warnings.length;
  const parts = [
    fixes > 0 ? `${fixes} repair(s)` : null,
    warnings > 0 ? `${warnings} warning(s)` : null,
  ].filter(Boolean);
  console.error(`xlsx-preview: --strict failed: load produced ${parts.join(", ")}`);
  process.exit(2);
}

main().catch((error) => {
  if (XlsxLoadError.isXlsxLoadError(error)) {
    console.error(`xlsx-preview: ${error.message}`);
    console.error(error.diagnosticsText());
  } else {
    console.error(error?.message || String(error));
  }
  console.error("");
  console.error(usage());
  process.exit(1);
});
