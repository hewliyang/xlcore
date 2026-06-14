import { useRef, useState } from "react";
import { Workbook } from "@hewliyang/xlsx-preview/api";
import type { PivotFieldFilter } from "@hewliyang/xlsx-preview/api";
import {
  ExcelPreviewer,
  distinctValuesFor,
  type PivotFilterController,
  type TableFilterController,
} from "@hewliyang/xlsx-preview/react";
import type { WorkbookPreviewer } from "@hewliyang/xlsx-preview/previewer";

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
  const [fileBlob, setFileBlob] = useState<Blob | null>(null);
  const wbRef = useRef<Workbook | null>(null);
  const previewerRef = useRef<WorkbookPreviewer | null>(null);
  const pivotIdRef = useRef<string | null>(null);
  const sourceRefRef = useRef("");
  const hiddenRef = useRef<Record<string, string[]>>({});
  const keptRef = useRef<Record<number, string[]>>({});

  async function onFile(f: File | null) {
    setFileBlob(null);
    wbRef.current = null;
    pivotIdRef.current = null;
    hiddenRef.current = {};
    if (!f) return;

    const wb = await Workbook.open(await f.arrayBuffer());
    const data = wb.activeSheet();
    const ref = `${data.name}!A1:${colLetter(data.columnCount)}${data.rowCount}`;
    const headerRow = data.range(`A1:${colLetter(data.columnCount)}1`).values()[0] ?? [];
    const headers = headerRow.map((c, i) => (c.type === "blank" ? `Column ${i + 1}` : String(c.value)));
    if (headers.length < 2) return;

    if (!wb.worksheets().some((w) => w.name === OUTPUT_SHEET)) wb.addSheet(OUTPUT_SHEET);
    const out = wb.sheet(OUTPUT_SHEET);
    const info = out.pivots.set({
      anchorCell: `${OUTPUT_SHEET}!A1`,
      sourceRef: ref,
      name: "DemoPivot",
      rowFields: [headers[0]!],
      columnFields: headers.length > 2 ? [headers[1]!] : [],
      filterFields: [],
      dataFields: [{ field: headers[headers.length - 1]!, aggregation: "sum" }],
    });

    pivotIdRef.current = info.id;
    sourceRefRef.current = ref;
    keptRef.current = {};
    wbRef.current = wb;
    setFileBlob(new Blob([wb.save() as BlobPart]));
  }

  const pivotController: PivotFilterController = {
    items: ({ field }) => distinctValuesFor(wbRef.current!, sourceRefRef.current, field),
    hiddenValues: ({ field }) => hiddenRef.current[field] ?? [],
    setHidden: ({ field, hidden }) => {
      hiddenRef.current = { ...hiddenRef.current, [field]: hidden };
      const wb = wbRef.current;
      if (!wb || !pivotIdRef.current) return;
      const hiddenItems: PivotFieldFilter[] = Object.entries(hiddenRef.current)
        .filter(([, hide]) => hide.length > 0)
        .map(([f, hide]) => ({ field: f, hide }));
      wb.sheet(OUTPUT_SHEET).pivots.update(pivotIdRef.current, { hiddenItems });
      return wb.layout();
    },
  };

  function refSheet(ref: string): string | null {
    const bang = ref.lastIndexOf("!");
    if (bang < 0) return null;
    let s = ref.slice(0, bang);
    if (s.startsWith("'") && s.endsWith("'")) s = s.slice(1, -1).replace(/''/g, "'");
    return s;
  }

  const tableController: TableFilterController = {
    items: ({ rangeRef, field }) => distinctValuesFor(wbRef.current!, rangeRef, field),
    activeValues: ({ columnOffset, rangeRef, field }) =>
      keptRef.current[columnOffset] ?? distinctValuesFor(wbRef.current!, rangeRef, field),
    setFilter: ({ columnOffset, rangeRef, field, values }) => {
      const wb = wbRef.current;
      if (!wb) return;
      const sheetName = refSheet(rangeRef);
      const ws = sheetName ? wb.sheet(sheetName) : wb.activeSheet();
      if (!ws.autoFilter.get()) ws.autoFilter.set(rangeRef);
      const all = distinctValuesFor(wb, rangeRef, field);
      if (values.length === 0 || values.length >= all.length) {
        ws.autoFilter.removeColumn(columnOffset);
        delete keptRef.current[columnOffset];
      } else {
        ws.autoFilter.setColumnValues(columnOffset, values);
        keptRef.current[columnOffset] = values;
      }
      return wb.layout();
    },
    setSort: ({ columnOffset, rangeRef, descending }) => {
      const wb = wbRef.current;
      if (!wb) return;
      const sheetName = refSheet(rangeRef);
      const ws = sheetName ? wb.sheet(sheetName) : wb.activeSheet();
      if (!ws.autoFilter.get()) ws.autoFilter.set(rangeRef);
      if (descending === null) ws.autoFilter.clearSort();
      else ws.autoFilter.setSort(columnOffset, { descending });
      return wb.layout();
    },
  };

  return (
    <div style={{ fontFamily: "system-ui, sans-serif", padding: 16 }}>
      <h1 style={{ margin: 0 }}>xlsx-preview · Pivot filter dropdown</h1>
      <p style={{ color: "#666", marginTop: 4 }}>
        The pivot is authored via <code>xlcore-api</code>. Click a filter arrow on the table to
        hide/show items — it recomputes in place.
      </p>
      <p>
        <input
          type="file"
          accept=".xlsx,.csv,.tsv,.parquet,.pqt"
          onChange={(e) => onFile(e.target.files?.[0] ?? null)}
        />
      </p>

      <div style={{ height: "70vh", border: "1px solid #ddd", borderRadius: 8, overflow: "hidden" }}>
        <ExcelPreviewer
          file={fileBlob}
          initialSheet={OUTPUT_SHEET}
          previewerRef={previewerRef}
          pivotController={pivotController}
          tableController={tableController}
          style={{ width: "100%", height: "100%" }}
        />
      </div>
    </div>
  );
}
