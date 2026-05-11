// Regression test for "blue numbers bleeding through collapsed outline
// columns". Symptom: when an outline group was collapsed (its columns
// forced to width 0 via colOverrides) the renderer was still painting
// the hidden cells' text and, because most of those cells were
// center-aligned, the overflow-into-empty-neighbors logic stretched the
// glyphs out into the visible band — making it look like ghost values
// appeared in the rendered sheet.
//
// We exercise the bug end-to-end through the node render path:
//   1. Load the existing `outline-groups.xlsx` fixture (cols B/C/D have
//      outlineLevel 1, rows 3/4 and 7/8 have row outlineLevel 1).
//   2. Collapse the outline columns by passing width-0 overrides for
//      B/C/D, exactly mirroring what `setRunCollapsed` does when the
//      user clicks the `[-]` button in the browser.
//   3. Render to a real skia canvas, then wrap its 2D context to record
//      every `fillText` call.
//
// Assertion: none of the text content that lives only in B/C/D ("Q1",
// "Q2", "Q3", and the numeric Q1/Q2/Q3 cell values like "100", "110",
// "120", ...) should be drawn. The "Total" / "Notes" / row label
// columns are unaffected and *must* still render.

import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { expect, test } from "vitest";
// Import the *built* node entry so we pick up the wasm next to `dist/`.
// Running directly from `./node.js` (TS source) would try to load the
// wasm from `src/`, which is not where the build copies it.
import { loadWorkbookFromXlsx, renderToCanvas } from "../dist/index.js";

const FIXTURE = resolve(
  fileURLToPath(import.meta.url),
  "../../../../tests/fixtures/outline/outline-groups.xlsx",
);

// Strings that only appear in the outline-grouped columns B/C/D and
// must therefore never reach `fillText` when those columns are
// collapsed. The column headers ("Q1"/"Q2"/"Q3") live in row 1 of
// cols B/C/D; the numeric values live in rows 2-9. We intentionally
// pick numbers that don't collide with anything in the visible
// "Total" / row-label cells.
const FORBIDDEN_LITERALS = new Set([
  "Q1",
  "Q2",
  "Q3",
  // Region A row values in B/C/D (rows 2-5).
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
  // Region B row values in B/C/D (rows 6-9).
  "80",
  "85",
  "90",
  "160",
  "170",
  "180",
]);

// Strings that *must* still render — sanity-check that we haven't
// over-corrected and dropped all text. These live in cols A/E/F which
// remain visible.
const REQUIRED_LITERALS = ["Region/Quarter", "Total", "Notes", "Region A total"];

async function loadCollapsedAndRecordFillText() {
  const bytes = await readFile(FIXTURE);
  const layout = await loadWorkbookFromXlsx(bytes);
  const sheet = layout.sheets[0]!;

  // Mirror the interactive collapse: force every column with
  // outlineLevel ≥ 1 to width 0.
  const colOverrides = new Map<number, number>();
  for (const c of sheet.cols) {
    if ((c.outlineLevel ?? 0) >= 1) {
      for (let i = c.min; i <= c.max; i++) colOverrides.set(i, 0);
    }
  }
  // Sanity: the fixture must have at least one grouped column.
  expect(colOverrides.size).toBeGreaterThan(0);

  const canvas = renderToCanvas(layout, { colOverrides });

  // Re-render through a recording proxy. Skia's `getContext` returns
  // the *same* 2D context each call, so we can wrap it after the fact:
  // pull every fillText call out of the raw canvas by replaying the
  // render with a spying ctx.
  //
  // Easier: render once with a proxy from the start. We do that by
  // calling `renderToCanvas` again but intercepting via a per-canvas
  // context wrapper. Since renderToCanvas constructs the canvas
  // internally, we wrap the prototype's `getContext` temporarily.
  const fillTextCalls: string[] = [];
  type Ctx2D = CanvasRenderingContext2D & { __wrapped?: boolean };
  const proto = Object.getPrototypeOf(canvas.getContext("2d") as Ctx2D) as Ctx2D;
  const originalFillText = proto.fillText;
  proto.fillText = function (this: Ctx2D, text: string, x: number, y: number, maxWidth?: number) {
    fillTextCalls.push(text);
    return originalFillText.call(this, text, x, y, maxWidth as number);
  } as Ctx2D["fillText"];
  try {
    // Second render, this time every fillText is captured.
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
