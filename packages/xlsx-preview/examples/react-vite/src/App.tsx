import { useRef, useState } from "react";
import { Workbook } from "@hewliyang/xlsx-preview/api";
import { ExcelPreviewer, PivotBuilder, type PivotBuilderConfig } from "@hewliyang/xlsx-preview/react";

const OUTPUT_SHEET = "PivotView";

function colLetter(n: number): string {
  let s = "";
  let x = n;
  while (x > 0) {
    const r = (x - 1) % 26;
    s = String.fromCharCode(65 + r) + s;
    x = Math.floor((x - 1) / 26);
  }
  return s || "A";
}

export function App() {
  const [workbook, setWorkbook] = useState<Workbook | null>(null);
  const [sourceRef, setSourceRef] = useState("");
  const [fileBlob, setFileBlob] = useState<Blob | null>(null);
  const wbRef = useRef<Workbook | null>(null);
  const pivotIdRef = useRef<string | null>(null);
  const sourceRefRef = useRef("");

  async function onFile(f: File | null) {
    setWorkbook(null);
    setFileBlob(null);
    wbRef.current = null;
    pivotIdRef.current = null;
    if (!f) return;
    const wb = await Workbook.open(await f.arrayBuffer());
    const dataSheet = wb.activeSheet();
    const ref = `${dataSheet.name}!A1:${colLetter(dataSheet.columnCount)}${dataSheet.rowCount}`;
    if (!wb.worksheets().some((w) => w.name === OUTPUT_SHEET)) wb.addSheet(OUTPUT_SHEET);
    wbRef.current = wb;
    sourceRefRef.current = ref;
    setWorkbook(wb);
    setSourceRef(ref);
    setFileBlob(new Blob([wb.save()]));
  }

  function onConfig(cfg: PivotBuilderConfig) {
    const wb = wbRef.current;
    if (!wb) return;
    const out = wb.sheet(OUTPUT_SHEET);
    if (pivotIdRef.current) {
      try {
        out.pivots.remove(pivotIdRef.current);
      } catch {}
      pivotIdRef.current = null;
    }
    if (cfg.rowFields.length > 0 && cfg.dataFields.length > 0) {
      try {
        const info = out.pivots.set({
          anchorCell: `${OUTPUT_SHEET}!A1`,
          sourceRef: sourceRefRef.current,
          name: "BuilderPivot",
          ...cfg,
        });
        pivotIdRef.current = info.id;
      } catch (err) {
        console.error(err);
      }
    }
    setFileBlob(new Blob([wb.save()]));
  }

  return (
    <div style={{ fontFamily: "system-ui, sans-serif", padding: 16 }}>
      <h1 style={{ margin: 0 }}>xlsx-preview · Pivot Builder</h1>
      <p>
        <input
          type="file"
          accept=".xlsx,.csv,.tsv,.parquet,.pqt"
          onChange={(e) => onFile(e.target.files?.[0] ?? null)}
        />
      </p>

      {workbook && (
        <>
          <p>
            <label>
              Source range:{" "}
              <input value={sourceRef} onChange={(e) => setSourceRef(e.target.value)} style={{ width: 320 }} />
            </label>
          </p>
          <PivotBuilder
            workbook={workbook}
            sourceRef={sourceRef}
            showPreview={false}
            onChange={onConfig}
            style={{ marginBottom: 16 }}
          />
        </>
      )}

      <div style={{ height: "60vh", border: "1px solid #ddd", borderRadius: 8, overflow: "hidden" }}>
        <ExcelPreviewer file={fileBlob} initialSheet={OUTPUT_SHEET} style={{ width: "100%", height: "100%" }} />
      </div>
    </div>
  );
}
