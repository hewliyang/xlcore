// Sibling declaration so `import init, { extract_xlsx } from "./xlcore_wasm.js"`
// in xlsxWorker.ts typechecks against the wasm-bindgen pkg. At build time
// the actual .js + .wasm are copied into `dist/` by scripts/build.mjs.
export * from "xlcore-wasm";
export { default } from "xlcore-wasm";
