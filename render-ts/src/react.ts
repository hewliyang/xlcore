import { createElement, useEffect, useImperativeHandle, useRef, useState } from "react";
import type * as React from "react";
import {
  createWorkbookPreviewerFromFile,
  type CreateWorkbookPreviewerFromFileOptions,
  type WorkbookLoadProgress,
} from "./browserLoader.js";
import type { PreviewerState, WorkbookPreviewer } from "./previewer.js";

export interface UseWorkbookPreviewerOptions extends CreateWorkbookPreviewerFromFileOptions {
  onReady?: (previewer: WorkbookPreviewer) => void;
  onError?: (error: Error) => void;
  onSelectionChange?: (state: PreviewerState) => void;
  onSheetChange?: (state: PreviewerState) => void;
  onZoomChange?: (state: PreviewerState) => void;
}

export interface UseWorkbookPreviewerResult {
  containerRef: React.RefObject<HTMLDivElement | null>;
  previewer: WorkbookPreviewer | null;
  progress: WorkbookLoadProgress | null;
  error: Error | null;
  loading: boolean;
}

export interface ExcelPreviewerProps
  extends Omit<React.HTMLAttributes<HTMLDivElement>, "children" | "onError" | "onProgress">,
    UseWorkbookPreviewerOptions {
  file: Blob | null | undefined;
  previewerRef?: React.Ref<WorkbookPreviewer | null>;
}

export function useWorkbookPreviewer(
  file: Blob | null | undefined,
  options: UseWorkbookPreviewerOptions = {},
): UseWorkbookPreviewerResult {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const [previewer, setPreviewer] = useState<WorkbookPreviewer | null>(null);
  const [progress, setProgress] = useState<WorkbookLoadProgress | null>(null);
  const [error, setError] = useState<Error | null>(null);
  const [loading, setLoading] = useState(false);
  const optionsRef = useRef(options);
  optionsRef.current = options;

  useEffect(() => {
    const container = containerRef.current;
    if (!container || !file) {
      setPreviewer(null);
      setProgress(null);
      setError(null);
      setLoading(false);
      return;
    }

    let cancelled = false;
    let activePreviewer: WorkbookPreviewer | null = null;
    setPreviewer(null);
    setProgress(null);
    setError(null);
    setLoading(true);

    const progressHandler = (next: WorkbookLoadProgress) => {
      if (cancelled) return;
      setProgress(next);
      optionsRef.current.onProgress?.(next);
    };

    createWorkbookPreviewerFromFile(container, file, {
      ...optionsRef.current,
      onProgress: progressHandler,
    })
      .then((next) => {
        if (cancelled) {
          next.destroy();
          return;
        }
        activePreviewer = next;
        attachPreviewerEvents(next, optionsRef);
        setPreviewer(next);
        setLoading(false);
        optionsRef.current.onReady?.(next);
      })
      .catch((reason: unknown) => {
        const nextError = reason instanceof Error ? reason : new Error(String(reason));
        if (cancelled) return;
        setError(nextError);
        setLoading(false);
        optionsRef.current.onError?.(nextError);
      });

    return () => {
      cancelled = true;
      activePreviewer?.destroy();
    };
  }, [file]);

  return { containerRef, previewer, progress, error, loading };
}

export function ExcelPreviewer(props: ExcelPreviewerProps): React.ReactElement {
  const {
    file,
    previewerRef,
    className,
    style,
    wasmUrl,
    workerUrl,
    initialSheet,
    initialZoom,
    onProgress,
    onReady,
    onError,
    onSelectionChange,
    onSheetChange,
    onZoomChange,
    ...divProps
  } = props;
  const result = useWorkbookPreviewer(file, {
    wasmUrl,
    workerUrl,
    initialSheet,
    initialZoom,
    className,
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

  return createElement("div", {
    ...divProps,
    ref: result.containerRef,
    className,
    style: { width: "100%", height: "100%", minHeight: 0, ...style },
  });
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
