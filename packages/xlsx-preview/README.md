# @hewliyang/xlsx-preview

Workbook preview/rendering library for browser, React, and Node. Supports
XLSX extraction plus unstyled CSV and Parquet previews through the same
`WorkbookLayout` renderer.

## Browser

```ts
import { createWorkbookPreviewerFromFile } from "@hewliyang/xlsx-preview/browser";

await createWorkbookPreviewerFromFile(container, file);
```

`createWorkbookPreviewerFromFile` sniffs `File.name` / `File.type` for
`.xlsx`, `.csv`, `.tsv`, `.txt`, `.parquet`, and `.pqt`. You can override the
format and pass tabular options:

```ts
await createWorkbookPreviewerFromFile(container, file, {
  format: "csv",
  csvOptions: { delimiter: "tab", maxRows: 50_000 },
});
```

Use `@hewliyang/xlsx-preview/react` for the React component wrapper and
`@hewliyang/xlsx-preview/previewer` for the imperative previewer API.

## Node

```ts
import {
  loadWorkbookFromCsv,
  loadWorkbookFromParquet,
  loadWorkbookFromXlsx,
  renderToPng,
} from "@hewliyang/xlsx-preview/node";
import { writeFile } from "node:fs/promises";

const layout = await loadWorkbookFromXlsx("workbook.xlsx", { sheetName: "Cover" });
await writeFile("cover.png", await renderToPng(layout, { scale: 2 }));

const csvLayout = await loadWorkbookFromCsv("sales.csv", { maxRows: 10_000 });
const parquetLayout = await loadWorkbookFromParquet("events.parquet");
```

Node rendering uses `skia-canvas`, so it is intended for server-side PNG
generation and CLI workflows.

## CLI

```bash
xlsx-preview workbook.xlsx --output sheet.png --sheet Cover --scale 2
xlsx-preview data.csv --output data.png --delimiter ","
xlsx-preview events.parquet --output events.png --max-rows 5000
xlsx-preview workbook.xlsx --info
```

The CLI sniffs file signatures first, then falls back to extension hints, and
accepts `--format xlsx|csv|parquet` when the hints are wrong.

The Rust crates in this repository are implementation details for extraction/WASM for now; the intended public package is this npm package.
