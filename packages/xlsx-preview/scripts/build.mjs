#!/usr/bin/env node
import { build } from "esbuild";
import { cp } from "node:fs/promises";

const common = {
  bundle: true,
  sourcemap: false,
  logLevel: "info",
};

// Standalone bundle used by `xlcore preview` HTML output.
await build({
  ...common,
  entryPoints: ["src/browser.ts"],
  outfile: "dist/browser.js",
  platform: "browser",
  format: "iife",
  target: "es2022",
});

// Public ESM entry points.
for (const [entry, outfile, platform, external = []] of [
  ["src/index.ts", "dist/index.js", "node", ["skia-canvas"]],
  ["src/cli.ts", "dist/cli.js", "node", ["skia-canvas"]],
  ["src/previewer.ts", "dist/previewer.js", "browser"],
  ["src/color.ts", "dist/color.js", "browser"],
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

// Keep loader and worker as separate ESM files. Bundlers can then discover
// the worker module and wasm binary from their `new URL(..., import.meta.url)`
// references instead of seeing one pre-bundled blob.
await build({
  ...common,
  bundle: false,
  entryPoints: ["src/browserLoader.ts", "src/xlsxWorker.ts"],
  outdir: "dist",
  platform: "browser",
  format: "esm",
  target: "es2022",
});

// Runtime assets used by the browser worker and Node entry.
await cp("../../crates/xlcore-wasm/pkg/xlcore_wasm.js", "dist/xlcore_wasm.js");
await cp("../../crates/xlcore-wasm/pkg/xlcore_wasm.d.ts", "dist/xlcore_wasm.d.ts");
await cp("../../crates/xlcore-wasm/pkg/xlcore_wasm_bg.wasm", "dist/xlcore_wasm_bg.wasm");
await cp(
  "../../crates/xlcore-wasm/pkg/xlcore_wasm_bg.wasm.d.ts",
  "dist/xlcore_wasm_bg.wasm.d.ts",
);
