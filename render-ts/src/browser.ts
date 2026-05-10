// Browser entry: exposes a single global, `window.xlcoreRender`, used by the
// preview HTML emitted by `xlcore preview`.
import { render } from "./render.js";
import { attachInteractivity } from "./interact.js";

declare global {
  interface Window {
    xlcoreRender: typeof render;
    xlcoreAttachInteractivity: typeof attachInteractivity;
  }
}

const g = globalThis as unknown as {
  xlcoreRender?: typeof render;
  xlcoreAttachInteractivity?: typeof attachInteractivity;
};
g.xlcoreRender = render;
g.xlcoreAttachInteractivity = attachInteractivity;
