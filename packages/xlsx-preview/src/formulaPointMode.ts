export interface RefSpan {
  start: number;
  end: number;
}

export interface ApplyRefResult {
  text: string;
  caret: number;
  span: RefSpan;
}

const ACCEPT_BEFORE = new Set([
  "=",
  "(",
  ",",
  "+",
  "-",
  "*",
  "/",
  "^",
  "&",
  "<",
  ">",
  "%",
  " ",
]);

function insideStringLiteral(text: string, caretIndex: number): boolean {
  let quotes = 0;
  for (let i = 0; i < caretIndex; i++) {
    if (text[i] === '"') quotes++;
  }
  return quotes % 2 === 1;
}

export function caretAcceptsReference(text: string, caretIndex: number): boolean {
  if (!text.startsWith("=")) return false;
  if (caretIndex <= 0 || caretIndex > text.length) return false;
  if (insideStringLiteral(text, caretIndex)) return false;
  const before = text[caretIndex - 1]!;
  return ACCEPT_BEFORE.has(before);
}

export function applyReferenceAtCaret(
  text: string,
  caretIndex: number,
  ref: string,
  activeSpan: RefSpan | null,
): ApplyRefResult {
  const replace =
    activeSpan !== null && activeSpan.end === caretIndex && activeSpan.start <= activeSpan.end;
  const start = replace ? activeSpan!.start : caretIndex;
  const end = replace ? activeSpan!.end : caretIndex;
  const next = text.slice(0, start) + ref + text.slice(end);
  const span = { start, end: start + ref.length };
  return { text: next, caret: span.end, span };
}
