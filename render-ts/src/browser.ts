// Browser entry: exposes globals used by the preview HTML emitted by
// `xlcore preview`. The bootstrap in `crates/xlcore-cli/src/main.rs`
// gunzips the embedded layout JSON, then calls
// `xlcoreDecodeLayout(layout)` to inflate the columnar typed-array
// blobs into real Uint32Array/Int32Array views before any render call.
import { render } from "./render.js";
import { attachInteractivity } from "./interact.js";
import { decodeWorkbookLayout, iterRows } from "./columnar.js";

declare global {
  interface Window {
    xlcoreRender: typeof render;
    xlcoreAttachInteractivity: typeof attachInteractivity;
    xlcoreDecodeLayout: typeof decodeWorkbookLayout;
    xlcoreIterRows: typeof iterRows;
  }
}

const g = globalThis as unknown as {
  xlcoreRender?: typeof render;
  xlcoreAttachInteractivity?: typeof attachInteractivity;
  xlcoreDecodeLayout?: typeof decodeWorkbookLayout;
  xlcoreIterRows?: typeof iterRows;
};
g.xlcoreRender = render;
g.xlcoreAttachInteractivity = attachInteractivity;
g.xlcoreDecodeLayout = decodeWorkbookLayout;
g.xlcoreIterRows = iterRows;
