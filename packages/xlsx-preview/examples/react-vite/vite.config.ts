import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  // The previewer constructs its worker via `new URL("./xlsxWorker.js",
  // import.meta.url)`. Vite's dep optimizer doesn't re-emit that asset,
  // so we keep the package un-bundled in dev.
  optimizeDeps: { exclude: ["@hewliyang/xlsx-preview"] },
});
