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

const IMAGE_TYPES = ["image/png", "image/jpeg", "image/gif", "image/webp"];

export async function readClipboard(): Promise<{
  html?: string;
  tsv?: string;
  imageBytes?: Uint8Array;
  imageType?: string;
}> {
  try {
    const items = await navigator.clipboard.read();
    let html: string | undefined;
    let tsv: string | undefined;
    let imageBytes: Uint8Array | undefined;
    let imageType: string | undefined;
    for (const item of items) {
      if (item.types.includes("text/html")) html = await (await item.getType("text/html")).text();
      if (item.types.includes("text/plain")) tsv = await (await item.getType("text/plain")).text();
      const imgType = IMAGE_TYPES.find((t) => item.types.includes(t));
      if (imgType && !imageBytes) {
        imageBytes = new Uint8Array(await (await item.getType(imgType)).arrayBuffer());
        imageType = imgType;
      }
    }
    if (html || tsv || imageBytes) return { html, tsv, imageBytes, imageType };
  } catch {
    // fall through
  }
  try {
    return { tsv: await navigator.clipboard.readText() };
  } catch {
    return {};
  }
}
