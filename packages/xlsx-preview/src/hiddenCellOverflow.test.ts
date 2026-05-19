import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { expect, test } from "vitest";

import { loadWorkbookFromXlsx, renderToCanvas } from "../dist/index.js";

const FIXTURE = resolve(
  fileURLToPath(import.meta.url),
  "../../../../tests/fixtures/outline/outline-groups.xlsx",
);

const FORBIDDEN_LITERALS = new Set([
  "Q1",
  "Q2",
  "Q3",

  "100",
  "110",
  "120",
  "40",
  "45",
  "50",
  "60",
  "65",
  "70",
  "200",
  "220",
  "240",

  "80",
  "85",
  "90",
  "160",
  "170",
  "180",
]);

const REQUIRED_LITERALS = ["Region/Quarter", "Total", "Notes", "Region A total"];

async function loadCollapsedAndRecordFillText() {
  const bytes = await readFile(FIXTURE);
  const layout = await loadWorkbookFromXlsx(bytes);
  const sheet = layout.sheets[0]!;

  const colOverrides = new Map<number, number>();
  for (const c of sheet.cols) {
    if ((c.outlineLevel ?? 0) >= 1) {
      for (let i = c.min; i <= c.max; i++) colOverrides.set(i, 0);
    }
  }

  expect(colOverrides.size).toBeGreaterThan(0);

  const canvas = renderToCanvas(layout, { colOverrides });

  const fillTextCalls: string[] = [];

  type Ctx2D = {
    fillText(text: string, x: number, y: number, maxWidth?: number): void;
  };
  const ctx = canvas.getContext("2d") as unknown as Ctx2D;
  const proto = Object.getPrototypeOf(ctx) as Ctx2D;
  const originalFillText = proto.fillText;
  proto.fillText = function (this: Ctx2D, text, x, y, maxWidth) {
    fillTextCalls.push(text);
    return originalFillText.call(this, text, x, y, maxWidth);
  };
  try {
    renderToCanvas(layout, { colOverrides });
  } finally {
    proto.fillText = originalFillText;
  }
  return fillTextCalls;
}

test("collapsed outline columns do not paint their cell text into visible neighbors", async () => {
  const calls = await loadCollapsedAndRecordFillText();

  const leaked = calls.filter((t) => FORBIDDEN_LITERALS.has(t.trim()));
  expect(
    leaked,
    `Hidden outline-column cells leaked text into the render: ${JSON.stringify(leaked)}`,
  ).toEqual([]);

  for (const needed of REQUIRED_LITERALS) {
    expect(
      calls.some((t) => t.includes(needed)),
      `Expected visible cell text "${needed}" to still render, but it was missing`,
    ).toBe(true);
  }
});
