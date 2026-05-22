import {
  useEffect,
  useImperativeHandle,
  useRef,
  useState,
  type CSSProperties,
  type ReactNode,
} from "react";
import type * as React from "react";
import {
  createWorkbookPreviewerFromFile,
  type CreateWorkbookPreviewerFromFileOptions,
  type WorkbookLoadProgress,
} from "./browserLoader.js";
import {
  EMPTY_LOAD_REPORT,
  type FixedAttribute,
  type LoadReport,
  XlsxLoadError,
  reportIsClean,
} from "./errors.js";
import type { PreviewerState, WorkbookPreviewer } from "./previewer.js";

export interface UseWorkbookPreviewerOptions extends CreateWorkbookPreviewerFromFileOptions {
  onReady?: (previewer: WorkbookPreviewer) => void;
  onError?: (error: XlsxLoadError) => void;
  onSelectionChange?: (state: PreviewerState) => void;
  onSheetChange?: (state: PreviewerState) => void;
  onZoomChange?: (state: PreviewerState) => void;
}

export interface UseWorkbookPreviewerResult {
  containerRef: React.RefObject<HTMLDivElement | null>;
  previewer: WorkbookPreviewer | null;
  progress: WorkbookLoadProgress | null;
  error: XlsxLoadError | null;
  report: LoadReport | null;
  loading: boolean;
}

export interface ExcelPreviewerProps
  extends Omit<React.HTMLAttributes<HTMLDivElement>, "children" | "onError" | "onProgress">,
    UseWorkbookPreviewerOptions {
  file: Blob | null | undefined;
  fileName?: string;
  previewerRef?: React.Ref<WorkbookPreviewer | null>;
  renderError?: (info: { error: XlsxLoadError; fileName?: string }) => ReactNode;
  hideErrorUI?: boolean;
  showLeniencyChip?: boolean;
}

export function useWorkbookPreviewer(
  file: Blob | null | undefined,
  options: UseWorkbookPreviewerOptions = {},
): UseWorkbookPreviewerResult {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const [previewer, setPreviewer] = useState<WorkbookPreviewer | null>(null);
  const [progress, setProgress] = useState<WorkbookLoadProgress | null>(null);
  const [error, setError] = useState<XlsxLoadError | null>(null);
  const [report, setReport] = useState<LoadReport | null>(null);
  const [loading, setLoading] = useState(false);
  const optionsRef = useRef(options);
  optionsRef.current = options;
  const loadKey = workbookLoadKey(options);

  useEffect(() => {
    const container = containerRef.current;
    if (!container || !file) {
      setPreviewer(null);
      setProgress(null);
      setError(null);
      setReport(null);
      setLoading(false);
      return;
    }

    let cancelled = false;
    let activePreviewer: WorkbookPreviewer | null = null;
    setPreviewer(null);
    setProgress(null);
    setError(null);
    setReport(null);
    setLoading(true);

    const onProgress = (next: WorkbookLoadProgress) => {
      if (!cancelled) {
        setProgress(next);
        optionsRef.current.onProgress?.(next);
      }
    };

    createWorkbookPreviewerFromFile(container, file, {
      ...optionsRef.current,
      onProgress,
    })
      .then((next) => {
        if (cancelled) {
          next.destroy();
          return;
        }
        activePreviewer = next;
        attachPreviewerEvents(next, optionsRef);
        setPreviewer(next);
        setReport(next.report ?? EMPTY_LOAD_REPORT);
        setLoading(false);
        optionsRef.current.onReady?.(next);
      })
      .catch((reason: unknown) => {
        if (cancelled) return;
        const nextError = XlsxLoadError.fromUnknown(reason);
        setError(nextError);
        setLoading(false);
        optionsRef.current.onError?.(nextError);
      });

    return () => {
      cancelled = true;
      activePreviewer?.destroy();
    };
  }, [file, loadKey]);

  return { containerRef, previewer, progress, error, report, loading };
}

function workbookLoadKey(options: UseWorkbookPreviewerOptions): string {
  return JSON.stringify({
    wasmBinaryUrl: options.wasmBinaryUrl,
    workerUrl: options.workerUrl,
    sheetIndex: options.sheetIndex,
    sheetName: options.sheetName,
    format: options.format,
    csvOptions: options.csvOptions,
    parquetOptions: options.parquetOptions,
    initialSheet: options.initialSheet,
    initialZoom: options.initialZoom,
    showHidden: options.showHidden,
  });
}

export function ExcelPreviewer({
  file,
  fileName,
  previewerRef,
  className,
  style,
  wasmBinaryUrl,
  workerUrl,
  sheetIndex,
  sheetName,
  format,
  csvOptions,
  parquetOptions,
  initialSheet,
  initialZoom,
  onProgress,
  onReady,
  onError,
  onSelectionChange,
  onSheetChange,
  onZoomChange,
  renderError,
  hideErrorUI = false,
  showLeniencyChip = true,
  ...divProps
}: ExcelPreviewerProps): React.ReactElement {
  const result = useWorkbookPreviewer(file, {
    wasmBinaryUrl,
    workerUrl,
    sheetIndex,
    sheetName,
    format,
    csvOptions,
    parquetOptions,
    initialSheet,
    initialZoom,
    onProgress,
    onReady,
    onError,
    onSelectionChange,
    onSheetChange,
    onZoomChange,
  });

  useImperativeHandle<WorkbookPreviewer | null, WorkbookPreviewer | null>(
    previewerRef,
    () => result.previewer,
    [result.previewer],
  );

  const resolvedFileName = fileName ?? getBlobName(file);
  const errorNode =
    result.error && !hideErrorUI
      ? (renderError?.({ error: result.error, fileName: resolvedFileName }) ?? (
          <DefaultErrorCard
            error={result.error}
            fileName={resolvedFileName}
            fileSize={file?.size}
          />
        ))
      : null;
  const chipNode =
    !result.error && showLeniencyChip && !reportIsClean(result.report) ? (
      <LeniencyChip report={result.report!} />
    ) : null;

  return (
    <div
      {...divProps}
      className={className}
      style={{
        width: "100%",
        height: "100%",
        minHeight: 0,
        position: "relative",
        ...style,
      }}
    >
      <div ref={result.containerRef} style={{ width: "100%", height: "100%", minHeight: 0 }} />
      {errorNode}
      {chipNode}
    </div>
  );
}

function attachPreviewerEvents(
  previewer: WorkbookPreviewer,
  optionsRef: React.MutableRefObject<UseWorkbookPreviewerOptions>,
): void {
  previewer.on("selectionchange", (event) => {
    optionsRef.current.onSelectionChange?.((event as CustomEvent<PreviewerState>).detail);
  });
  previewer.on("sheetchange", (event) => {
    optionsRef.current.onSheetChange?.((event as CustomEvent<PreviewerState>).detail);
  });
  previewer.on("zoomchange", (event) => {
    optionsRef.current.onZoomChange?.((event as CustomEvent<PreviewerState>).detail);
  });
}

function getBlobName(file: Blob | null | undefined): string | undefined {
  return file && "name" in file && typeof file.name === "string" ? file.name : undefined;
}

function DefaultErrorCard({
  error,
  fileName,
  fileSize,
}: {
  error: XlsxLoadError;
  fileName?: string;
  fileSize?: number;
}): React.ReactElement {
  const [copied, setCopied] = useState(false);
  const copy = errorCopy(error);
  const fileLine = [fileName, fileSize ? formatBytes(fileSize) : null].filter(Boolean).join(" · ");

  const copyDiagnostics = () => {
    if (typeof navigator !== "undefined") {
      void navigator.clipboard?.writeText(error.diagnosticsText());
    }
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1500);
  };

  return (
    <div style={ERROR_CARD_OUTER_STYLE} role="alert">
      <div style={ERROR_CARD_STYLE}>
        <div style={ERROR_HEADER_STYLE}>
          <span style={ERROR_ICON_STYLE} aria-hidden="true">
            ⚠
          </span>
          <h2 style={ERROR_TITLE_STYLE}>{copy.headline}</h2>
        </div>
        {fileLine ? <div style={ERROR_META_STYLE}>{fileLine}</div> : null}
        <p style={ERROR_BODY_STYLE}>{copy.body}</p>
        {copy.detail ? <pre style={ERROR_DETAIL_STYLE}>{copy.detail}</pre> : null}
        <div style={ERROR_ACTIONS_STYLE}>
          <button type="button" onClick={copyDiagnostics} style={BUTTON_STYLE}>
            {copied ? "Copied" : "Copy diagnostics"}
          </button>
        </div>
      </div>
    </div>
  );
}

function errorCopy(err: XlsxLoadError): {
  headline: string;
  body: string;
  detail?: string;
} {
  if (err.code === "Zip") {
    return {
      headline: "Couldn't open this workbook",
      body: "The file is not a valid XLSX archive. If this is CSV or Parquet, pass the matching format option or use a matching file extension.",
      detail: err.message,
    };
  }
  if (err.code === "Schema") {
    const where = err.part && err.part !== "<unknown>" ? err.part : "the workbook";
    const attr = err.field ? `${err.field}=${JSON.stringify(err.value ?? "")}` : err.value;
    const detail =
      err.ty || attr !== undefined ? `<${err.ty ?? "?"}> ${attr ?? ""}\nin ${where}` : err.message;
    return {
      headline: "Couldn't open this workbook",
      body: "A value in the file doesn't match the Excel file format and couldn't be recovered automatically.",
      detail,
    };
  }
  if (err.code === "MissingPart") {
    return {
      headline: "Workbook is incomplete",
      body: `A required part is missing: ${err.part ?? "(unknown)"}.`,
    };
  }
  if (err.code === "Io") {
    return {
      headline: "Couldn't read the file",
      body: "An I/O error occurred while reading the workbook.",
      detail: err.message,
    };
  }
  return { headline: "Couldn't open this workbook", body: err.message };
}

function LeniencyChip({ report }: { report: LoadReport }): React.ReactElement | null {
  const [dismissed, setDismissed] = useState(false);
  const total = report.fixes.reduce((n, f) => n + f.occurrences, 0);
  if (dismissed || (total === 0 && report.warnings.length === 0)) return null;

  return (
    <div style={CHIP_OUTER_STYLE}>
      <div style={CHIP_STYLE}>
        <span style={CHIP_ICON_STYLE} aria-hidden="true">
          ⓘ
        </span>
        <span style={{ marginRight: 8 }}>
          {total > 0
            ? `Fixed ${total} invalid attribute${total === 1 ? "" : "s"}`
            : "Loaded with warnings"}
        </span>
        <FixSummary fixes={report.fixes} />
        <button
          type="button"
          style={CHIP_CLOSE_STYLE}
          onClick={() => setDismissed(true)}
          aria-label="Dismiss"
        >
          ×
        </button>
      </div>
    </div>
  );
}

function FixSummary({ fixes }: { fixes: readonly FixedAttribute[] }): React.ReactElement | null {
  const summary = formatFixesSummary(fixes);
  if (!summary) return null;
  return <span style={CHIP_MUTED_STYLE}>· {summary}</span>;
}

function formatFixesSummary(fixes: readonly FixedAttribute[]): string {
  const head = fixes.slice(0, 3).map((f) => {
    const attr = f.field ? `${f.field}=${JSON.stringify(f.value ?? "")}` : (f.value ?? "?");
    return f.occurrences > 1 ? `${attr} ×${f.occurrences}` : attr;
  });
  if (fixes.length > 3) head.push(`+${fixes.length - 3} more`);
  return head.join(", ");
}

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / 1024 / 1024).toFixed(1)} MB`;
}

const ERROR_CARD_OUTER_STYLE: CSSProperties = {
  position: "absolute",
  inset: 0,
  display: "flex",
  alignItems: "center",
  justifyContent: "center",
  padding: 24,
  background: "rgba(248, 250, 252, 0.92)",
  backdropFilter: "blur(2px)",
  zIndex: 10,
};

const ERROR_CARD_STYLE: CSSProperties = {
  maxWidth: 520,
  width: "100%",
  background: "#ffffff",
  border: "1px solid #e2e8f0",
  borderRadius: 12,
  boxShadow: "0 10px 30px rgba(15, 23, 42, 0.08)",
  padding: 20,
  fontFamily: "system-ui, -apple-system, Segoe UI, Roboto, sans-serif",
  color: "#0f172a",
};

const ERROR_HEADER_STYLE: CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: 10,
  marginBottom: 4,
};
const ERROR_ICON_STYLE: CSSProperties = { fontSize: 20, color: "#b45309" };
const ERROR_TITLE_STYLE: CSSProperties = {
  fontSize: 16,
  fontWeight: 600,
  margin: 0,
};
const ERROR_META_STYLE: CSSProperties = {
  fontSize: 12,
  color: "#64748b",
  marginBottom: 12,
};
const ERROR_BODY_STYLE: CSSProperties = {
  fontSize: 14,
  lineHeight: 1.5,
  margin: "0 0 12px 0",
  color: "#334155",
};
const ERROR_DETAIL_STYLE: CSSProperties = {
  fontFamily: "ui-monospace, SFMono-Regular, Menlo, monospace",
  fontSize: 12,
  background: "#f1f5f9",
  border: "1px solid #e2e8f0",
  borderRadius: 6,
  padding: "8px 10px",
  margin: "0 0 12px 0",
  color: "#0f172a",
  whiteSpace: "pre-wrap",
  wordBreak: "break-word",
};
const ERROR_ACTIONS_STYLE: CSSProperties = {
  display: "flex",
  gap: 8,
  justifyContent: "flex-end",
};
const BUTTON_STYLE: CSSProperties = {
  appearance: "none",
  font: "inherit",
  fontSize: 13,
  padding: "6px 12px",
  border: "1px solid #cbd5e1",
  borderRadius: 6,
  background: "#ffffff",
  color: "#0f172a",
  cursor: "pointer",
};

const CHIP_OUTER_STYLE: CSSProperties = {
  position: "absolute",
  bottom: 12,
  right: 12,
  maxWidth: 360,
  zIndex: 9,
  pointerEvents: "none",
};
const CHIP_STYLE: CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: 4,
  background: "rgba(15, 23, 42, 0.85)",
  color: "#f8fafc",
  fontFamily: "system-ui, -apple-system, Segoe UI, Roboto, sans-serif",
  fontSize: 12,
  padding: "6px 8px 6px 10px",
  borderRadius: 999,
  boxShadow: "0 4px 12px rgba(15, 23, 42, 0.18)",
  pointerEvents: "auto",
};
const CHIP_ICON_STYLE: CSSProperties = {
  fontSize: 13,
  marginRight: 4,
  opacity: 0.85,
};
const CHIP_MUTED_STYLE: CSSProperties = {
  opacity: 0.7,
  marginRight: 8,
  fontFamily: "ui-monospace, SFMono-Regular, Menlo, monospace",
  fontSize: 11,
};
const CHIP_CLOSE_STYLE: CSSProperties = {
  appearance: "none",
  background: "transparent",
  color: "#cbd5e1",
  border: "none",
  padding: "0 4px",
  font: "inherit",
  fontSize: 16,
  lineHeight: 1,
  cursor: "pointer",
};
