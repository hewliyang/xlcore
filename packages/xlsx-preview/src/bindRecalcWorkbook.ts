import { cellA1, rangeA1 } from "./api-refs.js";
import type { CellInput } from "./api-range.js";
import type { ChartAnchor, ImageFormat, ImagePatch } from "./api-schema/index.js";
import type { Selection } from "./interact.js";
import type { PreviewerEventName, WorkbookPreviewer } from "./previewer.js";
import type { WorkbookLayout } from "./types.js";

export interface RecalcWorkbookLike {
  applyEdit(input: {
    sheetName: string;
    address: string;
    input: string;
    recalc: boolean;
  }): Promise<{ layout: WorkbookLayout }>;
  setRangeValues(input: {
    sheetName: string;
    ref: string;
    values: CellInput[][];
    recalc: boolean;
  }): Promise<{ layout: WorkbookLayout }>;
  clearRange(input: {
    sheetName: string;
    ref: string;
    recalc: boolean;
  }): Promise<{ layout: WorkbookLayout }>;
  pasteCells(input: {
    sheetName: string;
    row: number;
    column: number;
    values: string[][];
    recalc: boolean;
  }): Promise<{ layout: WorkbookLayout }>;
  copyRange(input: {
    sheetName: string;
    ref: string;
    destSheet: string;
    destRef: string;
    recalc: boolean;
  }): Promise<{ layout: WorkbookLayout }>;
  moveRange(input: {
    sheetName: string;
    ref: string;
    destSheet: string;
    destRef: string;
    recalc: boolean;
  }): Promise<{ layout: WorkbookLayout }>;
  setImage(sheetName: string, patch: ImagePatch): Promise<{ layout: WorkbookLayout }>;
  addSheet(name: string): Promise<{ layout: WorkbookLayout; name: string }>;
  moveDrawing(input: {
    sheetName: string;
    kind: string;
    drawingIndex: number;
    anchor: ChartAnchor;
    prevAnchor: ChartAnchor;
  }): Promise<WorkbookLayout>;
  removeDrawing(input: {
    sheetName: string;
    kind: string;
    drawingIndex: number;
    prevAnchor: ChartAnchor;
  }): Promise<WorkbookLayout>;
  layout(options?: { sheetName?: string }): Promise<WorkbookLayout>;
}

export interface BindRecalcWorkbookOptions {
  autoRecalc?: boolean | (() => boolean);
  resyncOnSheetChange?: boolean;
  imageAnchor?: { rows: number; cols: number };
  imageName?: string;
  onChange?: (info: { event: PreviewerEventName }) => void;
  onStatus?: (message: string) => void;
  onError?: (error: unknown) => void;
}

export interface RecalcWorkbookBinding {
  unbind(): void;
}

const IMAGE_MIME: Record<string, ImageFormat> = {
  "image/png": "png",
  "image/jpeg": "jpeg",
  "image/gif": "gif",
  "image/webp": "webp",
};

interface CellEditDetail {
  sheetIndex: number;
  r: number;
  c: number;
  input: string;
  commitMove: "down" | "right" | "up" | "left" | null;
}

interface FillDetail {
  target: Selection;
  values: CellInput[][];
}

interface ClearDetail {
  ref: string;
}

interface PasteDetail {
  target: { r: number; c: number };
  values: string[][];
  source?: string;
  sourceSheet?: string;
  sourceRange?: string;
  cutRange?: Selection | null;
}

interface ImagePasteDetail {
  bytes: Uint8Array;
  mime: string;
}

function normalize(sel: Selection): Selection {
  return {
    r1: Math.min(sel.r1, sel.r2),
    c1: Math.min(sel.c1, sel.c2),
    r2: Math.max(sel.r1, sel.r2),
    c2: Math.max(sel.c1, sel.c2),
  };
}

function selectionRef(sel: Selection): string {
  const n = normalize(sel);
  return rangeA1(n.r1, n.c1, n.r2 - n.r1 + 1, n.c2 - n.c1 + 1);
}

export function bindRecalcWorkbook(
  previewer: WorkbookPreviewer,
  workbook: RecalcWorkbookLike,
  options: BindRecalcWorkbookOptions = {},
): RecalcWorkbookBinding {
  const recalc = (): boolean => {
    const value = options.autoRecalc ?? true;
    return typeof value === "function" ? value() : value;
  };
  const span = options.imageAnchor ?? { rows: 10, cols: 5 };
  const report = (event: PreviewerEventName): void => options.onChange?.({ event });
  const status = (message: string): void => options.onStatus?.(message);
  const fail = (error: unknown): void => {
    if (options.onError) options.onError(error);
    else status(error instanceof Error ? error.message : String(error));
  };

  const onCellEdit = async (detail: CellEditDetail): Promise<void> => {
    const sheetName = previewer.layout.sheets[detail.sheetIndex]?.name;
    if (sheetName == null) return;
    const address = cellA1(detail.r, detail.c);
    try {
      const { layout } = await workbook.applyEdit({
        sheetName,
        address,
        input: detail.input,
        recalc: recalc(),
      });
      previewer.patchSheetLayout(layout);
      if (detail.commitMove) {
        let nr = detail.r;
        let nc = detail.c;
        if (detail.commitMove === "down") nr = detail.r + 1;
        else if (detail.commitMove === "up") nr = Math.max(1, detail.r - 1);
        else if (detail.commitMove === "right") nc = detail.c + 1;
        else if (detail.commitMove === "left") nc = Math.max(1, detail.c - 1);
        previewer.selectCell(nr, nc, { scroll: true });
      }
      report("celledit");
      status(`edited ${address}`);
    } catch (error) {
      fail(error);
    }
  };

  const onFill = async (detail: FillDetail): Promise<void> => {
    if (!detail.values?.length || !detail.values[0]?.length) return;
    const sheetName = previewer.getActiveSheet().name;
    const ref = selectionRef(detail.target);
    try {
      const { layout } = await workbook.setRangeValues({
        sheetName,
        ref,
        values: detail.values,
        recalc: recalc(),
      });
      previewer.patchSheetLayout(layout);
      previewer.selectRange(detail.target, { scroll: true });
      report("rangefill");
      status(`filled ${ref}`);
    } catch (error) {
      fail(error);
    }
  };

  const onClear = async (detail: ClearDetail): Promise<void> => {
    const sheetName = previewer.getActiveSheet().name;
    try {
      const { layout } = await workbook.clearRange({
        sheetName,
        ref: detail.ref,
        recalc: recalc(),
      });
      previewer.patchSheetLayout(layout);
      report("cellclear");
      status(`cleared ${detail.ref}`);
    } catch (error) {
      fail(error);
    }
  };

  const onPaste = async (detail: PasteDetail): Promise<void> => {
    if (!detail.values?.length || !detail.values[0]?.length) return;
    const rows = detail.values.length;
    const cols = detail.values[0].length;
    const r2 = detail.target.r + rows - 1;
    const c2 = detail.target.c + cols - 1;
    const destSel: Selection = { r1: detail.target.r, c1: detail.target.c, r2, c2 };
    const destRef = selectionRef(destSel);
    const sheetName = previewer.getActiveSheet().name;
    try {
      let layout: WorkbookLayout;
      if (detail.source === "internal" && detail.sourceSheet && detail.sourceRange && detail.cutRange) {
        ({ layout } = await workbook.moveRange({
          sheetName: detail.sourceSheet,
          ref: detail.sourceRange,
          destSheet: sheetName,
          destRef,
          recalc: recalc(),
        }));
      } else if (detail.source === "internal" && detail.sourceSheet && detail.sourceRange) {
        ({ layout } = await workbook.copyRange({
          sheetName: detail.sourceSheet,
          ref: detail.sourceRange,
          destSheet: sheetName,
          destRef,
          recalc: recalc(),
        }));
      } else {
        ({ layout } = await workbook.pasteCells({
          sheetName,
          row: detail.target.r,
          column: detail.target.c,
          values: detail.values,
          recalc: recalc(),
        }));
      }
      previewer.patchSheetLayout(layout);
      previewer.selectRange(destSel, { scroll: true });
      report("rangepaste");
      status(`pasted ${destRef}`);
    } catch (error) {
      fail(error);
    }
  };

  const onImagePaste = async (detail: ImagePasteDetail): Promise<void> => {
    const format = IMAGE_MIME[detail.mime];
    if (!format) {
      fail(new Error(`unsupported image type: ${detail.mime}`));
      return;
    }
    const sheetName = previewer.getActiveSheet().name;
    const active = previewer.getActiveCell();
    const anchor = rangeA1(active.r, active.c, span.rows, span.cols);
    try {
      const { layout } = await workbook.setImage(sheetName, {
        anchor,
        bytes: detail.bytes,
        format,
        name: options.imageName ?? "pasted-image.png",
      });
      previewer.patchSheetLayout(layout);
      report("imagepaste");
      status("inserted image");
    } catch (error) {
      fail(error);
    }
  };

  const onSheetAdd = async (detail: { name: string }): Promise<void> => {
    try {
      const { layout, name } = await workbook.addSheet(detail.name);
      previewer.replaceLayout(layout);
      previewer.setActiveSheet(name);
      report("sheetadd");
      status(`added sheet ${name}`);
    } catch (error) {
      fail(error);
    }
  };

  const onDrawingMoved = async (detail: {
    sheetName: string;
    kind: string;
    drawingIndex: number;
    anchor: ChartAnchor;
    prevAnchor: ChartAnchor;
  }): Promise<void> => {
    try {
      const layout = await workbook.moveDrawing(detail);
      previewer.patchSheetLayout(layout);
      report("drawingmoved");
      status("moved drawing");
    } catch (error) {
      fail(error);
    }
  };

  const onDrawingDeleted = async (detail: {
    sheetName: string;
    kind: string;
    drawingIndex: number;
    prevAnchor: ChartAnchor;
  }): Promise<void> => {
    try {
      const layout = await workbook.removeDrawing(detail);
      previewer.patchSheetLayout(layout);
      report("drawingdeleted");
      status("deleted drawing");
    } catch (error) {
      fail(error);
    }
  };

  const onSheetChange = async (): Promise<void> => {
    if (options.resyncOnSheetChange === false) return;
    const name = previewer.getActiveSheet().name;
    try {
      const layout = await workbook.layout({ sheetName: name });
      previewer.patchSheetLayout(layout);
    } catch {}
  };

  const listeners: Array<[PreviewerEventName, EventListener]> = [
    ["celledit", (e) => void onCellEdit((e as CustomEvent).detail)],
    ["rangefill", (e) => void onFill((e as CustomEvent).detail)],
    ["cellclear", (e) => void onClear((e as CustomEvent).detail)],
    ["rangepaste", (e) => void onPaste((e as CustomEvent).detail)],
    ["imagepaste", (e) => void onImagePaste((e as CustomEvent).detail)],
    ["sheetadd", (e) => void onSheetAdd((e as CustomEvent).detail)],
    ["drawingmoved", (e) => void onDrawingMoved((e as CustomEvent).detail)],
    ["drawingdeleted", (e) => void onDrawingDeleted((e as CustomEvent).detail)],
    ["sheetchange", () => void onSheetChange()],
  ];

  for (const [name, listener] of listeners) previewer.on(name, listener);

  return {
    unbind(): void {
      for (const [name, listener] of listeners) previewer.off(name, listener);
    },
  };
}
