// Helpers for loading the wasm + worker from a public CDN. Useful when
// you can't (or don't want to) emit them through your bundler — e.g.
// in a plain `<script type="module">` page, a CodePen, an Observable
// notebook, or behind a build pipeline that doesn't follow
// `new URL(..., import.meta.url)`.
//
//   import { ExcelPreviewer } from "@hewliyang/xlsx-preview/react";
//   import { jsDelivrUrls } from "@hewliyang/xlsx-preview/cdn";
//   <ExcelPreviewer file={file} {...jsDelivrUrls()} />
//
// Pin the version explicitly in production to avoid surprises.

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
