import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, test } from "vitest";

const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");

type NodeModule = typeof import("./node.js");
async function loadNode(): Promise<NodeModule> {
  return (await import(resolve(packageRoot, "dist/node.js"))) as NodeModule;
}

const KINDS = [
  "waterfall",
  "funnel",
  "treemap",
  "sunburst",
  "histogram",
  "pareto",
  "boxWhisker",
  "regionMap",
] as const;

describe("chartEx authoring", () => {
  test("authors all 8 kinds, round-trips with resolved data", async () => {
    const { Workbook, loadWorkbookFromXlsx } = await loadNode();
    const wb = await Workbook.create();
    const s = wb.sheet("Sheet1");
    for (let i = 0; i < 6; i++) {
      s.cell({ row: i + 1, column: 1 }).setValue(`Cat${i}`);
      s.cell({ row: i + 1, column: 2 }).setValue((i + 1) * 10);
      s.cell({ row: i + 1, column: 3 }).setValue(i < 3 ? "G1" : "G2");
    }

    for (let k = 0; k < KINDS.length; k++) {
      const kind = KINDS[k]!;
      const patch: Parameters<typeof s.chartsEx.set>[0] = {
        kind,
        title: `${kind}`,
        anchor: { fromColumn: 5, fromRow: 1 + k * 18, toColumn: 12, toRow: 16 + k * 18 },
        categoriesRef: "Sheet1!$A$1:$A$6",
        series: [{ name: "V", valuesRef: "Sheet1!$B$1:$B$6" }],
        subtotals: kind === "waterfall" ? [5] : [],
      };
      if (kind === "treemap" || kind === "sunburst") patch.categoriesRef = "Sheet1!$C$1:$A$6";
      if (kind === "histogram") {
        patch.categoriesRef = undefined;
        patch.binCount = 5;
      }
      if (kind === "boxWhisker") {
        patch.series = [
          { name: "A", valuesRef: "Sheet1!$B$1:$B$6" },
          { name: "B", valuesRef: "Sheet1!$B$1:$B$6" },
        ];
      }
      const info = s.chartsEx.set(patch);
      expect(info.kind).toBe(kind);
    }

    expect(s.chartsEx.list()).toHaveLength(8);

    const bytes = wb.save();
    const layout = await loadWorkbookFromXlsx(bytes);
    const charts = layout.sheets[0]!.drawings.filter((d) => d.chart).map((d) => d.chart);
    expect(charts).toHaveLength(8);
    for (const ch of charts) {
      expect(ch).toBeDefined();
      expect(ch?.type).toBe("chartex");
      expect(ch?.cxLayout).toBeTruthy();
      const hasData = (ch?.series ?? []).some((sr) => sr.values.length > 0);
      expect(hasData).toBe(true);
    }
  });
});
