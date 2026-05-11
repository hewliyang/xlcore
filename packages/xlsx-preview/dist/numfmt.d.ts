export interface FormatResult {
    text: string;
    /** CSS color from `[Red]` / `[Color12]` etc., if the matched section
     *  carried one. The renderer uses it as a font-color override. */
    color?: string;
    /** Fill chars (one per `*x` token), in left-to-right order in the
     *  rendered text. Each occurrence is marked in `text` with the
     *  sentinel `\u0001` (`FILL_SENTINEL`). The renderer expands each
     *  sentinel to N copies of the matching fill char where N is sized
     *  to make the whole string fill the cell's inner width — this needs
     *  font metrics + a cell rect, which numfmt doesn't have. */
    fills?: string[];
}
/** Placeholder char emitted at each `*x` fill point in `FormatResult.text`.
 *  Renderer measures the rest of the text and expands the sentinel. */
export declare const FILL_SENTINEL = "\u0001";
/** Format a numeric value through an OOXML format code.
 *
 *  `fmt` is the raw format code as it appears in `<numFmt formatCode="…"/>`
 *  or one of the built-in IDs (resolved by the caller before this hits).
 *
 *  Returns `{ text }` on the happy path; falls back to `formatGeneral` if
 *  the format is missing, "General", or the parser bails. Never throws. */
export declare function formatValue(value: number, fmt: string | undefined): FormatResult;
/** "General" rendering — used when no format is set or as a fallback. */
export declare function formatGeneral(v: number): string;
export type Tok = {
    kind: "lit";
    s: string;
} | {
    kind: "digit";
    ch: "0" | "#" | "?";
} | {
    kind: "dot";
} | {
    kind: "percent";
} | {
    kind: "exp";
    sign: "+" | "-" | "";
    upper: boolean;
} | {
    kind: "date";
    field: string;
} | {
    kind: "elapsed";
    field: "h" | "m" | "s";
    width: number;
} | {
    kind: "ampm";
    upper: boolean;
    abbreviated: boolean;
} | {
    kind: "fill";
    ch: string;
} | {
    kind: "text";
};
export interface Section {
    tokens: Tok[];
    color?: string;
    condition?: {
        op: ">" | "<" | ">=" | "<=" | "=" | "<>";
        value: number;
    };
    /** kind of section, picked from the token mix */
    flavor: "number" | "date" | "fraction" | "scientific" | "text" | "literal";
    intPlaces: number;
    fracPlaces: number;
    hasGrouping: boolean;
    scale: number;
    fractionDenom: number;
    fractionDenomQs: number;
    fractionIntPlaces: number;
    fractionHideZeroInt: boolean;
    expSign: "+" | "-" | "";
    expDigits: number;
    expUpper: boolean;
}
