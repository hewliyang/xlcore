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
  ["src/index.ts", "dist/index.js", "browser", ["./xlcore_wasm.js", "node:fs"]],
  ["src/node.ts", "dist/node.js", "node", ["skia-canvas", "./api.js"]],
  ["src/cli.ts", "dist/cli.js", "node", ["skia-canvas", "./api.js"]],
  ["src/previewer.ts", "dist/previewer.js", "browser"],
  ["src/color.ts", "dist/color.js", "browser"],
  ["src/react.tsx", "dist/react.js", "browser", ["react", "react/jsx-runtime"]],
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

await build({
  ...common,
  bundle: false,

  entryPoints: [
    "src/browserLoader.ts",
    "src/xlsxWorker.ts",
    "src/editWorker.ts",
    "src/worker.ts",
    "src/api.ts",
    "src/api-refs.ts",
    "src/api-range.ts",
    "src/api-worksheet.ts",
    "src/api-collections.ts",
    "src/pivotSource.ts",
    "src/number-formats.ts",
    "src/errors.ts",
    "src/sourceFormat.ts",
  ],
  outdir: "dist",
  platform: "browser",
  format: "esm",
  target: "es2022",
});

await cp("../../crates/xlcore-wasm/pkg/xlcore_wasm.js", "dist/xlcore_wasm.js");
await cp("../../crates/xlcore-wasm/pkg/xlcore_wasm.d.ts", "dist/xlcore_wasm.d.ts");
await cp("../../crates/xlcore-wasm/pkg/xlcore_wasm_bg.wasm", "dist/xlcore_wasm_bg.wasm");
await cp(
  "../../crates/xlcore-wasm/pkg/xlcore_wasm_bg.wasm.d.ts",
  "dist/xlcore_wasm_bg.wasm.d.ts",
);

// Vitest transpiles from src/, so `import.meta.url` inside `node.ts` resolves
// to `src/xlcore_wasm_bg.wasm`. Stage the binary there so the same `node.ts`
// works for both bundled (dist) and source-based (tests) loads.
await cp(
  "../../crates/xlcore-wasm/pkg/xlcore_wasm_bg.wasm",
  "src/xlcore_wasm_bg.wasm",
);
