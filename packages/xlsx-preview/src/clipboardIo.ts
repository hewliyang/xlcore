import type { SerializedRange } from "./clipboardModel.js";

export async function writeClipboard({ tsv, html }: SerializedRange): Promise<void> {
  try {
    const item = new ClipboardItem({
      "text/plain": new Blob([tsv], { type: "text/plain" }),
      "text/html": new Blob([html], { type: "text/html" }),
    });
    await navigator.clipboard.write([item]);
  } catch {
    await navigator.clipboard.writeText(tsv);
  }
}
