import { render } from "./render.js";
import { attachInteractivity } from "./interact.js";
import { decodeWorkbookLayout, iterRows } from "./columnar.js";
import { createWorkbookPreviewer } from "./previewer.js";
import { colorToCss, colorToCssWithTheme } from "./color.js";

declare global {
  interface Window {
    xlcoreRender: typeof render;
    xlcoreAttachInteractivity: typeof attachInteractivity;
    xlcoreDecodeLayout: typeof decodeWorkbookLayout;
    xlcoreIterRows: typeof iterRows;
    xlcoreCreatePreviewer: typeof createWorkbookPreviewer;
    xlcoreColorToCss: typeof colorToCss;
    xlcoreColorToCssWithTheme: typeof colorToCssWithTheme;
  }
}

const g = globalThis as unknown as {
  xlcoreRender?: typeof render;
  xlcoreAttachInteractivity?: typeof attachInteractivity;
  xlcoreDecodeLayout?: typeof decodeWorkbookLayout;
  xlcoreIterRows?: typeof iterRows;
  xlcoreCreatePreviewer?: typeof createWorkbookPreviewer;
  xlcoreColorToCss?: typeof colorToCss;
  xlcoreColorToCssWithTheme?: typeof colorToCssWithTheme;
};
g.xlcoreRender = render;
g.xlcoreAttachInteractivity = attachInteractivity;
g.xlcoreDecodeLayout = decodeWorkbookLayout;
g.xlcoreIterRows = iterRows;
g.xlcoreCreatePreviewer = createWorkbookPreviewer;
g.xlcoreColorToCss = colorToCss;
g.xlcoreColorToCssWithTheme = colorToCssWithTheme;
