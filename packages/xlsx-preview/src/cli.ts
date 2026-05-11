#!/usr/bin/env node

import { mkdir, writeFile } from "node:fs/promises";
import { dirname, extname, join, resolve } from "node:path";
import { loadWorkbookFromXlsx, renderToPng, renderXlsxToPng } from "./node.js";

interface CliOptions {
  input?: string;
  output?: string;
  range?: string;
  sheet?: string;
  sheetIndex?: number;
  scale: number;
  info: boolean;
  all: boolean;
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
    "",
    "Examples:",
    "  xlsx-preview model.xlsx --info",
    "  xlsx-preview model.xlsx -o cover.png --range \"'Cover'!B3:E12\" --scale 2",
    "  xlsx-preview model.xlsx -o previews/{index}-{sheet}.png --all",
    "  xlsx-preview model.xlsx -o sheet.png --sheet Cover --range B3:E12",
  ].join("\n");
}

function parseArgs(argv: string[]): CliOptions {
  const options: CliOptions = { scale: 1, info: false, all: false };
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
  if (!options.info && !options.output) throw new Error("Missing --output");
  if (options.all && (options.sheet || options.sheetIndex !== undefined))
    throw new Error("--all cannot be combined with --sheet or --sheet-index");
  if (!Number.isFinite(options.scale) || options.scale <= 0)
    throw new Error(`Invalid --scale: ${options.scale}`);
  if (options.sheetIndex !== undefined && (!Number.isInteger(options.sheetIndex) || options.sheetIndex < 0))
    throw new Error(`Invalid --sheet-index: ${options.sheetIndex}`);
  return options;
}

async function main(): Promise<void> {
  const options = parseArgs(process.argv.slice(2));
  const input = resolve(options.input!);

  if (options.info) {
    const layout = await loadWorkbookFromXlsx(input);
    console.log(JSON.stringify(workbookInfo(input, layout), null, 2));
    if (!options.all) return;
  }

  if (options.all) {
    const layout = await loadWorkbookFromXlsx(input);
    const rendered = [];
    for (let i = 0; i < layout.sheets.length; i++) {
      const output = outputForSheet(resolve(options.output!), layout.sheets[i]?.name ?? `Sheet${i + 1}`, i);
      const png = await renderToPng(layout, { range: options.range, sheetIndex: i, scale: options.scale });
      await mkdir(dirname(output), { recursive: true });
      await writeFile(output, png);
      rendered.push({ sheetIndex: i, sheet: layout.sheets[i]?.name, output, bytes: png.length });
    }
    console.log(JSON.stringify({ input, scale: options.scale, rendered }, null, 2));
    return;
  }

  const output = resolve(options.output!);
  const png = await renderXlsxToPng(input, {
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

function workbookInfo(input: string, layout: Awaited<ReturnType<typeof loadWorkbookFromXlsx>>) {
  return {
    input,
    activeSheetIndex: layout.activeSheetIndex ?? 0,
    sheets: layout.sheets.map((sheet, index) => ({
      index,
      name: sheet.name,
      maxRow: sheet.maxRow,
      maxCol: sheet.maxCol,
      usedRange: sheet.maxRow > 0 && sheet.maxCol > 0 ? `A1:${colName(sheet.maxCol)}${sheet.maxRow}` : null,
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

main().catch((error) => {
  console.error(error?.message || String(error));
  console.error("");
  console.error(usage());
  process.exit(1);
});
