# xlsx-preview · React + Vite

Minimal Vite + React + TypeScript starter using `@hewliyang/xlsx-preview/react`.

```bash
pnpm install
pnpm dev
```

Then open the printed URL and pick an `.xlsx` file.

## What this shows

- `<ExcelPreviewer file={file} />` is the entire integration.
- Vite picks up the worker (`xlsxWorker.js`) and wasm (`xlcore_wasm_bg.wasm`)
  automatically via the `new URL(..., import.meta.url)` pattern inside the
  package — no config required.
- `pnpm build` emits the worker as a hashed asset under `dist/assets/`.

## Files

| File | Why |
| --- | --- |
| `package.json` | Pulls `@hewliyang/xlsx-preview`, `react`, `react-dom` |
| `vite.config.ts` | Just `@vitejs/plugin-react` |
| `src/App.tsx` | File input + `<ExcelPreviewer />` |
