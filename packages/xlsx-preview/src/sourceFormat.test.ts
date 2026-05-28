import { describe, expect, test } from "vitest";
import {
  detectWorkbookFormatFromBytes,
  detectWorkbookFormatFromHint,
  resolveWorkbookFormat,
} from "./sourceFormat.js";

describe("workbook source format detection", () => {
  test("parquet magic wins over misleading filename", () => {
    const bytes = new Uint8Array([0x50, 0x41, 0x52, 0x31, 1, 2, 0x50, 0x41, 0x52, 0x31]);
    expect(detectWorkbookFormatFromBytes(bytes)).toBe("parquet");
    expect(resolveWorkbookFormat("auto", bytes, { fileName: "data.csv" })).toBe("parquet");
  });

  test("zip magic is treated as xlsx", () => {
    expect(detectWorkbookFormatFromBytes(new Uint8Array([0x50, 0x4b, 0x03, 0x04]))).toBe("xlsx");
  });

  test("csv falls back to name and mime hints", () => {
    expect(detectWorkbookFormatFromHint({ fileName: "data.tsv" })).toBe("csv");
    expect(detectWorkbookFormatFromHint({ mimeType: "text/csv" })).toBe("csv");
    expect(detectWorkbookFormatFromHint({ mimeType: "text/csv; charset=utf-8" })).toBe("csv");
  });

  test("parquet falls back to common mime hints", () => {
    expect(detectWorkbookFormatFromHint({ fileName: "events.pqt" })).toBe("parquet");
    expect(detectWorkbookFormatFromHint({ mimeType: "application/vnd.apache.parquet" })).toBe(
      "parquet",
    );
    expect(detectWorkbookFormatFromHint({ mimeType: "application/x-parquet" })).toBe("parquet");
    expect(
      detectWorkbookFormatFromHint({
        mimeType: "application/vnd.apache.parquet; charset=binary",
      }),
    ).toBe("parquet");
  });

  test("explicit format wins and unknown auto defaults to xlsx", () => {
    expect(resolveWorkbookFormat("csv", new Uint8Array([0x50, 0x4b, 0x03, 0x04]))).toBe("csv");
    expect(resolveWorkbookFormat("auto", new Uint8Array([1, 2, 3]))).toBe("xlsx");
  });
});
