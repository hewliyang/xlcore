export interface AutocompleteState {
  token: string;
  start: number;
  end: number;
  matches: string[];
}

const MAX_MATCHES = 12;
const NAME_START_BEFORE = new Set([
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

function isNameStartChar(ch: string): boolean {
  return /[A-Za-z]/.test(ch);
}

function isNameChar(ch: string): boolean {
  return /[A-Za-z0-9.]/.test(ch);
}

function insideStringLiteral(text: string, caretIndex: number): boolean {
  let quotes = 0;
  for (let i = 0; i < caretIndex; i++) {
    if (text[i] === '"') quotes++;
  }
  return quotes % 2 === 1;
}

export function autocompleteState(
  text: string,
  caretIndex: number,
  names: string[],
): AutocompleteState | null {
  if (caretIndex < 0 || caretIndex > text.length) return null;
  if (!text.startsWith("=")) return null;
  if (insideStringLiteral(text, caretIndex)) return null;

  let start = caretIndex;
  while (start > 0 && isNameChar(text[start - 1]!)) start--;
  const token = text.slice(start, caretIndex);
  if (token.length === 0) return null;
  if (!isNameStartChar(token[0]!)) return null;

  const before = start === 0 ? "" : text[start - 1]!;
  if (start !== 0 && !NAME_START_BEFORE.has(before)) return null;

  const upper = token.toUpperCase();
  const matches: string[] = [];
  for (const name of names) {
    if (name.toUpperCase().startsWith(upper)) {
      matches.push(name);
      if (matches.length >= MAX_MATCHES) break;
    }
  }
  if (matches.length === 0) return null;

  return { token, start, end: caretIndex, matches };
}
