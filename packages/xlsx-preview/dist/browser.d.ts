import { render } from "./render.js";
import { attachInteractivity } from "./interact.js";
import { decodeWorkbookLayout, iterRows } from "./columnar.js";
import { createWorkbookPreviewer } from "./previewer.js";
declare global {
    interface Window {
        xlcoreRender: typeof render;
        xlcoreAttachInteractivity: typeof attachInteractivity;
        xlcoreDecodeLayout: typeof decodeWorkbookLayout;
        xlcoreIterRows: typeof iterRows;
        xlcoreCreatePreviewer: typeof createWorkbookPreviewer;
    }
}
