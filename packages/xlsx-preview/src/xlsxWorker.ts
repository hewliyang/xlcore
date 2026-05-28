import { xlsxLoadErrorPayloadFromUnknown, type LoadReport } from "./errors.js";
import type { WorkbookLayout } from "./types.js";
import init, { extract_xlsx } from "./xlcore_wasm.js";

let wasmReady: Promise<void> | null = null;

function post(message: unknown): void {
  (globalThis as unknown as { postMessage(message: unknown): void }).postMessage(message);
}

function stage(label: string): void {
  post({ type: "stage", label });
}

(globalThis as unknown as { onmessage: ((event: MessageEvent) => void) | null }).onmessage = async (
  event: MessageEvent,
) => {
  try {
    const { bytes, wasmBinaryUrl } = event.data as {
      bytes: ArrayBuffer;
      wasmBinaryUrl: string;
    };
    stage("Loading WASM");
    wasmReady ??= init({ module_or_path: wasmBinaryUrl }).then(() => undefined);
    await wasmReady;
    stage("Extracting OOXML");

    const envelope = extract_xlsx(new Uint8Array(bytes), undefined) as {
      layout: WorkbookLayout;
      report: LoadReport;
    };
    post({ type: "loaded", layout: envelope.layout, report: envelope.report });
  } catch (error) {
    post({ type: "error", payload: xlsxLoadErrorPayloadFromUnknown(error) });
  }
};
