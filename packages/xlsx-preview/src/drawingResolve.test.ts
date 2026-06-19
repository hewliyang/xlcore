import { describe, expect, it } from "vitest";
import type { ChartAnchor } from "./api-schema/ChartAnchor.js";
import type { ChartInfo } from "./api-schema/ChartInfo.js";
import { resolveChartId } from "./drawingResolve.js";

function anchor(fromColumn: number, fromRow: number, toColumn: number, toRow: number): ChartAnchor {
  return { fromColumn, fromRow, toColumn, toRow };
}

function chart(id: string, a: ChartAnchor): ChartInfo {
  return { sheet: "Sheet1", id, name: id, kind: "column", series: [], anchor: a };
}

describe("resolveChartId", () => {
  it("returns the id of the single anchor match", () => {
    const charts = [chart("a", anchor(0, 0, 5, 10)), chart("b", anchor(6, 0, 11, 10))];
    expect(resolveChartId(charts, anchor(6, 0, 11, 10))).toBe("b");
  });

  it("ignores offset/bigint differences when matching cells", () => {
    const charts = [chart("a", { fromColumn: 0, fromRow: 0, toColumn: 5, toRow: 10, fromColumnOffsetEmu: 9525n })];
    expect(resolveChartId(charts, anchor(0, 0, 5, 10))).toBe("a");
  });

  it("falls back to ordinal when ambiguous", () => {
    const charts = [chart("a", anchor(0, 0, 5, 10)), chart("b", anchor(0, 0, 5, 10))];
    expect(resolveChartId(charts, anchor(0, 0, 5, 10), 1)).toBe("b");
  });

  it("falls back to ordinal when no match", () => {
    const charts = [chart("a", anchor(0, 0, 5, 10))];
    expect(resolveChartId(charts, anchor(99, 99, 100, 100), 0)).toBe("a");
  });

  it("returns null when no match and no ordinal", () => {
    const charts = [chart("a", anchor(0, 0, 5, 10))];
    expect(resolveChartId(charts, anchor(99, 99, 100, 100))).toBeNull();
  });
});
