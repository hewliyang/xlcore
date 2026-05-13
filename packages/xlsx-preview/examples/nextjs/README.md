# xlsx-preview · Next.js

Minimal Next.js (App Router) starter using `@hewliyang/xlsx-preview/react`.

```bash
pnpm install
pnpm dev
```

Then open <http://localhost:3000> and pick an `.xlsx` file.

## What this shows

- `<ExcelPreviewer file={file} />` inside a client component.
- The first line of `app/page.tsx` is `"use client"` — required because the
  previewer uses React hooks, browser APIs (`File`, `Worker`, `URL`), and
  cannot run on the server.
- Next.js / webpack 5 / Turbopack pick up the worker and wasm assets
  automatically via the `new URL(..., import.meta.url)` pattern inside the
  package — no config required.
- `pnpm build` produces a working production build (`next start`).

## Files

| File | Why |
| --- | --- |
| `app/layout.tsx` | Root layout |
| `app/page.tsx` | `"use client"` page with file input + `<ExcelPreviewer />` |
| `next.config.ts` | Empty — no extra config needed |
