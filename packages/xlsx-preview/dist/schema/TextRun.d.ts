import type { Color } from "./Color.js";
/**
 * One styled span inside a rich-text cell. Mirrors `<r><rPr/><t/></r>` in
 * OOXML. Properties left as `None`/`false` mean "inherit from the cell's
 * own font".
 */
export type TextRun = {
    text: string;
    bold: boolean;
    italic: boolean;
    underline: boolean;
    /**
     * OOXML `<u val="..."/>` variant when not the default `single`.
     * One of `"single"` / `"double"` / `"singleAccounting"` /
     * `"doubleAccounting"`. Absent = `single` (matches the OOXML default).
     * Renderer paints `double*` as two parallel strokes; the
     * accounting variants currently fall through to single/double
     * (the "line extends across the full cell width" semantics are
     * not honored yet — tracked in PARITY.md).
     */
    underlineStyle?: string;
    strike: boolean;
    /**
     * Font size in points (matches `Font.size`).
     */
    size?: number;
    fontName?: string;
    color?: Color;
    /**
     * OOXML `<vertAlign val="..."/>` — `"superscript"` or
     * `"subscript"`. `"baseline"` (the default) is omitted. Renderer
     * draws sup/sub at ~58% of the run's font size, shifted ±33%/+14% of
     * the base font's em above/below the baseline.
     */
    vertAlign?: string;
    /**
     * OOXML `<family val="N"/>` — numeric font-family hint (0..5):
     * 0=N/A, 1=Roman (serif), 2=Swiss (sans-serif), 3=Modern (monospace),
     * 4=Script (cursive), 5=Decorative (fantasy). Renderer uses this to
     * pick a richer CSS fallback so a workbook authored in a serif
     * typeface that's not installed locally still falls back to a serif
     * (not the generic sans-serif default).
     */
    family?: number;
    /**
     * OOXML `<scheme val="major|minor"/>` — theme-font reference. When
     * present, the run logically references the workbook's theme major /
     * minor font; the `<rFont>` cache may be stale if a different theme
     * document has been swapped in. Renderer prefers the resolved theme
     * font over `font_name` when this is set. `"none"` is omitted.
     */
    scheme?: string;
};
