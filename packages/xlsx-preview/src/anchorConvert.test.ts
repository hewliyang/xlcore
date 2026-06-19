import { describe, expect, it } from "vitest";
import type { DrawingAnchor } from "./schema/DrawingAnchor.js";
import {
  buildDrawingMovedDetail,
  chartAnchorToWireAnchor,
  wireAnchorToChartAnchor,
} from "./anchorConvert.js";

const cases: DrawingAnchor[] = [
  { fromCol: 0, fromColOffEmu: 0, fromRow: 0, fromRowOffEmu: 0, toCol: 0, toColOffEmu: 0, toRow: 0, toRowOffEmu: 0 },
  { fromCol: 1, fromColOffEmu: 9525, fromRow: 2, fromRowOffEmu: 19050, toCol: 5, toColOffEmu: 12345, toRow: 8, toRowOffEmu: 67890 },
  {
    fromCol: 10,
    fromColOffEmu: 123456789,
    fromRow: 20,
    fromRowOffEmu: 987654321,
    toCol: 30,
    toColOffEmu: 5000000000,
    toRow: 40,
    toRowOffEmu: 9000000000,
  },
];

describe("anchorConvert", () => {
  it("round-trips wire -> chart -> wire", () => {
    for (const a of cases) {
      const back = chartAnchorToWireAnchor(wireAnchorToChartAnchor(a));
      expect(back.fromCol).toBe(a.fromCol);
      expect(back.fromColOffEmu).toBe(a.fromColOffEmu);
      expect(back.fromRow).toBe(a.fromRow);
      expect(back.fromRowOffEmu).toBe(a.fromRowOffEmu);
      expect(back.toCol).toBe(a.toCol);
      expect(back.toColOffEmu).toBe(a.toColOffEmu);
      expect(back.toRow).toBe(a.toRow);
      expect(back.toRowOffEmu).toBe(a.toRowOffEmu);
    }
  });

  it("emits bigint offsets only when nonzero", () => {
    const zero = wireAnchorToChartAnchor(cases[0]!);
    expect(zero.fromColumnOffsetEmu).toBeUndefined();
    expect(zero.toRowOffsetEmu).toBeUndefined();

    const nonzero = wireAnchorToChartAnchor(cases[1]!);
    expect(nonzero.fromColumnOffsetEmu).toBe(9525n);
    expect(nonzero.toRowOffsetEmu).toBe(67890n);
  });

  it("builds drawingmoved detail with converted anchors", () => {
    const detail = buildDrawingMovedDetail("Sheet1", "chart", 2, cases[0]!, cases[1]!);
    expect(detail).toEqual({
      sheetName: "Sheet1",
      kind: "chart",
      drawingIndex: 2,
      prevAnchor: wireAnchorToChartAnchor(cases[0]!),
      anchor: wireAnchorToChartAnchor(cases[1]!),
    });
    const ev = new CustomEvent("drawingmoved", { detail });
    expect((ev.detail as typeof detail).anchor.toColumn).toBe(5);
  });

  it("defaults missing chart offsets to 0", () => {
    const wire = chartAnchorToWireAnchor({ fromColumn: 1, fromRow: 2, toColumn: 3, toRow: 4 });
    expect(wire.fromColOffEmu).toBe(0);
    expect(wire.toRowOffEmu).toBe(0);
  });
});
