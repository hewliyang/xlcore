import { expect, test } from "bun:test";
import { computeOutlineRuns, isOutlineRunCollapsed, outlineButtonHits } from "./outlineGutter";
import type { Grid } from "./grid";
import type { Sheet } from "./types";

// Build a synthetic grid + sheet for outline-runs tests. Geometry is
// deliberately simple: 10 rows × 5 cols, every row 20px, every col 80px.
// Mirrors the `outline-groups.xlsx` fixture (rows 3-4 grouped, rows 7-8
// grouped, cols B-D grouped — all at level 1, summaryBelow/right=true).

function makeGrid(opts: {
  rowOutlineDepth: number;
  colOutlineDepth: number;
  hiddenRows?: Set<number>;
  hiddenCols?: Set<number>;
}): Grid {
  const rowH: number[] = [0];
  const rowY: number[] = [0, 50];
  for (let r = 1; r <= 10; r++) {
    const h = opts.hiddenRows?.has(r) ? 0 : 20;
    rowH[r] = h;
    rowY[r + 1] = (rowY[r] ?? 0) + h;
  }
  const colW: number[] = [0];
  const colX: number[] = [0, 76]; // originX = HEADER_W(44) + rowGutterW(32)
  for (let c = 1; c <= 5; c++) {
    const w = opts.hiddenCols?.has(c) ? 0 : 80;
    colW[c] = w;
    colX[c + 1] = (colX[c] ?? 0) + w;
  }
  return {
    colX,
    colW,
    rowY,
    rowH,
    totalW: colX[6] ?? 0,
    totalH: rowY[11] ?? 0,
    maxCol: 5,
    maxRow: 10,
    rowGutterW: opts.rowOutlineDepth > 0 ? 8 + (opts.rowOutlineDepth + 1) * 12 : 0,
    colGutterH: opts.colOutlineDepth > 0 ? 8 + (opts.colOutlineDepth + 1) * 12 : 0,
    originX: 76,
    originY: 50,
    rowOutlineDepth: opts.rowOutlineDepth,
    colOutlineDepth: opts.colOutlineDepth,
  };
}

function makeSheet(opts: {
  rowGroups?: Array<{ start: number; end: number }>;
  colGroups?: Array<{ start: number; end: number }>;
}): Sheet {
  const rowOutline = new Uint8Array(11);
  for (const g of opts.rowGroups ?? []) {
    for (let r = g.start; r <= g.end; r++) rowOutline[r] = 1;
  }
  const cols: Sheet["cols"] = [{ min: 1, max: 5, widthPx: 80, outlineLevel: 0 } as any];
  if (opts.colGroups && opts.colGroups.length > 0) {
    const arr: Sheet["cols"] = [];
    for (let c = 1; c <= 5; c++) {
      const inGroup = opts.colGroups.some((g) => c >= g.start && c <= g.end);
      arr.push({ min: c, max: c, widthPx: 80, outlineLevel: inGroup ? 1 : 0 } as any);
    }
    return makeSheet0(rowOutline, arr);
  }
  return makeSheet0(rowOutline, cols);
}

function makeSheet0(rowOutline: Uint8Array, cols: Sheet["cols"]): Sheet {
  const idx: number[] = [];
  for (let r = 1; r < rowOutline.length; r++) idx.push(r);
  return {
    name: "Test",
    maxRow: 10,
    maxCol: 5,
    defaultColWidthPx: 80,
    defaultRowHeightPx: 20,
    cols,
    rows: [],
    decodedRowMeta: {
      count: 10,
      index: idx,
      heightPx: new Float32Array(10),
      hidden: new Uint8Array(10),
      outlineLevel: rowOutline.slice(1),
    } as any,
    merges: [],
  } as any;
}

test("computeOutlineRuns finds row groups (summaryBelow=true default)", () => {
  const sheet = makeSheet({
    rowGroups: [
      { start: 3, end: 4 },
      { start: 7, end: 8 },
    ],
  });
  const grid = makeGrid({ rowOutlineDepth: 1, colOutlineDepth: 0 });
  const runs = computeOutlineRuns(sheet, grid);
  expect(runs.length).toBe(2);
  expect(runs[0]).toEqual({ axis: "row", level: 1, start: 3, end: 4, summary: 5 });
  expect(runs[1]).toEqual({ axis: "row", level: 1, start: 7, end: 8, summary: 9 });
});

test("computeOutlineRuns finds col groups (summaryRight=true default)", () => {
  const sheet = makeSheet({ colGroups: [{ start: 2, end: 4 }] });
  const grid = makeGrid({ rowOutlineDepth: 0, colOutlineDepth: 1 });
  const runs = computeOutlineRuns(sheet, grid);
  expect(runs.length).toBe(1);
  expect(runs[0]).toEqual({ axis: "col", level: 1, start: 2, end: 4, summary: 5 });
});

test("isOutlineRunCollapsed reports true when every detail row has zero height", () => {
  const sheet = makeSheet({ rowGroups: [{ start: 3, end: 4 }] });
  const expandedGrid = makeGrid({ rowOutlineDepth: 1, colOutlineDepth: 0 });
  const collapsedGrid = makeGrid({
    rowOutlineDepth: 1,
    colOutlineDepth: 0,
    hiddenRows: new Set([3, 4]),
  });
  const [run] = computeOutlineRuns(sheet, expandedGrid);
  expect(run).toBeDefined();
  expect(isOutlineRunCollapsed(run!, expandedGrid)).toBe(false);
  expect(isOutlineRunCollapsed(run!, collapsedGrid)).toBe(true);
});

test("outlineButtonHits emits a button at the summary row, in the correct level track", () => {
  const sheet = makeSheet({ rowGroups: [{ start: 3, end: 4 }] });
  const grid = makeGrid({ rowOutlineDepth: 1, colOutlineDepth: 0 });
  const hits = outlineButtonHits(sheet, grid, {
    sx: 0,
    sy: 0,
    splitX: 1,
    splitY: 1,
    pcw: 0,
    prh: 0,
    canvasW: 1000,
    canvasH: 1000,
  });
  expect(hits.length).toBe(1);
  // summary row 5 → y = 50 (originY) + 4*20 (rows 1..4) + 10 (row 5 / 2) = 140
  expect(hits[0]!.cy).toBe(140);
  // level-1 track: OUTLINE_GUTTER_PAD(4) + 0*12 + 12/2 = 10
  expect(hits[0]!.cx).toBe(10);
  expect(hits[0]!.collapsed).toBe(false);
});

test("outlineButtonHits still emits a button when the run is fully collapsed", () => {
  const sheet = makeSheet({ rowGroups: [{ start: 3, end: 4 }] });
  const grid = makeGrid({
    rowOutlineDepth: 1,
    colOutlineDepth: 0,
    hiddenRows: new Set([3, 4]),
  });
  const hits = outlineButtonHits(sheet, grid, {
    sx: 0,
    sy: 0,
    splitX: 1,
    splitY: 1,
    pcw: 0,
    prh: 0,
    canvasW: 1000,
    canvasH: 1000,
  });
  expect(hits.length).toBe(1);
  expect(hits[0]!.collapsed).toBe(true);
  // After rows 3-4 collapse, summary row 5 starts at y=50+2*20=90 (only rows 1-2 above it),
  // center = 90 + 10 = 100.
  expect(hits[0]!.cy).toBe(100);
});

test("outlineButtonHits omits buttons whose summary row is hidden", () => {
  const sheet = makeSheet({ rowGroups: [{ start: 3, end: 4 }] });
  const grid = makeGrid({
    rowOutlineDepth: 1,
    colOutlineDepth: 0,
    hiddenRows: new Set([5]),
  });
  const hits = outlineButtonHits(sheet, grid, {
    sx: 0,
    sy: 0,
    splitX: 1,
    splitY: 1,
    pcw: 0,
    prh: 0,
    canvasW: 1000,
    canvasH: 1000,
  });
  expect(hits.length).toBe(0);
});
