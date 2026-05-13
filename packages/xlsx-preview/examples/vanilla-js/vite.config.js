import { defineConfig } from "vite";

export default defineConfig({
  // The previewer constructs its worker via `new URL("./xlsxWorker.js",
  // import.meta.url)`. Vite's dep optimizer doesn't re-emit that asset,
  // so we keep the package un-bundled in dev.
  optimizeDeps: { exclude: ["@hewliyang/xlsx-preview"] },
});
