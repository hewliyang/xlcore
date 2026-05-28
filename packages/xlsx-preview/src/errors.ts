export type XlsxLoadErrorCode = "Zip" | "Schema" | "MissingPart" | "Io" | "Other";

export type XlsxSchemaErrorKind =
  | "InvalidFieldValue"
  | "InvalidEnumValue"
  | "Validation"
  | "UnexpectedTag"
  | "MissingField"
  | "UnexpectedEof"
  | "Other";

export interface XlsxLoadErrorPayload {
  code: XlsxLoadErrorCode;
  message: string;
  part?: string;
  schemaKind?: XlsxSchemaErrorKind;
  ty?: string;
  field?: string;
  value?: string;
}

const ERROR_CODES = new Set<XlsxLoadErrorCode>(["Zip", "Schema", "MissingPart", "Io", "Other"]);
const SCHEMA_KINDS = new Set<XlsxSchemaErrorKind>([
  "InvalidFieldValue",
  "InvalidEnumValue",
  "Validation",
  "UnexpectedTag",
  "MissingField",
  "UnexpectedEof",
  "Other",
]);

export function xlsxLoadErrorPayloadFromUnknown(error: unknown): XlsxLoadErrorPayload {
  if (!isRecord(error)) return { code: "Other", message: String(error) };

  const code = stringInSet(error.code, ERROR_CODES) ?? "Other";
  const message = typeof error.message === "string" ? error.message : String(error);
  const schemaKind = stringInSet(error.schemaKind, SCHEMA_KINDS);

  return {
    code,
    message,
    part: stringOrUndefined(error.part),
    schemaKind,
    ty: stringOrUndefined(error.ty),
    field: stringOrUndefined(error.field),
    value: stringOrUndefined(error.value),
  };
}

export class XlsxLoadError extends Error {
  readonly code: XlsxLoadErrorCode;
  readonly part?: string;
  readonly schemaKind?: XlsxSchemaErrorKind;
  readonly ty?: string;
  readonly field?: string;
  readonly value?: string;

  constructor(payload: XlsxLoadErrorPayload) {
    super(payload.message);
    this.name = "XlsxLoadError";
    this.code = payload.code;
    this.part = payload.part;
    this.schemaKind = payload.schemaKind;
    this.ty = payload.ty;
    this.field = payload.field;
    this.value = payload.value;
  }

  static isXlsxLoadError(error: unknown): error is XlsxLoadError {
    // Duck-type on `name` + `code` so detection survives worker / dual-ESM
    // boundaries where `instanceof` against a different realm fails.
    if (error instanceof XlsxLoadError) return true;
    if (!isRecord(error)) return false;
    return error.name === "XlsxLoadError" && stringInSet(error.code, ERROR_CODES) !== undefined;
  }

  static fromUnknown(error: unknown): XlsxLoadError {
    return error instanceof XlsxLoadError
      ? error
      : new XlsxLoadError(xlsxLoadErrorPayloadFromUnknown(error));
  }

  diagnosticsText(): string {
    const lines = [`XlsxLoadError [${this.code}]`, this.message];
    if (this.part) lines.push(`  part: ${this.part}`);
    if (this.schemaKind) lines.push(`  schemaKind: ${this.schemaKind}`);
    if (this.ty) lines.push(`  ty: ${this.ty}`);
    if (this.field !== undefined) lines.push(`  field: ${this.field}`);
    if (this.value !== undefined) lines.push(`  value: ${JSON.stringify(this.value)}`);
    return lines.join("\n");
  }
}

export interface FixedAttribute {
  part: string;
  ty?: string;
  field?: string;
  value?: string;
  occurrences: number;
  kind: XlsxSchemaErrorKind;
}

export interface LoadReport {
  fixes: FixedAttribute[];
  warnings: string[];
}

export const EMPTY_LOAD_REPORT: LoadReport = Object.freeze({
  fixes: Object.freeze([]) as readonly FixedAttribute[],
  warnings: Object.freeze([]) as readonly string[],
}) as unknown as LoadReport;

export function reportIsClean(report: LoadReport | null | undefined): boolean {
  if (!report) return true;
  return report.fixes.length === 0 && report.warnings.length === 0;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === "object";
}

function stringOrUndefined(value: unknown): string | undefined {
  return typeof value === "string" ? value : undefined;
}

function stringInSet<T extends string>(value: unknown, set: ReadonlySet<T>): T | undefined {
  return typeof value === "string" && set.has(value as T) ? (value as T) : undefined;
}
