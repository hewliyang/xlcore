/* tslint:disable */
/* eslint-disable */

export class WorkbookHandle {
    free(): void;
    [Symbol.dispose](): void;
    addMerge(reference: string): any;
    addThreadedNote(reference: string, patch: any): any;
    autoFilter(sheet: string): any;
    calcProperties(): any;
    clear(reference: string): any;
    clearConditionalFormats(reference: string): any;
    clearRange(reference: string): any;
    clearRangeWith(reference: string, mode: any): any;
    clearWith(reference: string, mode: any): any;
    comments(sheet: string): any;
    conditionalFormats(sheet: string): any;
    copyRange(src_reference: string, dst_reference: string): any;
    createSheet(name: string): any;
    dataValidations(sheet: string): any;
    definedNames(): any;
    deleteColumns(sheet: string, start: number, count: number): void;
    deleteRows(sheet: string, start: number, count: number): void;
    deleteSheet(name: string): void;
    dependencies(reference: string): any;
    dependents(reference: string): any;
    dispose(): void;
    fillRange(src_reference: string, dst_reference: string): any;
    getCell(reference: string): any;
    getFreeze(sheet: string): any;
    getRange(reference: string): any;
    hyperlinks(sheet: string): any;
    insertColumns(sheet: string, before: number, count: number): void;
    insertRows(sheet: string, before: number, count: number): void;
    layout(options: any): any;
    merges(sheet: string): any;
    moveSheet(name: string, to_index: number): any;
    constructor();
    static open(bytes: Uint8Array): WorkbookHandle;
    pageSetup(sheet: string): any;
    precedents(reference: string): any;
    properties(): any;
    recalculate(): any;
    removeAutoFilter(sheet: string): any;
    removeAutoFilterColumn(sheet: string, column_offset: number): any;
    removeComment(reference: string): any;
    removeDataValidation(reference: string): any;
    removeDefinedName(name: string, scope?: string | null): any;
    removeHyperlink(reference: string): any;
    removeMerge(reference: string): any;
    removePageSetup(sheet: string): any;
    removeSheetProtection(sheet: string): any;
    removeTable(name: string): any;
    removeThreadedThread(reference: string): any;
    removeWorkbookProtection(): any;
    renameSheet(old_name: string, new_name: string): void;
    replyThreadedNote(parent_id: string, patch: any): any;
    save(): Uint8Array;
    search(query: string, options: any): any;
    setActiveSheet(name: string): any;
    setAutoFilter(reference: string): any;
    setAutoFilterColumn(sheet: string, patch: any): any;
    setCalcProperties(patch: any): any;
    setColumnVisible(sheet: string, column: number, visible: boolean): void;
    setColumnWidth(sheet: string, column: number, width: number): void;
    setComment(reference: string, patch: any): any;
    setConditionalFormat(reference: string, patch: any): any;
    setDataValidation(reference: string, patch: any): any;
    setDefinedName(patch: any): any;
    setFormula(reference: string, formula: string): any;
    setFreeze(sheet: string, frozen_rows: number, frozen_columns: number): any;
    setHyperlink(reference: string, patch: any): any;
    setPageSetup(sheet: string, patch: any): any;
    setProperties(patch: any): any;
    setRangeFormulas(reference: string, formulas: any): any;
    setRangeValues(reference: string, values: any): any;
    setRowHeight(sheet: string, row: number, height: number): void;
    setRowVisible(sheet: string, row: number, visible: boolean): void;
    setSheetProtection(sheet: string, patch: any): any;
    setSheetVisibility(name: string, visibility: string): any;
    setStyle(reference: string, patch: any): any;
    setTable(patch: any): any;
    setValue(reference: string, value: any): any;
    setWorkbookProtection(patch: any): any;
    sheetProtection(sheet: string): any;
    sheets(): any;
    tables(sheet?: string | null): any;
    threadedNotes(sheet: string): any;
    workbookProtection(): any;
}

export function extractXlsxJson(bytes: Uint8Array, options: any): string;

export function extract_csv(bytes: Uint8Array, options: any): any;

export function extract_parquet(bytes: Uint8Array, options: any): any;

export function extract_xlsx(bytes: Uint8Array, options: any): any;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_workbookhandle_free: (a: number, b: number) => void;
    readonly extractXlsxJson: (a: number, b: number, c: number, d: number) => void;
    readonly extract_csv: (a: number, b: number, c: number, d: number) => void;
    readonly extract_parquet: (a: number, b: number, c: number, d: number) => void;
    readonly extract_xlsx: (a: number, b: number, c: number, d: number) => void;
    readonly workbookhandle_addMerge: (a: number, b: number, c: number, d: number) => void;
    readonly workbookhandle_addThreadedNote: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly workbookhandle_autoFilter: (a: number, b: number, c: number, d: number) => void;
    readonly workbookhandle_calcProperties: (a: number, b: number) => void;
    readonly workbookhandle_clear: (a: number, b: number, c: number, d: number) => void;
    readonly workbookhandle_clearConditionalFormats: (a: number, b: number, c: number, d: number) => void;
    readonly workbookhandle_clearRange: (a: number, b: number, c: number, d: number) => void;
    readonly workbookhandle_clearRangeWith: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly workbookhandle_clearWith: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly workbookhandle_comments: (a: number, b: number, c: number, d: number) => void;
    readonly workbookhandle_conditionalFormats: (a: number, b: number, c: number, d: number) => void;
    readonly workbookhandle_copyRange: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
    readonly workbookhandle_createSheet: (a: number, b: number, c: number, d: number) => void;
    readonly workbookhandle_dataValidations: (a: number, b: number, c: number, d: number) => void;
    readonly workbookhandle_definedNames: (a: number, b: number) => void;
    readonly workbookhandle_deleteColumns: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
    readonly workbookhandle_deleteRows: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
    readonly workbookhandle_deleteSheet: (a: number, b: number, c: number, d: number) => void;
    readonly workbookhandle_dependencies: (a: number, b: number, c: number, d: number) => void;
    readonly workbookhandle_dependents: (a: number, b: number, c: number, d: number) => void;
    readonly workbookhandle_dispose: (a: number) => void;
    readonly workbookhandle_fillRange: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
    readonly workbookhandle_getCell: (a: number, b: number, c: number, d: number) => void;
    readonly workbookhandle_getFreeze: (a: number, b: number, c: number, d: number) => void;
    readonly workbookhandle_getRange: (a: number, b: number, c: number, d: number) => void;
    readonly workbookhandle_hyperlinks: (a: number, b: number, c: number, d: number) => void;
    readonly workbookhandle_insertColumns: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
    readonly workbookhandle_insertRows: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
    readonly workbookhandle_layout: (a: number, b: number, c: number) => void;
    readonly workbookhandle_merges: (a: number, b: number, c: number, d: number) => void;
    readonly workbookhandle_moveSheet: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly workbookhandle_new: (a: number) => void;
    readonly workbookhandle_open: (a: number, b: number, c: number) => void;
    readonly workbookhandle_pageSetup: (a: number, b: number, c: number, d: number) => void;
    readonly workbookhandle_precedents: (a: number, b: number, c: number, d: number) => void;
    readonly workbookhandle_properties: (a: number, b: number) => void;
    readonly workbookhandle_recalculate: (a: number, b: number) => void;
    readonly workbookhandle_removeAutoFilter: (a: number, b: number, c: number, d: number) => void;
    readonly workbookhandle_removeAutoFilterColumn: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly workbookhandle_removeComment: (a: number, b: number, c: number, d: number) => void;
    readonly workbookhandle_removeDataValidation: (a: number, b: number, c: number, d: number) => void;
    readonly workbookhandle_removeDefinedName: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
    readonly workbookhandle_removeHyperlink: (a: number, b: number, c: number, d: number) => void;
    readonly workbookhandle_removeMerge: (a: number, b: number, c: number, d: number) => void;
    readonly workbookhandle_removePageSetup: (a: number, b: number, c: number, d: number) => void;
    readonly workbookhandle_removeSheetProtection: (a: number, b: number, c: number, d: number) => void;
    readonly workbookhandle_removeTable: (a: number, b: number, c: number, d: number) => void;
    readonly workbookhandle_removeThreadedThread: (a: number, b: number, c: number, d: number) => void;
    readonly workbookhandle_removeWorkbookProtection: (a: number, b: number) => void;
    readonly workbookhandle_renameSheet: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
    readonly workbookhandle_replyThreadedNote: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly workbookhandle_save: (a: number, b: number) => void;
    readonly workbookhandle_search: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly workbookhandle_setActiveSheet: (a: number, b: number, c: number, d: number) => void;
    readonly workbookhandle_setAutoFilter: (a: number, b: number, c: number, d: number) => void;
    readonly workbookhandle_setAutoFilterColumn: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly workbookhandle_setCalcProperties: (a: number, b: number, c: number) => void;
    readonly workbookhandle_setColumnVisible: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
    readonly workbookhandle_setColumnWidth: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
    readonly workbookhandle_setComment: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly workbookhandle_setConditionalFormat: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly workbookhandle_setDataValidation: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly workbookhandle_setDefinedName: (a: number, b: number, c: number) => void;
    readonly workbookhandle_setFormula: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
    readonly workbookhandle_setFreeze: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
    readonly workbookhandle_setHyperlink: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly workbookhandle_setPageSetup: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly workbookhandle_setProperties: (a: number, b: number, c: number) => void;
    readonly workbookhandle_setRangeFormulas: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly workbookhandle_setRangeValues: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly workbookhandle_setRowHeight: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
    readonly workbookhandle_setRowVisible: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
    readonly workbookhandle_setSheetProtection: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly workbookhandle_setSheetVisibility: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
    readonly workbookhandle_setStyle: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly workbookhandle_setTable: (a: number, b: number, c: number) => void;
    readonly workbookhandle_setValue: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly workbookhandle_setWorkbookProtection: (a: number, b: number, c: number) => void;
    readonly workbookhandle_sheetProtection: (a: number, b: number, c: number, d: number) => void;
    readonly workbookhandle_sheets: (a: number, b: number) => void;
    readonly workbookhandle_tables: (a: number, b: number, c: number, d: number) => void;
    readonly workbookhandle_threadedNotes: (a: number, b: number, c: number, d: number) => void;
    readonly workbookhandle_workbookProtection: (a: number, b: number) => void;
    readonly __wbindgen_export: (a: number, b: number) => number;
    readonly __wbindgen_export2: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_export3: (a: number) => void;
    readonly __wbindgen_add_to_stack_pointer: (a: number) => number;
    readonly __wbindgen_export4: (a: number, b: number, c: number) => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
