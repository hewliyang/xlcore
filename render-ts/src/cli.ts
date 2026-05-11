#!/usr/bin/env node

import { mkdir, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { renderXlsxToPng } from "./node.js";

interface CliOptions {
  input?: string;
  output?: string;
  range?: string;
  sheet?: string;
  scale: number;
}

function usage(): string {
  return [
    "Usage:",
    "  xlcore-render --input workbook.xlsx --output sheet.png [--range A1:H20] [--sheet Sheet1] [--scale 2]",
    "",
    "Examples:",
    "  xlcore-render -i model.xlsx -o cover.png --range \"'Cover'!B3:E12\" --scale 2",
    "  xlcore-render -i model.xlsx -o sheet.png --sheet Cover --range B3:E12",
  ].join("\n");
}

function parseArgs(argv: string[]): CliOptions {
  const options: CliOptions = { scale: 1 };
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    if (arg === "--help" || arg === "-h") {
      console.log(usage());
      process.exit(0);
    }
    const value = argv[i + 1];
    if (!value || value.startsWith("-")) throw new Error(`Missing value for ${arg}`);
    if (arg === "--input" || arg === "-i") options.input = value;
    else if (arg === "--output" || arg === "-o") options.output = value;
    else if (arg === "--range" || arg === "-r") options.range = value;
    else if (arg === "--sheet" || arg === "-s") options.sheet = value;
    else if (arg === "--scale") options.scale = Number(value);
    else throw new Error(`Unknown argument: ${arg}`);
    i++;
  }
  if (!options.input) throw new Error("Missing --input");
  if (!options.output) throw new Error("Missing --output");
  if (!Number.isFinite(options.scale) || options.scale <= 0)
    throw new Error(`Invalid --scale: ${options.scale}`);
  return options;
}

async function main(): Promise<void> {
  const options = parseArgs(process.argv.slice(2));
  const input = resolve(options.input!);
  const output = resolve(options.output!);
  const png = await renderXlsxToPng(input, {
    range: options.range,
    sheetName: options.sheet,
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
      scale: options.scale,
      bytes: png.length,
    }),
  );
}

main().catch((error) => {
  console.error(error?.message || String(error));
  console.error("");
  console.error(usage());
  process.exit(1);
});
