export interface CdnAssetUrls {
  wasmBinaryUrl: string;
  workerUrl: string;
}

const PACKAGE = "@hewliyang/xlsx-preview";

export function jsDelivrUrls(version = "latest"): CdnAssetUrls {
  const base = `https://cdn.jsdelivr.net/npm/${PACKAGE}@${version}/dist/`;
  return {
    wasmBinaryUrl: `${base}xlcore_wasm_bg.wasm`,
    workerUrl: `${base}xlsxWorker.js`,
  };
}

export function unpkgUrls(version = "latest"): CdnAssetUrls {
  const base = `https://unpkg.com/${PACKAGE}@${version}/dist/`;
  return {
    wasmBinaryUrl: `${base}xlcore_wasm_bg.wasm`,
    workerUrl: `${base}xlsxWorker.js`,
  };
}
