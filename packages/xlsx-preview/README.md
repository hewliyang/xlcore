# @hewliyang/xlsx-preview

XLSX preview/rendering library for browser, React, and Node.

```ts
import { createWorkbookPreviewerFromFile } from "@hewliyang/xlsx-preview/browser";

await createWorkbookPreviewerFromFile(container, file);
```

The Rust crates in this repository are implementation details for extraction/WASM for now; the intended public package is this npm package.
