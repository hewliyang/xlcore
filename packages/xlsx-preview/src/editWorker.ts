import type { CellInput } from "./api-range.js";
import type {
  DefinedNameInfo,
  SheetInfo,
  LayoutOptions as WorkbookLayoutOptions,
} from "./api-schema/index.js";
import { xlsxLoadErrorPayloadFromUnknown } from "./errors.js";
import init, { WorkbookHandle as WasmWorkbookHandle } from "./xlcore_wasm.js";

export interface WorkbookStructure {
  sheets: string[];
  definedNames: DefinedNameInfo[];
}

export type EditWorkerOp = "open" | "applyEdit" | "recalculate" | "layout" | "save";

export interface EditWorkerRequest {
  id: number;
  op: EditWorkerOp;
  args: Record<string, unknown>;
}

export type EditWorkerResponse =
  | { id: number; ok: true; result: unknown }
  | { id: number; ok: false; error: ReturnType<typeof xlsxLoadErrorPayloadFromUnknown> };

let wasmReady: Promise<void> | null = null;
let handle: WasmWorkbookHandle | null = null;

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

function requireHandle(): WasmWorkbookHandle {
  if (!handle) {
    throw new Error("workbook is not open");
  }
  return handle;
}

function structure(): WorkbookStructure {
  const h = requireHandle();
  return {
    sheets: (h.sheets() as SheetInfo[]).map((s) => s.name),
    definedNames: h.definedNames() as DefinedNameInfo[],
  };
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
      wasmReady ??= init({ module_or_path: wasmBinaryUrl }).then(() => undefined);
      await wasmReady;
      handle?.dispose();
      handle = WasmWorkbookHandle.open(new Uint8Array(bytes));
      return { result: { layout: handle.layout({}), structure: structure() } };
    }
    case "applyEdit": {
      const { sheetName, address, input, recalc } = request.args as {
        sheetName: string;
        address: string;
        input: string;
        recalc: boolean;
      };
      const h = requireHandle();
      if (input.startsWith("=")) {
        h.setFormula(sheetName, address, input);
      } else {
        h.setValue(sheetName, address, coerce(input));
      }
      if (recalc) {
        h.recalculate(true);
      }
      return { result: { layout: h.layout({}), structure: structure() } };
    }
    case "recalculate": {
      const h = requireHandle();
      h.recalculate(true);
      return { result: { layout: h.layout({}), structure: structure() } };
    }
    case "layout": {
      const { options } = request.args as { options?: WorkbookLayoutOptions };
      const h = requireHandle();
      return { result: { layout: h.layout(options ?? {}) } };
    }
    case "save": {
      const bytes = requireHandle().save();
      return { result: { bytes }, transfer: [bytes.buffer] };
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
