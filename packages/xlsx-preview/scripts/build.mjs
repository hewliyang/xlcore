#!/usr/bin/env node
import { build } from "esbuild";
import { cp } from "node:fs/promises";

const common = {
  bundle: true,
  sourcemap: false,
  logLevel: "info",
};

// Standalone IIFE for `xlcore preview` static-HTML bootstrap.
await build({
  ...common,
  entryPoints: ["src/browser.ts"],
  outfile: "dist/browser.js",
  platform: "browser",
  format: "iife",
  target: "es2022",
});

// Bundled ESM entries.
for (const [entry, outfile, platform, external = []] of [
  ["src/index.ts", "dist/index.js", "node", ["skia-canvas"]],
  ["src/cli.ts", "dist/cli.js", "node", ["skia-canvas"]],
  ["src/previewer.ts", "dist/previewer.js", "browser"],
  ["src/react.ts", "dist/react.js", "browser", ["react"]],
  ["src/cdn.ts", "dist/cdn.js", "browser"],
]) {
  await build({
    ...common,
    entryPoints: [entry],
    outfile,
    platform,
    format: "esm",
    target: "es2022",
    external,
  });
}

// Browser loader + worker: ship UN-bundled so that
//   new URL("./xlsxWorker.js", import.meta.url)
//   new URL("./xlcore_wasm_bg.wasm", import.meta.url)
// patterns survive verbatim into the published files. Vite, webpack 5,
// Rollup, Parcel, etc. all match these literal forms and emit the
// referenced assets. Pre-bundling defeats their static analysis.
await build({
  ...common,
  bundle: false,
  entryPoints: ["src/browserLoader.ts", "src/xlsxWorker.ts"],
  outdir: "dist",
  platform: "browser",
  format: "esm",
  target: "es2022",
});

// The wasm-bindgen shim + wasm binary live next to the worker so the
// `new URL(..., import.meta.url)` chain resolves at runtime AND so
// bundlers can follow it.
await cp("../../crates/xlcore-wasm/pkg/xlcore_wasm.js", "dist/xlcore_wasm.js");
await cp("../../crates/xlcore-wasm/pkg/xlcore_wasm.d.ts", "dist/xlcore_wasm.d.ts");
await cp("../../crates/xlcore-wasm/pkg/xlcore_wasm_bg.wasm", "dist/xlcore_wasm_bg.wasm");
await cp(
  "../../crates/xlcore-wasm/pkg/xlcore_wasm_bg.wasm.d.ts",
  "dist/xlcore_wasm_bg.wasm.d.ts",
);
