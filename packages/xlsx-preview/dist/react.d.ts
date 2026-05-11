import type * as React from "react";
import { type CreateWorkbookPreviewerFromFileOptions, type WorkbookLoadProgress } from "./browserLoader.js";
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
export interface ExcelPreviewerProps extends Omit<React.HTMLAttributes<HTMLDivElement>, "children" | "onError" | "onProgress">, UseWorkbookPreviewerOptions {
    file: Blob | null | undefined;
    previewerRef?: React.Ref<WorkbookPreviewer | null>;
}
export declare function useWorkbookPreviewer(file: Blob | null | undefined, options?: UseWorkbookPreviewerOptions): UseWorkbookPreviewerResult;
export declare function ExcelPreviewer(props: ExcelPreviewerProps): React.ReactElement;
