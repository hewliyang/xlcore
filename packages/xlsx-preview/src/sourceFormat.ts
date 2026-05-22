export type WorkbookSourceFormat = "xlsx" | "csv" | "parquet";

export interface WorkbookFormatHint {
  fileName?: string;
  mimeType?: string;
}

export function detectWorkbookFormatFromBytes(
  bytes: ArrayBuffer | ArrayBufferView,
): WorkbookSourceFormat | undefined {
  const view =
    bytes instanceof ArrayBuffer
      ? new Uint8Array(bytes)
      : new Uint8Array(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  if (isParquet(view)) return "parquet";
  // XLSX is a ZIP container. This is a candidate classification; the OOXML
  // loader still validates that the ZIP actually contains workbook parts.
  if (isZipContainer(view)) return "xlsx";
  return undefined;
}

export function detectWorkbookFormatFromHint(
  hint: WorkbookFormatHint,
): WorkbookSourceFormat | undefined {
  const name = hint.fileName?.toLowerCase() ?? "";
  if (name.endsWith(".parquet") || name.endsWith(".pqt")) return "parquet";
  if (name.endsWith(".csv") || name.endsWith(".tsv") || name.endsWith(".txt")) return "csv";
  if (name.endsWith(".xlsx")) return "xlsx";

  const mime = normalizeMimeType(hint.mimeType);
  if (mime === "application/vnd.apache.parquet" || mime === "application/x-parquet")
    return "parquet";
  if (mime === "text/csv" || mime === "text/tab-separated-values") return "csv";
  if (mime === "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet") return "xlsx";
  return undefined;
}

export function resolveWorkbookFormat(
  explicit: WorkbookSourceFormat | "auto" | undefined,
  bytes: ArrayBuffer | ArrayBufferView | undefined,
  hint: WorkbookFormatHint = {},
): WorkbookSourceFormat {
  if (explicit && explicit !== "auto") return explicit;
  return (
    (bytes ? detectWorkbookFormatFromBytes(bytes) : undefined) ??
    detectWorkbookFormatFromHint(hint) ??
    "xlsx"
  );
}

function isParquet(bytes: Uint8Array): boolean {
  return (
    bytes.length >= 8 &&
    bytes[0] === 0x50 &&
    bytes[1] === 0x41 &&
    bytes[2] === 0x52 &&
    bytes[3] === 0x31 &&
    bytes[bytes.length - 4] === 0x50 &&
    bytes[bytes.length - 3] === 0x41 &&
    bytes[bytes.length - 2] === 0x52 &&
    bytes[bytes.length - 1] === 0x31
  );
}

function isZipContainer(bytes: Uint8Array): boolean {
  return (
    bytes.length >= 4 &&
    bytes[0] === 0x50 &&
    bytes[1] === 0x4b &&
    ((bytes[2] === 0x03 && bytes[3] === 0x04) ||
      (bytes[2] === 0x05 && bytes[3] === 0x06) ||
      (bytes[2] === 0x07 && bytes[3] === 0x08))
  );
}

function normalizeMimeType(mimeType: string | undefined): string {
  return mimeType?.split(";", 1)[0]?.trim().toLowerCase() ?? "";
}
