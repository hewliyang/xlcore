import { describe, expect, it } from "vitest";
import { parseClipboard, serializeRange } from "./clipboardModel.js";
import type { Sheet, WorkbookLayout } from "./types.js";

interface TestCell {
  v?: string;
  f?: string;
}

function buildLayout(grid: (TestCell | null)[][], name = "Sheet1"): WorkbookLayout {
  const valuePool: string[] = [];
  const formulaPool: string[] = [];
  const rs: number[] = [];
  const cs: number[] = [];
  const kind: number[] = [];
  const valueIdx: number[] = [];
  const formulaIdx: number[] = [];
  const styleIdx: number[] = [];
  const runsIdx: number[] = [];
  const rowPtr: number[] = [0];
  const index: number[] = [];
  for (let r = 0; r < grid.length; r++) {
    const row = grid[r] ?? [];
    let any = false;
    for (let c = 0; c < row.length; c++) {
      const cell = row[c];
      if (!cell) continue;
      any = true;
      rs.push(r + 1);
      cs.push(c + 1);
      kind.push(5);
      if (cell.v !== undefined) {
        valueIdx.push(valuePool.length);
        valuePool.push(cell.v);
      } else valueIdx.push(-1);
      if (cell.f !== undefined) {
        formulaIdx.push(formulaPool.length);
        formulaPool.push(cell.f);
      } else formulaIdx.push(-1);
      styleIdx.push(-1);
      runsIdx.push(-1);
    }
    if (any) {
      index.push(r + 1);
      rowPtr.push(rs.length);
    }
  }
  const byIndex = new Map<number, number>();
  index.forEach((idx, i) => {
    byIndex.set(idx, i);
  });
  const sheet = {
    name,
    cols: [],
    valuePool,
    formulaPool,
    inlineRuns: [],
    decodedCells: {
      count: rs.length,
      r: Uint32Array.from(rs),
      c: Uint32Array.from(cs),
      kind: Uint8Array.from(kind),
      valueIdx: Int32Array.from(valueIdx),
      formulaIdx: Int32Array.from(formulaIdx),
      styleIdx: Int32Array.from(styleIdx),
      runsIdx: Int32Array.from(runsIdx),
      rowPtr: Uint32Array.from(rowPtr),
    },
    decodedRowMeta: {
      count: index.length,
      index: Uint32Array.from(index),
      heightPx: Float32Array.from(index.map(() => Number.NaN)),
      styleIdx: Int32Array.from(index.map(() => -1)),
      hidden: Uint8Array.from(index.map(() => 0)),
      outlineLevel: new Uint8Array(0),
      byIndex,
    },
  } as unknown as Sheet;
  return {
    sheets: [sheet],
    styles: { cellXfs: [] },
    sharedStrings: [],
    sharedStringRuns: [],
    dxfs: [],
    tableStyles: [],
    definedNames: [],
  } as unknown as WorkbookLayout;
}

describe("serializeRange", () => {
  it("produces plain TSV grid", () => {
    const layout = buildLayout([
      [{ v: "a" }, { v: "b" }],
      [{ v: "c" }, { v: "d" }],
    ]);
    const { tsv } = serializeRange(layout, "Sheet1", { r1: 1, c1: 1, r2: 2, c2: 2 });
    expect(tsv).toBe("a\tb\nc\td");
  });

  it("quotes fields with tab, newline, or quote", () => {
    const layout = buildLayout([
      [{ v: "ta\tb" }, { v: "line\nbreak" }],
      [{ v: 'say "hi"' }, { v: "plain" }],
    ]);
    const { tsv } = serializeRange(layout, "Sheet1", { r1: 1, c1: 1, r2: 2, c2: 2 });
    expect(tsv).toBe('"ta\tb"\t"line\nbreak"\n"say ""hi"""\tplain');
  });

  it("embeds internal payload with formulas in html", () => {
    const layout = buildLayout([[{ v: "3", f: "=1+2" }, { v: "x" }]]);
    const { html } = serializeRange(layout, "Sheet1", { r1: 1, c1: 1, r2: 1, c2: 2 });
    expect(html).toContain("data-xlcore=");
    const parsed = parseClipboard({ html });
    expect(parsed.source).toBe("internal");
    expect(parsed.values).toEqual([["3", "x"]]);
    expect(parsed.formulas).toEqual([["=1+2", null]]);
  });
});

describe("round-trip serialize -> parse", () => {
  it("internal payload survives tab/newline/quote", () => {
    const layout = buildLayout([
      [{ v: "ta\tb" }, { v: "line\nbreak" }],
      [{ v: 'say "hi"' }, { v: "plain" }],
    ]);
    const { html } = serializeRange(layout, "Sheet1", { r1: 1, c1: 1, r2: 2, c2: 2 });
    const parsed = parseClipboard({ html });
    expect(parsed.source).toBe("internal");
    expect(parsed.values).toEqual([
      ["ta\tb", "line\nbreak"],
      ['say "hi"', "plain"],
    ]);
  });
});

describe("parseClipboard external", () => {
  it("parses plain TSV", () => {
    const parsed = parseClipboard({ tsv: "a\tb\nc\td" });
    expect(parsed.source).toBe("external");
    expect(parsed.values).toEqual([
      ["a", "b"],
      ["c", "d"],
    ]);
  });

  it("parses TSV with quoting", () => {
    const parsed = parseClipboard({ tsv: '"ta\tb"\t"line\nbreak"\n"say ""hi"""\tplain' });
    expect(parsed.values).toEqual([
      ["ta\tb", "line\nbreak"],
      ['say "hi"', "plain"],
    ]);
  });

  it("parses an external html table", () => {
    const html = "<table><tbody><tr><td>a</td><td>b&amp;c</td></tr><tr><td>1</td><td>2</td></tr></tbody></table>";
    const parsed = parseClipboard({ html });
    expect(parsed.source).toBe("external");
    expect(parsed.values).toEqual([
      ["a", "b&c"],
      ["1", "2"],
    ]);
  });
});
