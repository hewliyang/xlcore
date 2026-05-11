#!/usr/bin/env node
import { build } from "esbuild";
import { cp } from "node:fs/promises";

const common = {
  bundle: true,
  sourcemap: false,
  logLevel: "info",
};

await build({
  ...common,
  entryPoints: ["src/browser.ts"],
  outfile: "dist/browser.js",
  platform: "browser",
  format: "iife",
  target: "es2022",
});

for (const [entry, outfile, platform, external = []] of [
  ["src/index.ts", "dist/index.js", "node", ["skia-canvas"]],
  ["src/cli.ts", "dist/cli.js", "node", ["skia-canvas"]],
  ["src/previewer.ts", "dist/previewer.js", "browser"],
  ["src/browserLoader.ts", "dist/browser-loader.js", "browser"],
  ["src/react.ts", "dist/react.js", "browser", ["react"]],
  ["src/xlsxWorker.ts", "dist/xlsxWorker.js", "browser"],
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

await cp("../../crates/xlcore-wasm/pkg/xlcore_wasm.js", "dist/xlcore_wasm.js");
await cp("../../crates/xlcore-wasm/pkg/xlcore_wasm_bg.wasm", "dist/xlcore_wasm_bg.wasm");
