import { Workbook, distinctValuesFor } from "./api.js";
import type { Worksheet } from "./api-worksheet.js";
import type { CellInput } from "./api-range.js";
import type { ClearMode, ImagePatch } from "./api-schema/index.js";
import type {
  ChartAnchor,
  ChartInfo,
  DefinedNameInfo,
  ImageInfo,
  PivotInfo,
  PivotUpdate,
  LayoutOptions as WorkbookLayoutOptions,
} from "./api-schema/index.js";
import { resolveDrawingId } from "./drawingResolve.js";
import { xlsxLoadErrorPayloadFromUnknown } from "./errors.js";

export interface WorkbookStructure {
  sheets: string[];
  definedNames: DefinedNameInfo[];
}

export interface PivotMeta {
  name: string;
  sheet: string;
  id: string;
  sourceRef: string;
}

export type EditWorkerOp =
  | "open"
  | "applyEdit"
  | "setRangeValues"
  | "setRangeFormulas"
  | "copyRange"
  | "clearRange"
  | "setImage"
  | "addSheet"
  | "recalculate"
  | "layout"
  | "save"
  | "pivotMetas"
  | "distinctValues"
  | "updatePivot"
  | "moveDrawing"
  | "removeDrawing"
  | "tableSetFilter"
  | "tableSetSort";

export interface EditWorkerRequest {
  id: number;
  op: EditWorkerOp;
  args: Record<string, unknown>;
}

export type EditWorkerResponse =
  | { id: number; ok: true; result: unknown }
  | { id: number; ok: false; error: ReturnType<typeof xlsxLoadErrorPayloadFromUnknown> };

let wb: Workbook | null = null;

function post(message: EditWorkerResponse, transfer?: Transferable[]): void {
  (
    globalThis as unknown as {
      postMessage(message: unknown, transfer?: Transferable[]): void;
    }
  ).postMessage(message, transfer ?? []);
}

function coerce(input: string): CellInput {
  if (input === "") return null;
  if (/^-?\d+(\.\d+)?$/.test(input)) return Number(input);
  if (input === "true") return true;
  if (input === "false") return false;
  return input;
}

function requireWorkbook(): Workbook {
  if (!wb) {
    throw new Error("workbook is not open");
  }
  return wb;
}

function structure(): WorkbookStructure {
  const w = requireWorkbook();
  return {
    sheets: w.worksheets().map((s) => s.name),
    definedNames: w.definedNames.list(),
  };
}

function unquoteSheet(ref: string): string | null {
  const bang = ref.lastIndexOf("!");
  if (bang < 0) return null;
  let s = ref.slice(0, bang);
  if (s.startsWith("'") && s.endsWith("'")) s = s.slice(1, -1).replace(/''/g, "'");
  return s;
}

function wsFromRangeRef(w: Workbook, rangeRef: string): Worksheet {
  const name = unquoteSheet(rangeRef);
  const ws = name ? w.sheet(name) : w.activeSheet();
  if (!ws.autoFilter.get()) ws.autoFilter.set(rangeRef);
  return ws;
}

async function handleRequest(request: EditWorkerRequest): Promise<{
  result: unknown;
  transfer?: Transferable[];
}> {
  switch (request.op) {
    case "open": {
      const { bytes, wasmBinaryUrl } = request.args as {
        bytes: ArrayBuffer;
        wasmBinaryUrl: string;
      };
      wb?.dispose();
      wb = await Workbook.open(new Uint8Array(bytes), { wasmBinaryUrl });
      return { result: { layout: wb.layout({}), structure: structure() } };
    }
    case "applyEdit": {
      const { sheetName, address, input, recalc } = request.args as {
        sheetName: string;
        address: string;
        input: string;
        recalc: boolean;
      };
      const w = requireWorkbook();
      const cell = w.sheet(sheetName).cell(address);
      if (input.startsWith("=")) {
        cell.setFormula(input);
      } else {
        cell.setValue(coerce(input));
      }
      if (recalc) {
        w.recalculate();
      }
      return { result: { layout: w.layout({ sheetName }), structure: structure() } };
    }
    case "setRangeValues": {
      const { sheetName, ref, values, recalc } = request.args as {
        sheetName: string;
        ref: string;
        values: CellInput[][];
        recalc: boolean;
      };
      const w = requireWorkbook();
      w.sheet(sheetName).range(ref).setValues(values);
      if (recalc) {
        w.recalculate();
      }
      return { result: { layout: w.layout({ sheetName }), structure: structure() } };
    }
    case "setRangeFormulas": {
      const { sheetName, ref, formulas, recalc } = request.args as {
        sheetName: string;
        ref: string;
        formulas: Array<Array<string | null>>;
        recalc: boolean;
      };
      const w = requireWorkbook();
      w.sheet(sheetName).range(ref).setFormulas(formulas);
      if (recalc) {
        w.recalculate();
      }
      return { result: { layout: w.layout({ sheetName }), structure: structure() } };
    }
    case "copyRange": {
      const { sheetName, ref, destSheet, destRef, recalc } = request.args as {
        sheetName: string;
        ref: string;
        destSheet: string;
        destRef: string;
        recalc: boolean;
      };
      const w = requireWorkbook();
      w.sheet(sheetName).range(ref).copyTo(w.sheet(destSheet).range(destRef));
      if (recalc) {
        w.recalculate();
      }
      return { result: { layout: w.layout({ sheetName: destSheet }), structure: structure() } };
    }
    case "clearRange": {
      const { sheetName, ref, mode, recalc } = request.args as {
        sheetName: string;
        ref: string;
        mode?: ClearMode;
        recalc: boolean;
      };
      const w = requireWorkbook();
      w.sheet(sheetName).range(ref).clear(mode);
      if (recalc) {
        w.recalculate();
      }
      return { result: { layout: w.layout({ sheetName }), structure: structure() } };
    }
    case "setImage": {
      const { sheetName, patch, bytes } = request.args as {
        sheetName: string;
        patch: Omit<ImagePatch, "bytes">;
        bytes: ArrayBuffer;
      };
      const w = requireWorkbook();
      w.sheet(sheetName).images.set({ ...patch, bytes: new Uint8Array(bytes) });
      return { result: { layout: w.layout({ sheetName }), structure: structure() } };
    }
    case "addSheet": {
      const { name } = request.args as { name: string };
      const w = requireWorkbook();
      const ws = w.addSheet(name);
      return { result: { layout: w.layout({}), structure: structure(), name: ws.name } };
    }
    case "recalculate": {
      const w = requireWorkbook();
      w.recalculate();
      return { result: { layout: w.layout({}), structure: structure() } };
    }
    case "layout": {
      const { options } = request.args as { options?: WorkbookLayoutOptions };
      const w = requireWorkbook();
      return { result: { layout: w.layout(options ?? {}) } };
    }
    case "save": {
      const bytes = requireWorkbook().save();
      return { result: { bytes }, transfer: [bytes.buffer] };
    }
    case "pivotMetas": {
      const w = requireWorkbook();
      const pivots: PivotMeta[] = [];
      for (const ws of w.worksheets()) {
        for (const info of ws.pivots.list() as PivotInfo[]) {
          pivots.push({
            name: info.name,
            sheet: ws.name,
            id: info.id,
            sourceRef: info.sourceRef,
          });
        }
      }
      return { result: { pivots } };
    }
    case "distinctValues": {
      const { sourceRef, field } = request.args as { sourceRef: string; field: string };
      const w = requireWorkbook();
      return { result: { values: distinctValuesFor(w, sourceRef, field) } };
    }
    case "updatePivot": {
      const { sheet, id, patch } = request.args as {
        sheet: string;
        id: string;
        patch: PivotUpdate;
      };
      const w = requireWorkbook();
      w.sheet(sheet).pivots.update(id, patch);
      return { result: { layout: w.layout({}), structure: structure() } };
    }
    case "moveDrawing": {
      const { sheetName, kind, drawingIndex, anchor, prevAnchor } = request.args as {
        sheetName: string;
        kind: string;
        drawingIndex: number;
        anchor: ChartAnchor;
        prevAnchor: ChartAnchor;
      };
      const w = requireWorkbook();
      if (kind === "chart") {
        const id = resolveDrawingId(
          w.sheet(sheetName).charts.list() as ChartInfo[],
          prevAnchor,
          drawingIndex,
        );
        if (id) w.sheet(sheetName).charts.update(id, { anchor });
      } else if (kind === "image") {
        const id = resolveDrawingId(
          w.sheet(sheetName).images.list() as ImageInfo[],
          prevAnchor,
          drawingIndex,
        );
        if (id) w.sheet(sheetName).images.update(id, { anchor });
      }
      return { result: { layout: w.layout({ sheetName }), structure: structure() } };
    }
    case "removeDrawing": {
      const { sheetName, kind, drawingIndex, prevAnchor } = request.args as {
        sheetName: string;
        kind: string;
        drawingIndex: number;
        prevAnchor: ChartAnchor;
      };
      const w = requireWorkbook();
      if (kind === "chart") {
        const id = resolveDrawingId(
          w.sheet(sheetName).charts.list() as ChartInfo[],
          prevAnchor,
          drawingIndex,
        );
        if (id) w.sheet(sheetName).charts.remove(id);
      } else if (kind === "image") {
        const id = resolveDrawingId(
          w.sheet(sheetName).images.list() as ImageInfo[],
          prevAnchor,
          drawingIndex,
        );
        if (id) w.sheet(sheetName).images.remove(id);
      }
      return { result: { layout: w.layout({ sheetName }), structure: structure() } };
    }
    case "tableSetFilter": {
      const { rangeRef, columnOffset, field, values } = request.args as {
        rangeRef: string;
        columnOffset: number;
        field: string;
        values: string[];
      };
      const w = requireWorkbook();
      const ws = wsFromRangeRef(w, rangeRef);
      const all = distinctValuesFor(w, rangeRef, field);
      if (values.length === 0 || values.length >= all.length) {
        ws.autoFilter.removeColumn(columnOffset);
      } else {
        ws.autoFilter.setColumnValues(columnOffset, values);
      }
      return { result: { layout: w.layout({}), structure: structure() } };
    }
    case "tableSetSort": {
      const { rangeRef, columnOffset, descending } = request.args as {
        rangeRef: string;
        columnOffset: number;
        descending: boolean | null;
      };
      const w = requireWorkbook();
      const ws = wsFromRangeRef(w, rangeRef);
      if (descending === null) {
        ws.autoFilter.clearSort();
      } else {
        ws.autoFilter.setSort(columnOffset, { descending });
      }
      return { result: { layout: w.layout({}), structure: structure() } };
    }
  }
}

(globalThis as unknown as { onmessage: ((event: MessageEvent) => void) | null }).onmessage = async (
  event: MessageEvent,
) => {
  const request = event.data as EditWorkerRequest;
  try {
    const { result, transfer } = await handleRequest(request);
    post({ id: request.id, ok: true, result }, transfer);
  } catch (error) {
    post({ id: request.id, ok: false, error: xlsxLoadErrorPayloadFromUnknown(error) });
  }
};
