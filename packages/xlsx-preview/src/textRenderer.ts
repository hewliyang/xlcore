import type { Cell, Dxf, Font, Sheet, TextRun, WorkbookLayout } from "./types.js";
import { resolveCellText, resolveCellXf } from "./cellText.js";
import { colorToCss } from "./color.js";
import { iterCellsInRange } from "./columnar.js";

import type { Grid } from "./grid.js";
import { buildMergeMaps, rectFor } from "./geometry.js";
import type { Visible } from "./renderTypes.js";

// ---------- rich-text run resolution ----------

/// One shaped span: a contiguous slice of text with a single resolved
/// font/style. Multiple `Span`s on the same line render side-by-side via
/// successive `fillText` calls.
interface Span {
  text: string;
  font: string; // canvas `ctx.font` shorthand (already shrunk for sup/sub)
  fontSizePx: number; // base-font size: drives line-height + decoration math
  color: string;
  bold: boolean; // kept for re-`measureText` shortcuts
  underline: boolean;
  /// OOXML `<u val="..."/>` variant when not the default `single`.
  /// One of `"double"` / `"singleAccounting"` / `"doubleAccounting"`.
  /// Painted by `paintTextDecorations`.
  underlineStyle?: string;
  strike: boolean;
  /// Per-piece baseline shift in px (sup = negative → raise; sub =
  /// positive → drop). Underline / strike inherit this shift because
  /// they're computed relative to the baseline argument passed in.
  baselineShiftPx?: number;
}

/// Build the flat list of `Span`s for a cell, honoring rich-text runs from
/// inline strings, shared-string runs, or falling back to a single span
/// styled by the cell's own font.
function resolveCellSpans(
  cell: Cell,
  text: string,
  layout: WorkbookLayout,
  baseFont: Font | undefined,
  baseColor: string,
  defaultFontFamily: string,
  defaultFontSizePt: number,
): Span[] {
  const baseSizePt = baseFont?.size ?? defaultFontSizePt;
  // OOXML `<scheme val="major|minor"/>` on the cell font resolves against
  // the workbook theme's `major_font` / `minor_font`. The `<name>` cache
  // can be stale when a theme document swaps in different typefaces, so
  // when scheme is present we trust the theme over the cache. Falls back
  // to the cached name if the theme is missing the relevant slot.
  const baseName =
    resolveSchemeName(baseFont?.scheme, layout) ?? baseFont?.name ?? defaultFontFamily;
  const baseFamily = baseFont?.family;
  const baseBold = baseFont?.bold ?? false;
  const baseItalic = baseFont?.italic ?? false;
  const baseUnderline = baseFont?.underline ?? false;
  const baseUnderlineStyle = baseFont?.underlineStyle;
  const baseStrike = baseFont?.strike ?? false;

  // Pull the run list. Inline cells carry it directly; shared-string cells
  // index into the workbook-level `sharedStringRuns` table.
  let runs: TextRun[] | undefined;
  if (cell.runs && cell.runs.length > 0) {
    runs = cell.runs;
  } else if (cell.type === "s" && layout.sharedStringRuns && cell.value !== undefined) {
    const idx = parseInt(cell.value, 10);
    const sr = layout.sharedStringRuns[idx];
    if (sr && sr.length > 0) runs = sr;
  }

  const baseVertAlign = baseFont?.vertAlign;

  if (!runs) {
    return [
      buildSpan(
        text,
        baseSizePt,
        baseName,
        baseFamily,
        baseBold,
        baseItalic,
        baseColor,
        baseUnderline,
        baseUnderlineStyle,
        baseStrike,
        baseVertAlign,
      ),
    ];
  }

  return runs.map((r) => {
    const sizePt = r.size ?? baseSizePt;
    // Same scheme → theme-font resolution path as the cell base; a run
    // with `<scheme val="major"/>` re-resolves to the theme's major font
    // even when `<rFont>` carries a stale cached name.
    const name = resolveSchemeName(r.scheme, layout) ?? r.fontName ?? baseName;
    const family = r.family ?? baseFamily;
    const bold = r.bold ?? baseBold;
    const italic = r.italic ?? baseItalic;
    const color = r.color ? colorToCss(r.color, baseColor) : baseColor;
    // TextRun fields are non-optional booleans (`false` when absent); we
    // OR the cell's base flags so e.g. a hyperlink dxf underline still
    // wins on a run with no rPr underline of its own.
    const underline = r.underline || baseUnderline;
    // Per-run variant wins; falls through to the cell-base font's variant.
    const underlineStyle = r.underlineStyle ?? baseUnderlineStyle;
    const strike = r.strike || baseStrike;
    // vertAlign on a run wins over the cell's base font; absent on both
    // means baseline (no shift).
    const vertAlign = r.vertAlign ?? baseVertAlign;
    return buildSpan(
      r.text,
      sizePt,
      name,
      family,
      bold,
      italic,
      color,
      underline,
      underlineStyle,
      strike,
      vertAlign,
    );
  });
}

/// Resolve OOXML `<scheme val="major|minor"/>` against the workbook's
/// theme font scheme. Returns `undefined` when scheme is absent, set to
/// `"none"`, or the requested slot is missing on the theme — caller
/// falls back to the `<name>` cache.
function resolveSchemeName(scheme: string | undefined, layout: WorkbookLayout): string | undefined {
  if (!scheme || scheme === "none") return undefined;
  const t = layout.theme;
  if (!t) return undefined;
  if (scheme === "major") return t.majorFont || undefined;
  if (scheme === "minor") return t.minorFont || undefined;
  return undefined;
}

/// Construct a `Span`, lowering OOXML `vertAlign` to a shrunk font + a
/// baseline-shift offset. Excel sup/sub is ~58% of the base font size
/// (`Font` panel in the desktop client), raised by ~33% of the base em
/// for superscript or dropped ~14% for subscript.
function buildSpan(
  text: string,
  sizePt: number,
  name: string,
  family: number | undefined,
  bold: boolean,
  italic: boolean,
  color: string,
  underline: boolean,
  underlineStyle: string | undefined,
  strike: boolean,
  vertAlign: string | undefined,
): Span {
  const basePx = ptToPx(sizePt);
  let drawSizePt = sizePt;
  let baselineShiftPx = 0;
  if (vertAlign === "superscript") {
    drawSizePt = sizePt * 0.58;
    baselineShiftPx = -basePx * 0.33;
  } else if (vertAlign === "subscript") {
    drawSizePt = sizePt * 0.58;
    baselineShiftPx = basePx * 0.14;
  }
  return {
    text,
    font: cssFont(name, drawSizePt, bold, italic, family),
    fontSizePx: basePx,
    color,
    bold,
    underline,
    underlineStyle,
    strike,
    baselineShiftPx: baselineShiftPx || undefined,
  };
}

/// Paint underline / strike for a single text segment on the canvas at
/// (`x`, `baseline`). Underline sits ~2px below the baseline; strike
/// runs through the visual middle (~30% above the baseline). Width is
/// the segment's measured pixel width.
///
/// `accountingExtent` (optional): when the span uses an OOXML
/// `singleAccounting` / `doubleAccounting` underline variant, the
/// underline extends across the full cell width (Excel's accounting
/// convention) instead of just the text segment. Caller passes the
/// cell's inner-rect `{x, w}` here. Non-accounting underlines ignore
/// it. Strike is unaffected by this option.
function paintTextDecorations(
  ctx: CanvasRenderingContext2D,
  span: Span,
  x: number,
  baseline: number,
  width: number,
  accountingExtent?: { x: number; w: number },
): void {
  if (!span.underline && !span.strike) return;
  ctx.save();
  ctx.strokeStyle = span.color;
  // Line thickness scales with font size; floor to 1 so it stays crisp
  // at small zooms.
  ctx.lineWidth = Math.max(1, span.fontSizePx / 16);
  if (span.underline) {
    const y = baseline + Math.max(1, span.fontSizePx * 0.12);
    const v = span.underlineStyle;
    const isAccounting = v === "singleAccounting" || v === "doubleAccounting";
    // Accounting variants span the full cell width; otherwise the line
    // matches the text segment.
    const ux = isAccounting && accountingExtent ? accountingExtent.x : x;
    const uw = isAccounting && accountingExtent ? accountingExtent.w : width;
    ctx.beginPath();
    ctx.moveTo(ux, y);
    ctx.lineTo(ux + uw, y);
    ctx.stroke();
    // OOXML `<u val="double">` / `"doubleAccounting"`: paint a second
    // parallel stroke ~2px below the first.
    if (v === "double" || v === "doubleAccounting") {
      const gap = Math.max(2, span.fontSizePx * 0.1);
      const y2 = y + gap;
      ctx.beginPath();
      ctx.moveTo(ux, y2);
      ctx.lineTo(ux + uw, y2);
      ctx.stroke();
    }
  }
  if (span.strike) {
    const y = baseline - span.fontSizePx * 0.3;
    ctx.beginPath();
    ctx.moveTo(x, y);
    ctx.lineTo(x + width, y);
    ctx.stroke();
  }
  ctx.restore();
}

function ptToPx(pt: number): number {
  return (pt * 4) / 3;
}

/// Map an OOXML `<family val="N"/>` numeric family hint to a chain of
/// CSS fallback typefaces. When the workbook's named typeface isn't
/// installed locally, the browser walks this chain — so a serif workbook
/// stays in a serif, a monospace workbook stays in a monospace, etc.
///
/// OOXML ST_FontFamilyNum (ECMA-376 §18.18.30):
///   0 = Not applicable
///   1 = Roman      → serif
///   2 = Swiss      → sans-serif
///   3 = Modern     → monospace
///   4 = Script     → cursive
///   5 = Decorative → fantasy
function familyFallbackChain(family: number | undefined): string {
  switch (family) {
    case 1:
      return '"Cambria", "Times New Roman", Georgia, serif';
    case 3:
      return 'Consolas, "Courier New", monospace';
    case 4:
      return '"Brush Script MT", "Lucida Handwriting", cursive';
    case 5:
      return "Papyrus, Impact, fantasy";
    case 2:
    case 0:
    default:
      // Sans-serif covers most workbooks (Calibri / Aptos / Arial are
      // all family=2). Unknown / missing also falls here.
      return "Calibri, Aptos, Arial, sans-serif";
  }
}

function cssFont(
  name: string,
  sizePt: number,
  bold: boolean,
  italic: boolean,
  family?: number,
): string {
  const px = ptToPx(sizePt);
  return `${italic ? "italic " : ""}${bold ? "bold " : ""}${px}px "${name}", ${familyFallbackChain(family)}`;
}

/// One laid-out line: a list of (font-styled) span pieces in display order,
/// plus the line's pixel width and the tallest font on the line (drives the
/// per-line baseline advance).
interface LaidLine {
  pieces: { span: Span; text: string; width: number }[];
  width: number;
  height: number; // line height = 1.2 * tallest fontSizePx
  ascent: number; // baseline offset from the line's top
}

/// Lay spans out into wrapped/hard-broken lines that fit `maxWidth`.
/// Honors `\n` as hard breaks. When `wrap` is false, only `\n` produces
/// new lines (mirrors Excel: unwrapped cells still respect explicit
/// newlines but never auto-wrap).
function layoutSpans(
  ctx: CanvasRenderingContext2D,
  spans: Span[],
  maxWidth: number,
  wrap: boolean,
): LaidLine[] {
  const lines: LaidLine[] = [];
  let current: LaidLine = { pieces: [], width: 0, height: 0, ascent: 0 };

  const finishLine = () => {
    // Empty lines (back-to-back \n) still need a height. Use the height of
    // the trailing span if we have one, else fall back to a default.
    if (current.height === 0) {
      const fallback = spans[spans.length - 1]?.fontSizePx ?? 14;
      current.height = fallback * 1.2;
      current.ascent = fallback * 0.8;
    }
    lines.push(current);
    current = { pieces: [], width: 0, height: 0, ascent: 0 };
  };

  const pushPiece = (span: Span, text: string, width: number) => {
    current.pieces.push({ span, text, width });
    current.width += width;
    const lh = span.fontSizePx * 1.2;
    if (lh > current.height) current.height = lh;
    const asc = span.fontSizePx * 0.8;
    if (asc > current.ascent) current.ascent = asc;
  };

  for (const span of spans) {
    ctx.font = span.font;
    // Split into hard-break segments first; each `\n` ends the current line.
    const segs = span.text.split("\n");
    for (let si = 0; si < segs.length; si++) {
      const seg = segs[si]!;
      if (seg.length > 0) {
        if (!wrap) {
          pushPiece(span, seg, ctx.measureText(seg).width);
        } else {
          // Word-by-word wrap. Keep trailing whitespace on the *current* word
          // so we never strand a leading space at line-start. Tokenize on
          // word boundaries while preserving the separators.
          const tokens = seg.match(/\s+|\S+/g) ?? [];
          let buf = "";
          let bufW = 0;
          for (const tok of tokens) {
            const tokW = ctx.measureText(tok).width;
            // First word on a fresh line always fits, even if oversized:
            // Excel never breaks mid-word in v0 (we'll add it later if
            // needed). Otherwise, see if it overflows.
            if (
              current.width + bufW + tokW > maxWidth &&
              (current.pieces.length > 0 || buf.length > 0)
            ) {
              if (buf.length > 0) {
                pushPiece(span, buf, bufW);
                buf = "";
                bufW = 0;
              }
              finishLine();
              ctx.font = span.font; // ctx.font got reset across finishLine ops
              // Skip pure-whitespace token at the start of a wrapped line.
              if (/^\s+$/.test(tok)) continue;
            }
            buf += tok;
            bufW += tokW;
          }
          if (buf.length > 0) pushPiece(span, buf, bufW);
        }
      }
      if (si < segs.length - 1) finishLine();
    }
  }
  if (current.pieces.length > 0 || current.height > 0 || lines.length === 0) {
    finishLine();
  }
  return lines;
}

function occupiedCellsInRange(
  sheet: Sheet,
  layout: WorkbookLayout,
  firstRow: number,
  lastRow: number,
  firstCol: number,
  lastCol: number,
): Set<string> {
  const occupied = new Set<string>();
  iterCellsInRange(sheet, firstRow, lastRow, firstCol, lastCol, (cell) => {
    // A cell is "occupied" iff it has visible content. Empty styled cells
    // can still be overflowed into.
    if (hasContent(cell, sheet, layout)) occupied.add(`${cell.r}:${cell.c}`);
  });
  return occupied;
}

export function drawCellText(
  ctx: CanvasRenderingContext2D,
  sheet: Sheet,
  layout: WorkbookLayout,
  g: Grid,
  vis: Visible,
  cfDxfs: Map<string, Dxf>,
  cfTextSuppress: Set<string>,
  cfIconReserve: Map<string, number>,
): void {
  const { covered, topLeftOf } = buildMergeMaps(sheet);
  const styles = layout.styles;

  // Allow text to overflow horizontally into the visible column band; we
  // also pad by a few columns on the left so a long string anchored just
  // off-screen still bleeds in.
  const overflowFirstCol = Math.max(1, vis.firstCol - 8);
  const overflowLastCol = Math.min(g.maxCol, vis.lastCol + 8);
  // Build a fast "this position is occupied" lookup only for the rows and
  // columns this paint can actually consult. A whole-sheet occupancy map is
  // pathological for large flat data exports where first paint only needs a
  // small viewport.
  const occupied = occupiedCellsInRange(
    sheet,
    layout,
    vis.firstRow,
    vis.lastRow,
    overflowFirstCol,
    overflowLastCol,
  );
  for (const k of covered) occupied.add(k);

  iterCellsInRange(sheet, vis.firstRow, vis.lastRow, overflowFirstCol, overflowLastCol, (cell) => {
    const k = `${cell.r}:${cell.c}`;
    if (covered.has(k)) return;
    if (cfTextSuppress.has(k)) return;
    // Skip cells whose own column/row has been collapsed to zero size
    // (hidden columns, hidden rows, or columns inside a collapsed
    // outline group). Excel does not paint or overflow text from a
    // hidden cell into its visible neighbors. Merged cells use the
    // merge anchor's rect later, so let those through — `topLeftOf`
    // handles the size accounting for the whole range.
    if (!topLeftOf.has(k)) {
      const ownColW = g.colW[cell.c] ?? 0;
      const ownRowH = g.rowH[cell.r] ?? 0;
      if (ownColW <= 0 || ownRowH <= 0) return;
    }
    const xf = resolveCellXf(cell, sheet, layout);
    const resolved = resolveCellText(cell, layout, xf);
    let { text } = resolved;
    const { defaultAlign, formatColor, fills } = resolved;
    if (!text) return;

    const baseFontEntry = xf?.fontId !== undefined ? styles.fonts[xf.fontId] : undefined;
    // Apply CF dxf overrides (cellIs / expression). Bold/italic/underline/
    // strike/font-color from the dxf win over the base font; missing fields
    // inherit. Number-format override is handled in resolveCellText below
    // when present (re-derive `text` so percentages etc. work).
    const dxf = cfDxfs.get(k);
    let font: Font | undefined = baseFontEntry;
    if (dxf) {
      font = {
        ...(baseFontEntry ?? {}),
        bold: dxf.bold ?? baseFontEntry?.bold ?? false,
        italic: dxf.italic ?? baseFontEntry?.italic ?? false,
        underline: dxf.underline ?? baseFontEntry?.underline ?? false,
        underlineStyle: dxf.underlineStyle ?? baseFontEntry?.underlineStyle,
        strike: dxf.strike ?? baseFontEntry?.strike ?? false,
        color: dxf.fontColor ?? baseFontEntry?.color,
      };
    }
    const baseColor =
      (dxf?.fontColor ? colorToCss(dxf.fontColor, "#000000") : formatColor) ??
      colorToCss(font?.color, "#000000");
    const halign = xf?.horizontalAlignment ?? defaultAlign;
    const valign = xf?.verticalAlignment ?? "bottom";
    const wrap = xf?.wrapText ?? false;

    // Build run-styled spans (rich text or single span) before any wrap
    // computation, since per-run font sizes affect wrapping width.
    const spans = resolveCellSpans(
      cell,
      text,
      layout,
      font,
      baseColor,
      styles.defaultFont,
      styles.defaultFontSize,
    );

    const ownRect = rectFor(sheet, g, cell.r, cell.c, topLeftOf);
    const merge = topLeftOf.get(k);
    const isMerged = !!merge;
    const padX = 4;

    // Accounting `*x` fill expansion: numfmt left FILL_SENTINEL chars
    // in `text`; we measure the rest at the primary span's font and
    // pad each sentinel with N copies of its fill char so the whole
    // string fills the cell's inner width. Excel accounting renders
    // `_($* #,##0_)` as `$    80,539 ` — the `*` is what produces the
    // gap between the currency symbol and the right-justified number.
    // We force the text rectangle to span the full inner width (no
    // overflow / no extra alignment shift) because the format already
    // encodes the horizontal placement.
    if (fills && fills.length > 0 && text.includes("\u0001")) {
      const primary = spans[0]!;
      const prevFont = ctx.font;
      ctx.font = primary.font;
      // Measure the text with every sentinel stripped — that's the
      // fixed-width content. The remaining width goes to fill chars.
      const stripped = text.replace(/\u0001/g, "");
      const baseW = ctx.measureText(stripped).width;
      const innerW = Math.max(0, ownRect.w - padX * 2);
      let avail = innerW - baseW;
      const parts = text.split("\u0001");
      const fillCount = parts.length - 1;
      if (fillCount > 0) {
        let assembled = parts[0]!;
        for (let fi = 0; fi < fillCount; fi++) {
          const ch = fills[fi] ?? fills[fills.length - 1] ?? " ";
          const chW = Math.max(0.5, ctx.measureText(ch).width);
          // Spread the remaining slack evenly across the remaining
          // sentinels so multiple `*` fills (rare) share the gap.
          const slice = avail / (fillCount - fi);
          const n = Math.max(0, Math.floor(slice / chW));
          avail -= n * chW;
          assembled += ch.repeat(n) + parts[fi + 1]!;
        }
        text = assembled;
        // Numbers never carry rich-text runs — spans is always length 1
        // on this path. Rebuild the span so downstream layout / measure
        // walks see the padded string.
        if (spans.length === 1) spans[0] = { ...spans[0]!, text };
      }
      ctx.font = prevFont;
    }

    // Text rotation fast path. OOXML `textRotation`:
    //   0       horizontal (fall through to standard pipeline)
    //   1..90   N° counterclockwise
    //   91..180 (value-90)° clockwise
    //   255     stacked: chars upright, drawn vertically.
    // Single-line, no-wrap layout. We do NOT clip rotated runs to
    // `ownRect`: when a tall+narrow cell hosts a large rotated string,
    // the rotated bbox's perpendicular extent can exceed the column
    // width, and Excel/SpreadJS let it bleed past. Stacked (255) still
    // gets the cell-rect clip — its glyphs fit by construction.
    const textRot = xf?.textRotation ?? 0;
    if (textRot !== 0) {
      // Use the first span's font for rotated/stacked text (rich-text
      // rotation is rare and SpreadJS / Excel don't do anything fancy
      // with per-run sizing in rotated cells).
      const span = spans[0]!;
      ctx.save();
      if (textRot === 255) {
        ctx.beginPath();
        ctx.rect(ownRect.x, ownRect.y, ownRect.w, ownRect.h);
        ctx.clip();
      }
      ctx.font = span.font;
      ctx.fillStyle = span.color;
      ctx.textBaseline = "alphabetic";

      if (textRot === 255) {
        // Stacked: each character on its own line, no rotation. Anchor
        // horizontally centered by default (Excel's stacked text is
        // always center-horizontal regardless of halign, with a small
        // padding from cell edges).
        const lineH = span.fontSizePx * 1.05;
        const ascent = span.fontSizePx * 0.8;
        const cx = ownRect.x + ownRect.w / 2;
        const totalH = lineH * text.length;
        // Vertical block placement honors valign.
        let blockTop: number;
        switch (valign) {
          case "top":
            blockTop = ownRect.y + 2;
            break;
          case "center":
            blockTop = ownRect.y + (ownRect.h - totalH) / 2;
            break;
          default:
            blockTop = ownRect.y + ownRect.h - totalH - 2;
        }
        const prevAlign = ctx.textAlign;
        ctx.textAlign = "center";
        for (let i = 0; i < text.length; i++) {
          const ch = text[i]!;
          ctx.fillText(ch, cx, blockTop + i * lineH + ascent);
        }
        ctx.textAlign = prevAlign;
        ctx.restore();
        return;
      }

      // Rotated text. Convert OOXML angle to canvas rotation (positive =
      // CW in screen space because y grows downward).
      const angleRad =
        textRot <= 90
          ? (-textRot * Math.PI) / 180 // CCW (1..90)
          : ((textRot - 90) * Math.PI) / 180; // CW  (91..180)
      const tw = ctx.measureText(text).width;
      const ascent = span.fontSizePx * 0.8;
      const descent = span.fontSizePx * 0.2;
      const pad = 2;
      // Anchor convention matches Excel / SpreadJS:
      //   CCW (angleRad < 0): bottom-left of cell content; text reads
      //     up-right along the rotated baseline.
      //   CW  (angleRad > 0): top-left of cell content; text reads
      //     down-right along the rotated baseline.
      // Project the rotated text rectangle onto the cell's x-axis and
      // align by its bbox extremes (not the baseline). The four corners
      // of the unrotated rect at textBaseline=alphabetic are
      // (0,-ascent), (tw,-ascent), (tw,descent), (0,descent); canvas
      // rotation maps (px,py) -> (px*cos - py*sin, _) and only the
      // x-component matters here. Baseline-anchoring ignored the
      // ascender/descender asymmetry and visibly mis-centered large
      // 90° glyphs by ~(ascender-descender)/2.
      const cosA = Math.cos(angleRad);
      const sinA = Math.sin(angleRad);
      const cornerXs = [
        0 * cosA - -ascent * sinA,
        tw * cosA - -ascent * sinA,
        tw * cosA - descent * sinA,
        0 * cosA - descent * sinA,
      ];
      const rxMin = Math.min(...cornerXs);
      const rxMax = Math.max(...cornerXs);
      let anchorX: number;
      const anchorY =
        angleRad < 0
          ? ownRect.y + ownRect.h - pad // bottom for CCW
          : ownRect.y + pad + ascent; // top  for CW
      if (halign === "center") {
        anchorX = ownRect.x + ownRect.w / 2 - (rxMin + rxMax) / 2;
      } else if (halign === "right") {
        anchorX = ownRect.x + ownRect.w - pad - rxMax;
      } else {
        anchorX = ownRect.x + pad - rxMin;
      }
      ctx.translate(anchorX, anchorY);
      ctx.rotate(angleRad);
      ctx.fillText(text, 0, 0);
      paintTextDecorations(ctx, span, 0, 0, tw);
      ctx.restore();
      return;
    }
    // CF iconSet reserves the leftmost N pixels of the cell for the
    // glyph; text positioning shifts right by that amount.
    const iconReserve = cfIconReserve.get(k) ?? 0;
    const textOriginX = ownRect.x + iconReserve;
    // OOXML indent: each unit is "3 character widths" of the default
    // font. ~9px at 11pt Calibri matches Excel/SpreadJS closely.
    // Indent only applies on the alignment-anchor side (left for
    // left/general, right for right). Center is unaffected.
    const indentUnits = xf?.indent ?? 0;
    const indentPx = indentUnits > 0 ? indentUnits * Math.round(styles.defaultFontSize * 0.75) : 0;
    const effectiveAlign: "left" | "right" | "center" | undefined =
      halign === "center"
        ? "center"
        : halign === "right"
          ? "right"
          : halign === "left"
            ? "left"
            : defaultAlign === "right"
              ? "right"
              : "left";
    const indentLeft = effectiveAlign === "left" ? indentPx : 0;
    const indentRight = effectiveAlign === "right" ? indentPx : 0;

    // For unwrapped text we still get the classic "bleed into empty
    // neighbors" overflow; wrapped text stays inside its own (possibly
    // merged) rect. We need at least one rough width measurement to know
    // whether overflow is even called for, so use the first span's font.
    ctx.font = spans[0]?.font ?? `${(styles.defaultFontSize * 4) / 3}px sans-serif`;
    // Total width of the flat text -- for hard-break detection below we
    // also need the widest hard-line in case the cell has \n.
    const flatHasNewline = text.indexOf("\n") >= 0;
    // `clip` may grow into empty neighbor cells to emulate Excel overflow,
    // but horizontal alignment is still anchored to the source cell/merge.
    // Center/right text must not be re-centered/re-right-aligned inside the
    // enlarged overflow band, or long centered labels drift out of their
    // authored boxes.
    const alignRect = { ...ownRect };
    const clip = { ...ownRect };
    if (iconReserve > 0) {
      alignRect.x += iconReserve;
      alignRect.w -= iconReserve;
      clip.x += iconReserve;
      clip.w -= iconReserve;
    }
    if (!wrap) {
      // Measure max line width using each span's own font so mixed-size
      // rich text picks the right overflow target width.
      let maxLineW = 0;
      let curW = 0;
      for (const s of spans) {
        ctx.font = s.font;
        const segs = s.text.split("\n");
        for (let i = 0; i < segs.length; i++) {
          const w = ctx.measureText(segs[i]!).width;
          curW += w;
          if (i < segs.length - 1) {
            if (curW > maxLineW) maxLineW = curW;
            curW = 0;
          }
        }
      }
      if (curW > maxLineW) maxLineW = curW;
      const need = maxLineW + padX * 2 + indentLeft + indentRight;
      const leftCol = isMerged ? merge!.c1 : cell.c;
      const rightCol = isMerged ? merge!.c2 : cell.c;
      if (need > ownRect.w) {
        if (
          halign === "left" ||
          halign === "general" ||
          (halign === undefined && defaultAlign === "left")
        ) {
          let cc = rightCol + 1;
          while (cc <= g.maxCol && !occupied.has(`${cell.r}:${cc}`)) {
            clip.w += g.colW[cc] ?? 0;
            cc++;
            if (clip.w >= need) break;
          }
        } else if (halign === "right" || (halign === undefined && defaultAlign === "right")) {
          let cc = leftCol - 1;
          while (cc >= 1 && !occupied.has(`${cell.r}:${cc}`)) {
            const w = g.colW[cc] ?? 0;
            clip.x -= w;
            clip.w += w;
            cc--;
            if (clip.w >= need) break;
          }
        } else if (halign === "center" || defaultAlign === "center") {
          let cl = leftCol - 1,
            cr = rightCol + 1;
          let leftAdded = 0;
          let rightAdded = 0;
          const sideNeed = Math.max(0, (need - alignRect.w) / 2);
          while ((leftAdded < sideNeed || rightAdded < sideNeed) && (cl >= 1 || cr <= g.maxCol)) {
            let progressed = false;
            if (leftAdded < sideNeed && cl >= 1 && !occupied.has(`${cell.r}:${cl}`)) {
              const w = g.colW[cl] ?? 0;
              clip.x -= w;
              clip.w += w;
              leftAdded += w;
              cl--;
              progressed = true;
            }
            if (rightAdded < sideNeed && cr <= g.maxCol && !occupied.has(`${cell.r}:${cr}`)) {
              const w = g.colW[cr] ?? 0;
              clip.w += w;
              rightAdded += w;
              cr++;
              progressed = true;
            }
            if (!progressed) break;
          }
        }
      }
    }
    const innerW = Math.max(0, clip.w - padX * 2 - indentLeft - indentRight);

    // Layout into lines. Wrap mode auto-wraps to innerW; non-wrap mode
    // only line-breaks on \n. Single-line text (no wrap, no \n) takes a
    // fast path that preserves the legacy ellipsis-truncation behavior.
    const wantLayout = wrap || flatHasNewline || spans.length > 1;
    ctx.save();
    ctx.beginPath();
    ctx.rect(clip.x, clip.y, clip.w, clip.h);
    ctx.clip();
    ctx.textBaseline = "alphabetic";

    if (!wantLayout) {
      // Legacy fast path: one span, one line, optional ellipsis truncation.
      const span = spans[0]!;
      ctx.font = span.font;
      ctx.fillStyle = span.color;
      // Many workbook-authored "buttons" are just centered cells padded with
      // literal leading/trailing spaces. Excel visually centers the label, not
      // the invisible padding run; canvas includes those spaces in its advance
      // width, which makes button labels look off-center. For centered
      // single-line text, drop only outer padding whitespace for painting.
      let display = halign === "center" ? span.text.trim() || span.text : span.text;
      if (ctx.measureText(display).width > innerW && innerW > 8) {
        const ell = "…";
        let lo = 0,
          hi = display.length;
        while (lo < hi) {
          const mid = (lo + hi + 1) >> 1;
          if (ctx.measureText(display.slice(0, mid) + ell).width <= innerW) lo = mid;
          else hi = mid - 1;
        }
        display = display.slice(0, lo) + ell;
      }
      const tw = ctx.measureText(display).width;
      let tx: number;
      switch (halign) {
        case "center":
          tx = alignRect.x + (alignRect.w - tw) / 2;
          break;
        case "right":
          tx = alignRect.x + alignRect.w - padX - indentRight - tw;
          break;
        default:
          if (defaultAlign === "right" && !halign)
            tx = alignRect.x + alignRect.w - padX - indentRight - tw;
          else tx = textOriginX + padX + indentLeft;
      }
      const ascent = span.fontSizePx * 0.8;
      let ty: number;
      switch (valign) {
        case "top":
          ty = ownRect.y + ascent + 2;
          break;
        case "center":
          ty = ownRect.y + ownRect.h / 2 + ascent / 2 - 1;
          break;
        default:
          ty = ownRect.y + ownRect.h - 4;
      }
      const tyShift = ty + (span.baselineShiftPx ?? 0);
      ctx.fillText(display, tx, tyShift);
      paintTextDecorations(ctx, span, tx, tyShift, ctx.measureText(display).width, {
        x: clip.x + 1,
        w: Math.max(0, clip.w - 2),
      });
      ctx.restore();
      return;
    }

    // Multi-line / multi-run layout. Wrap to `innerW` when wrap=true;
    // otherwise only \n produces line breaks.
    const lines = layoutSpans(ctx, spans, innerW, wrap);
    const totalH = lines.reduce((a, l) => a + l.height, 0);

    // Vertical block placement inside the (possibly merged) rect.
    let blockTop: number;
    switch (valign) {
      case "top":
        blockTop = ownRect.y + 2;
        break;
      case "center":
        blockTop = ownRect.y + (ownRect.h - totalH) / 2;
        break;
      default:
        blockTop = ownRect.y + ownRect.h - totalH - 2;
    }

    let lineTop = blockTop;
    for (const line of lines) {
      // Horizontal placement of the entire line.
      let lineX: number;
      switch (halign) {
        case "center":
          lineX = alignRect.x + (alignRect.w - line.width) / 2;
          break;
        case "right":
          lineX = alignRect.x + alignRect.w - padX - indentRight - line.width;
          break;
        default:
          if (defaultAlign === "right" && !halign)
            lineX = alignRect.x + alignRect.w - padX - indentRight - line.width;
          else lineX = textOriginX + padX + indentLeft;
      }
      const baseline = lineTop + line.ascent;
      let cursorX = lineX;
      for (const piece of line.pieces) {
        ctx.font = piece.span.font;
        ctx.fillStyle = piece.span.color;
        const pieceBaseline = baseline + (piece.span.baselineShiftPx ?? 0);
        ctx.fillText(piece.text, cursorX, pieceBaseline);
        paintTextDecorations(ctx, piece.span, cursorX, pieceBaseline, piece.width, {
          x: clip.x + 1,
          w: Math.max(0, clip.w - 2),
        });
        cursorX += piece.width;
      }
      lineTop += line.height;
    }

    ctx.restore();
  });
}

// Has any visible content (text, number, or formula result)?
function hasContent(cell: Cell, sheet: Sheet, layout: WorkbookLayout): boolean {
  const xf = resolveCellXf(cell, sheet, layout);
  const { text } = resolveCellText(cell, layout, xf);
  return text.length > 0;
}

