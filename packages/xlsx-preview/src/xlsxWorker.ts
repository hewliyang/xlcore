// Worker entry. Statically imports the wasm-bindgen shim so the bundler
// follows the chain to `xlcore_wasm_bg.wasm`. The main thread passes the
// final wasm binary URL via postMessage so we can bypass the shim's
// internal `new URL("./xlcore_wasm_bg.wasm", import.meta.url)` default —
// that default fails when bundlers copy the shim as a raw asset without
// recursing into it.
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
    const layout = extract_xlsx(new Uint8Array(bytes), undefined);
    post({ type: "layout", layout });
  } catch (error) {
    post({
      type: "error",
      message: error instanceof Error ? (error.stack ?? error.message) : String(error),
    });
  }
};
