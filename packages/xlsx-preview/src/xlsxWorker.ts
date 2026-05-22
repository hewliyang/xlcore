import type { CsvLoadOptions, ParquetLoadOptions, WorkbookLoaderOptions } from "./browserLoader.js";
import { xlsxLoadErrorPayloadFromUnknown, type LoadReport } from "./errors.js";
import type { WorkbookSourceFormat } from "./sourceFormat.js";
import type { WorkbookLayout } from "./types.js";
import init, { extract_csv, extract_parquet, extract_xlsx } from "./xlcore_wasm.js";

let wasmReady: Promise<void> | null = null;

function post(message: unknown): void {
  (globalThis as unknown as { postMessage(message: unknown): void }).postMessage(message);
}

function stage(label: string): void {
  post({ type: "stage", label });
}

type WorkerInput = {
  bytes: ArrayBuffer;
  wasmBinaryUrl: string;
  format?: WorkbookSourceFormat;
  xlsxOptions?: Pick<WorkbookLoaderOptions, "sheetIndex" | "sheetName">;
  csvOptions?: CsvLoadOptions;
  parquetOptions?: ParquetLoadOptions;
};

(globalThis as unknown as { onmessage: ((event: MessageEvent) => void) | null }).onmessage = async (
  event: MessageEvent,
) => {
  try {
    const { bytes, wasmBinaryUrl, format, xlsxOptions, csvOptions, parquetOptions } =
      event.data as WorkerInput;
    stage("Loading WASM");
    wasmReady ??= init({ module_or_path: wasmBinaryUrl }).then(() => undefined);
    await wasmReady;

    let envelope: { layout: WorkbookLayout; report: LoadReport };
    if (format === "csv") {
      stage("Parsing CSV");
      envelope = extract_csv(new Uint8Array(bytes), csvOptions) as {
        layout: WorkbookLayout;
        report: LoadReport;
      };
    } else if (format === "parquet") {
      stage("Parsing Parquet");
      envelope = extract_parquet(new Uint8Array(bytes), parquetOptions) as {
        layout: WorkbookLayout;
        report: LoadReport;
      };
    } else {
      stage("Extracting OOXML");
      envelope = extract_xlsx(new Uint8Array(bytes), xlsxOptions) as {
        layout: WorkbookLayout;
        report: LoadReport;
      };
    }
    post({ type: "loaded", layout: envelope.layout, report: envelope.report });
  } catch (error) {
    post({ type: "error", payload: xlsxLoadErrorPayloadFromUnknown(error) });
  }
};
