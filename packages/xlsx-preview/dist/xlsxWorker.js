// src/xlsxWorker.ts
var wasmModulePromise = null;
function post(message) {
  globalThis.postMessage(message);
}
function stage(label) {
  post({ type: "stage", label });
}
globalThis.onmessage = async (event) => {
  try {
    const { bytes, wasmUrl } = event.data;
    stage("Loading WASM");
    wasmModulePromise ??= import(wasmUrl).then(async (mod2) => {
      await mod2.default();
      return mod2;
    });
    const mod = await wasmModulePromise;
    stage("Extracting OOXML");
    const layout = mod.extract_xlsx(new Uint8Array(bytes), undefined);
    post({ type: "layout", layout });
  } catch (error) {
    post({
      type: "error",
      message: error instanceof Error ? error.stack ?? error.message : String(error)
    });
  }
};
