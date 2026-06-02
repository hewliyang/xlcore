import { useState } from "react";
import { Workbook } from "@hewliyang/xlsx-preview/api";
import { ExcelPreviewer, PivotBuilder } from "@hewliyang/xlsx-preview/react";

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
  const [file, setFile] = useState<File | null>(null);
  const [workbook, setWorkbook] = useState<Workbook | null>(null);
  const [sourceRef, setSourceRef] = useState("");

  async function onFile(f: File | null) {
    setFile(f);
    setWorkbook(null);
    if (!f) return;
    const wb = await Workbook.open(await f.arrayBuffer());
    const ws = wb.activeSheet();
    const ref = `${ws.name}!A1:${colLetter(ws.columnCount)}${ws.rowCount}`;
    setWorkbook(wb);
    setSourceRef(ref);
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
              <input
                value={sourceRef}
                onChange={(e) => setSourceRef(e.target.value)}
                style={{ width: 320 }}
              />
            </label>
          </p>
          <PivotBuilder
            workbook={workbook}
            sourceRef={sourceRef}
            style={{ marginBottom: 24 }}
          />
        </>
      )}

      <div
        style={{
          height: "60vh",
          border: "1px solid #ddd",
          borderRadius: 8,
          overflow: "hidden",
        }}
      >
        <ExcelPreviewer file={file} style={{ width: "100%", height: "100%" }} />
      </div>
    </div>
  );
}
