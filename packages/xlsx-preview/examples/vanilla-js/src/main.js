import { createWorkbookPreviewerFromFile } from "@hewliyang/xlsx-preview/browser";

const container = document.getElementById("preview");
const picker = document.getElementById("picker");

/** @type {import("@hewliyang/xlsx-preview/previewer").WorkbookPreviewer | null} */
let previewer = null;

picker.addEventListener("change", async (event) => {
  const file = event.target.files?.[0];
  if (!file) return;
  previewer?.destroy();
  previewer = await createWorkbookPreviewerFromFile(container, file);
});
