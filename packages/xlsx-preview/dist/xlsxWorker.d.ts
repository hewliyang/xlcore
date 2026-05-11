declare let wasmModulePromise: Promise<{
    extract_xlsx(bytes: Uint8Array, options: unknown): unknown;
}> | null;
declare function post(message: unknown): void;
declare function stage(label: string): void;
