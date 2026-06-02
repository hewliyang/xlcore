import { type CSSProperties, type DragEvent, useEffect, useMemo, useRef, useState } from "react";
import type { Workbook } from "./api.js";
import type { PivotAggregation, PivotDataField, PivotGrid } from "./api-schema/index.js";

type Bucket = "available" | "rows" | "columns" | "filters" | "values";

const AGGREGATIONS: PivotAggregation[] = [
  "sum",
  "count",
  "average",
  "max",
  "min",
  "product",
  "count_nums",
  "std_dev",
  "std_devp",
  "var",
  "varp",
];

const AGG_LABEL: Record<PivotAggregation, string> = {
  sum: "Sum",
  count: "Count",
  average: "Average",
  max: "Max",
  min: "Min",
  product: "Product",
  count_nums: "Count Numbers",
  std_dev: "StdDev",
  std_devp: "StdDevp",
  var: "Var",
  varp: "Varp",
};

export interface PivotBuilderProps {
  workbook: Workbook;
  sourceRef: string;
  outputSheet?: string;
  anchorCell?: string;
  initial?: {
    rowFields?: string[];
    columnFields?: string[];
    filterFields?: string[];
    dataFields?: PivotDataField[];
  };
  className?: string;
  style?: CSSProperties;
}

interface BuilderState {
  rows: string[];
  columns: string[];
  filters: string[];
  values: PivotDataField[];
}

export function applyMove(
  state: BuilderState,
  field: string,
  from: Bucket,
  to: Bucket,
): BuilderState {
  if (from === to) return state;
  const existing = state.values.find((v) => v.field === field);
  const next: BuilderState = {
    rows: state.rows.filter((f) => f !== field),
    columns: state.columns.filter((f) => f !== field),
    filters: state.filters.filter((f) => f !== field),
    values: state.values.filter((v) => v.field !== field),
  };
  if (to === "rows") next.rows.push(field);
  else if (to === "columns") next.columns.push(field);
  else if (to === "filters") next.filters.push(field);
  else if (to === "values") next.values.push(existing ?? { field, aggregation: "sum" });
  return next;
}

export function parseRef(ref: string): { sheet?: string; a1: string } {
  const bang = ref.lastIndexOf("!");
  if (bang < 0) return { a1: ref };
  let sheet = ref.slice(0, bang);
  if (sheet.startsWith("'") && sheet.endsWith("'")) {
    sheet = sheet.slice(1, -1).replace(/''/g, "'");
  }
  return { sheet, a1: ref.slice(bang + 1) };
}

export function headerRange(a1: string): string {
  const [start, end] = a1.split(":");
  if (!start || !end) return a1;
  const col = (s: string) => s.replace(/\d+/g, "");
  const rowNum = (s: string) => Number.parseInt(s.replace(/\D+/g, ""), 10) || 1;
  return `${col(start)}${rowNum(start)}:${col(end)}${rowNum(start)}`;
}

export function PivotBuilder({
  workbook,
  sourceRef,
  outputSheet,
  anchorCell,
  initial,
  className,
  style,
}: PivotBuilderProps) {
  const { sheet: srcSheet, a1 } = useMemo(() => parseRef(sourceRef), [sourceRef]);

  const headers = useMemo<string[]>(() => {
    try {
      const ws = srcSheet ? workbook.sheet(srcSheet) : workbook.activeSheet();
      const row = ws.range(headerRange(a1)).values()[0] ?? [];
      return row.map((c, i) => (c.type === "blank" ? `Column ${i + 1}` : String(c.value)));
    } catch {
      return [];
    }
  }, [workbook, srcSheet, a1]);

  const [state, setState] = useState<BuilderState>(() => ({
    rows: initial?.rowFields ?? [],
    columns: initial?.columnFields ?? [],
    filters: initial?.filterFields ?? [],
    values: initial?.dataFields ?? [],
  }));

  const used = new Set([
    ...state.rows,
    ...state.columns,
    ...state.filters,
    ...state.values.map((v) => v.field),
  ]);
  const available = headers.filter((h) => !used.has(h));

  const dragRef = useRef<{ field: string; from: Bucket } | null>(null);

  const onDragStart = (field: string, from: Bucket) => (e: DragEvent) => {
    dragRef.current = { field, from };
    e.dataTransfer.effectAllowed = "move";
  };

  const moveField = (field: string, from: Bucket, to: Bucket) => {
    setState((prev) => applyMove(prev, field, from, to));
  };

  const onDrop = (to: Bucket) => (e: DragEvent) => {
    e.preventDefault();
    const dragged = dragRef.current;
    dragRef.current = null;
    if (dragged) moveField(dragged.field, dragged.from, to);
  };

  const setAggregation = (field: string, aggregation: PivotAggregation) => {
    setState((prev) => ({
      ...prev,
      values: prev.values.map((v) => (v.field === field ? { ...v, aggregation } : v)),
    }));
  };

  const [grid, setGrid] = useState<PivotGrid | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (state.rows.length === 0 || state.values.length === 0) {
      setGrid(null);
      setError(null);
      return;
    }
    try {
      const scope = outputSheet ?? srcSheet;
      const ws = scope ? workbook.sheet(scope) : workbook.activeSheet();
      const result = ws.pivots.preview({
        anchorCell: anchorCell ?? "A1",
        sourceRef,
        rowFields: state.rows,
        columnFields: state.columns,
        filterFields: state.filters,
        dataFields: state.values,
      });
      setGrid(result);
      setError(null);
    } catch (err) {
      setGrid(null);
      setError(err instanceof Error ? err.message : String(err));
    }
  }, [workbook, sourceRef, srcSheet, outputSheet, anchorCell, state]);

  return (
    <div className={className} style={{ ...CONTAINER_STYLE, ...style }}>
      <div style={SIDEBAR_STYLE}>
        <Zone
          label="Fields"
          bucket="available"
          fields={available}
          onDragStart={onDragStart}
          onDrop={onDrop}
        />
        <div style={BUCKET_GRID_STYLE}>
          <Zone
            label="Filters"
            bucket="filters"
            fields={state.filters}
            onDragStart={onDragStart}
            onDrop={onDrop}
          />
          <Zone
            label="Columns"
            bucket="columns"
            fields={state.columns}
            onDragStart={onDragStart}
            onDrop={onDrop}
          />
          <Zone
            label="Rows"
            bucket="rows"
            fields={state.rows}
            onDragStart={onDragStart}
            onDrop={onDrop}
          />
          <ValuesZone
            values={state.values}
            onDragStart={onDragStart}
            onDrop={onDrop}
            onAggregation={setAggregation}
          />
        </div>
      </div>
      <div style={PREVIEW_STYLE}>
        {error ? (
          <p style={MESSAGE_STYLE}>{error}</p>
        ) : grid ? (
          <GridTable grid={grid} />
        ) : (
          <p style={MESSAGE_STYLE}>Drag a field into Rows and Values to build a pivot.</p>
        )}
      </div>
    </div>
  );
}

interface ZoneProps {
  label: string;
  bucket: Bucket;
  fields: string[];
  onDragStart: (field: string, from: Bucket) => (e: DragEvent) => void;
  onDrop: (to: Bucket) => (e: DragEvent) => void;
}

function Zone({ label, bucket, fields, onDragStart, onDrop }: ZoneProps) {
  return (
    <div style={ZONE_STYLE} onDragOver={(e) => e.preventDefault()} onDrop={onDrop(bucket)}>
      <div style={ZONE_LABEL_STYLE}>{label}</div>
      {fields.map((f) => (
        <div key={f} draggable onDragStart={onDragStart(f, bucket)} style={CHIP_STYLE}>
          {f}
        </div>
      ))}
    </div>
  );
}

interface ValuesZoneProps {
  values: PivotDataField[];
  onDragStart: (field: string, from: Bucket) => (e: DragEvent) => void;
  onDrop: (to: Bucket) => (e: DragEvent) => void;
  onAggregation: (field: string, agg: PivotAggregation) => void;
}

function ValuesZone({ values, onDragStart, onDrop, onAggregation }: ValuesZoneProps) {
  return (
    <div style={ZONE_STYLE} onDragOver={(e) => e.preventDefault()} onDrop={onDrop("values")}>
      <div style={ZONE_LABEL_STYLE}>Values</div>
      {values.map((v) => (
        <div
          key={v.field}
          draggable
          onDragStart={onDragStart(v.field, "values")}
          style={VALUE_CHIP_STYLE}
        >
          <span style={{ flex: 1 }}>{v.field}</span>
          <select
            value={v.aggregation}
            onChange={(e) => onAggregation(v.field, e.target.value as PivotAggregation)}
            style={SELECT_STYLE}
          >
            {AGGREGATIONS.map((a) => (
              <option key={a} value={a}>
                {AGG_LABEL[a]}
              </option>
            ))}
          </select>
        </div>
      ))}
    </div>
  );
}

function GridTable({ grid }: { grid: PivotGrid }) {
  const matrix: (PivotGrid["cells"][number] | undefined)[][] = Array.from(
    { length: grid.rows },
    () => new Array(grid.cols).fill(undefined),
  );
  for (const c of grid.cells) {
    const row = matrix[c.row];
    if (row && c.col < grid.cols) row[c.col] = c;
  }
  return (
    <table style={TABLE_STYLE}>
      <tbody>
        {matrix.map((row, ri) => (
          <tr key={ri}>
            {row.map((cell, ci) => (
              <td key={ci} style={cellStyle(cell)}>
                {cell?.value ?? ""}
              </td>
            ))}
          </tr>
        ))}
      </tbody>
    </table>
  );
}

function cellStyle(cell: PivotGrid["cells"][number] | undefined): CSSProperties {
  const base: CSSProperties = { ...TD_STYLE };
  if (!cell) return base;
  switch (cell.role) {
    case "header":
      return { ...base, background: "#4472C4", color: "#fff", fontWeight: 600 };
    case "total_label":
      return { ...base, fontWeight: 700 };
    case "total_value":
      return { ...base, fontWeight: 700, textAlign: "right" };
    case "value":
      return { ...base, textAlign: "right" };
    default:
      return base;
  }
}

const CONTAINER_STYLE: CSSProperties = {
  display: "flex",
  gap: 16,
  fontFamily: "system-ui, -apple-system, Segoe UI, Roboto, sans-serif",
  fontSize: 13,
  alignItems: "flex-start",
};
const SIDEBAR_STYLE: CSSProperties = {
  width: 260,
  flexShrink: 0,
  display: "flex",
  flexDirection: "column",
  gap: 12,
};
const BUCKET_GRID_STYLE: CSSProperties = { display: "flex", flexDirection: "column", gap: 8 };
const ZONE_STYLE: CSSProperties = {
  border: "1px solid #d4d4d8",
  borderRadius: 6,
  padding: 8,
  minHeight: 44,
  background: "#fafafa",
};
const ZONE_LABEL_STYLE: CSSProperties = {
  fontSize: 11,
  textTransform: "uppercase",
  letterSpacing: 0.5,
  color: "#71717a",
  marginBottom: 6,
};
const CHIP_STYLE: CSSProperties = {
  background: "#fff",
  border: "1px solid #d4d4d8",
  borderRadius: 4,
  padding: "4px 8px",
  marginBottom: 4,
  cursor: "grab",
};
const VALUE_CHIP_STYLE: CSSProperties = {
  ...CHIP_STYLE,
  display: "flex",
  alignItems: "center",
  gap: 6,
};
const SELECT_STYLE: CSSProperties = { fontSize: 12, border: "1px solid #d4d4d8", borderRadius: 4 };
const PREVIEW_STYLE: CSSProperties = { flex: 1, overflow: "auto" };
const MESSAGE_STYLE: CSSProperties = { color: "#71717a" };
const TABLE_STYLE: CSSProperties = { borderCollapse: "collapse", fontSize: 13 };
const TD_STYLE: CSSProperties = {
  border: "1px solid #e4e4e7",
  padding: "3px 10px",
  whiteSpace: "nowrap",
};
