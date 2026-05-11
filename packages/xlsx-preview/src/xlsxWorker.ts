let wasmModulePromise: Promise<{
  extract_xlsx(bytes: Uint8Array, options: unknown): unknown;
}> | null = null;

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
    const { bytes, wasmUrl } = event.data as { bytes: ArrayBuffer; wasmUrl: string };
    stage("Loading WASM");
    wasmModulePromise ??= import(wasmUrl).then(
      async (mod: { default: () => Promise<unknown>; extract_xlsx: unknown }) => {
        await mod.default();
        return mod as { extract_xlsx(bytes: Uint8Array, options: unknown): unknown };
      },
    );
    const mod = await wasmModulePromise;
    stage("Extracting OOXML");
    const layout = mod.extract_xlsx(new Uint8Array(bytes), undefined);
    post({ type: "layout", layout });
  } catch (error) {
    post({
      type: "error",
      message: error instanceof Error ? (error.stack ?? error.message) : String(error),
    });
  }
};
