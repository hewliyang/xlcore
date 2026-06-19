import type { Cell, Dxf, Font, Sheet, TextRun, WorkbookLayout } from "./types.js";
import { resolveCellText, resolveCellXf } from "./cellText.js";
import { colorToCss } from "./color.js";
import { iterCellsInRange } from "./columnar.js";

import type { Grid } from "./grid.js";
import { buildMergeMaps, rectFor } from "./geometry.js";
import type { Visible } from "./renderTypes.js";

interface Span {
  text: string;
  font: string;
  fontSizePx: number;
  color: string;
  bold: boolean;
  underline: boolean;

  underlineStyle?: string;
  strike: boolean;

  baselineShiftPx?: number;
}

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

  const baseName =
    resolveSchemeName(baseFont?.scheme, layout) ?? baseFont?.name ?? defaultFontFamily;
  const baseFamily = baseFont?.family;
  const baseBold = baseFont?.bold ?? false;
  const baseItalic = baseFont?.italic ?? false;
  const baseUnderline = baseFont?.underline ?? false;
  const baseUnderlineStyle = baseFont?.underlineStyle;
  const baseStrike = baseFont?.strike ?? false;

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

    const name = resolveSchemeName(r.scheme, layout) ?? r.fontName ?? baseName;
    const family = r.family ?? baseFamily;
    const bold = r.bold ?? baseBold;
    const italic = r.italic ?? baseItalic;
    const color = r.color ? colorToCss(r.color, baseColor) : baseColor;

    const underline = r.underline || baseUnderline;

    const underlineStyle = r.underlineStyle ?? baseUnderlineStyle;
    const strike = r.strike || baseStrike;

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

function resolveSchemeName(scheme: string | undefined, layout: WorkbookLayout): string | undefined {
  if (!scheme || scheme === "none") return undefined;
  const t = layout.theme;
  if (!t) return undefined;
  if (scheme === "major") return t.majorFont || undefined;
  if (scheme === "minor") return t.minorFont || undefined;
  return undefined;
}

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

  ctx.lineWidth = Math.max(1, span.fontSizePx / 16);
  if (span.underline) {
    const y = baseline + Math.max(1, span.fontSizePx * 0.12);
    const v = span.underlineStyle;
    const isAccounting = v === "singleAccounting" || v === "doubleAccounting";

    const ux = isAccounting && accountingExtent ? accountingExtent.x : x;
    const uw = isAccounting && accountingExtent ? accountingExtent.w : width;
    ctx.beginPath();
    ctx.moveTo(ux, y);
    ctx.lineTo(ux + uw, y);
    ctx.stroke();

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

interface LaidLine {
  pieces: { span: Span; text: string; width: number }[];
  width: number;
  height: number;
  ascent: number;
}

function layoutSpans(
  ctx: CanvasRenderingContext2D,
  spans: Span[],
  maxWidth: number,
  wrap: boolean,
): LaidLine[] {
  const lines: LaidLine[] = [];
  let current: LaidLine = { pieces: [], width: 0, height: 0, ascent: 0 };

  const finishLine = () => {
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

    const segs = span.text.split("\n");
    for (let si = 0; si < segs.length; si++) {
      const seg = segs[si]!;
      if (seg.length > 0) {
        if (!wrap) {
          pushPiece(span, seg, ctx.measureText(seg).width);
        } else {
          const tokens = seg.match(/\s+|\S+/g) ?? [];
          let buf = "";
          let bufW = 0;
          for (const tok of tokens) {
            const tokW = ctx.measureText(tok).width;

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
              ctx.font = span.font;

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

  const overflowFirstCol = Math.max(1, vis.firstCol - 8);
  const overflowLastCol = Math.min(g.maxCol, vis.lastCol + 8);

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

    if (fills && fills.length > 0 && text.includes("\u0001")) {
      const primary = spans[0]!;
      const prevFont = ctx.font;
      ctx.font = primary.font;

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

          const slice = avail / (fillCount - fi);
          const n = Math.max(0, Math.floor(slice / chW));
          avail -= n * chW;
          assembled += ch.repeat(n) + parts[fi + 1]!;
        }
        text = assembled;

        if (spans.length === 1) spans[0] = { ...spans[0]!, text };
      }
      ctx.font = prevFont;
    }

    const textRot = xf?.textRotation ?? 0;
    if (textRot !== 0) {
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
        const lineH = span.fontSizePx * 1.05;
        const ascent = span.fontSizePx * 0.8;
        const cx = ownRect.x + ownRect.w / 2;
        const totalH = lineH * text.length;

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

      const angleRad =
        textRot <= 90 ? (-textRot * Math.PI) / 180 : ((textRot - 90) * Math.PI) / 180;
      const tw = ctx.measureText(text).width;
      const ascent = span.fontSizePx * 0.8;
      const descent = span.fontSizePx * 0.2;
      const pad = 2;

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
      const anchorY = angleRad < 0 ? ownRect.y + ownRect.h - pad : ownRect.y + pad + ascent;
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

    const iconReserve = cfIconReserve.get(k) ?? 0;
    const textOriginX = ownRect.x + iconReserve;

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

    ctx.font = spans[0]?.font ?? `${(styles.defaultFontSize * 4) / 3}px sans-serif`;

    const flatHasNewline = text.indexOf("\n") >= 0;

    const alignRect = { ...ownRect };
    const clip = { ...ownRect };
    if (iconReserve > 0) {
      alignRect.x += iconReserve;
      alignRect.w -= iconReserve;
      clip.x += iconReserve;
      clip.w -= iconReserve;
    }
    if (halign === "centerContinuous") {
      const rightCol = isMerged ? merge!.c2 : cell.c;
      let cc = rightCol + 1;
      while (cc <= g.maxCol && !occupied.has(`${cell.r}:${cc}`)) {
        const w = g.colW[cc] ?? 0;
        alignRect.w += w;
        clip.w += w;
        cc++;
      }
    }
    if (!wrap) {
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

    const wantLayout = wrap || flatHasNewline || spans.length > 1;
    ctx.save();
    ctx.beginPath();
    ctx.rect(clip.x, clip.y, clip.w, clip.h);
    ctx.clip();
    ctx.textBaseline = "alphabetic";

    if (!wantLayout) {
      const span = spans[0]!;
      ctx.font = span.font;
      ctx.fillStyle = span.color;

      let display =
        halign === "center" || halign === "centerContinuous"
          ? span.text.trim() || span.text
          : span.text;
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
        case "centerContinuous":
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

    const lines = layoutSpans(ctx, spans, innerW, wrap);
    const totalH = lines.reduce((a, l) => a + l.height, 0);

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
      let lineX: number;
      switch (halign) {
        case "center":
        case "centerContinuous":
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

function hasContent(cell: Cell, sheet: Sheet, layout: WorkbookLayout): boolean {
  const xf = resolveCellXf(cell, sheet, layout);
  const { text } = resolveCellText(cell, layout, xf);
  return text.length > 0;
}

export function computeOverflowSuppressedSides(
  ctx: CanvasRenderingContext2D,
  sheet: Sheet,
  layout: WorkbookLayout,
  g: Grid,
  vis: Visible,
): Set<string> {
  const out = new Set<string>();
  const { covered, topLeftOf } = buildMergeMaps(sheet);
  const styles = layout.styles;

  const overflowFirstCol = Math.max(1, vis.firstCol - 8);
  const overflowLastCol = Math.min(g.maxCol, vis.lastCol + 8);

  const occupied = occupiedCellsInRange(
    sheet,
    layout,
    vis.firstRow,
    vis.lastRow,
    overflowFirstCol,
    overflowLastCol,
  );
  for (const k of covered) occupied.add(k);

  const prevFont = ctx.font;
  iterCellsInRange(sheet, vis.firstRow, vis.lastRow, overflowFirstCol, overflowLastCol, (cell) => {
    const k = `${cell.r}:${cell.c}`;
    if (covered.has(k)) return;
    if (topLeftOf.has(k)) return;

    const xf = resolveCellXf(cell, sheet, layout);
    const resolved = resolveCellText(cell, layout, xf);
    const { text, defaultAlign } = resolved;
    if (!text) return;
    const textRot = xf?.textRotation ?? 0;
    if (textRot !== 0) return;
    if (xf?.wrapText) return;
    if (text.indexOf("\n") >= 0) return;

    const ownRect = rectFor(sheet, g, cell.r, cell.c, topLeftOf);
    if (ownRect.w <= 0 || ownRect.h <= 0) return;

    const baseFontEntry = xf?.fontId !== undefined ? styles.fonts[xf.fontId] : undefined;
    const spans = resolveCellSpans(
      cell,
      text,
      layout,
      baseFontEntry,
      "#000000",
      styles.defaultFont,
      styles.defaultFontSize,
    );
    if (spans.length === 0) return;

    let measured = 0;
    for (const s of spans) {
      ctx.font = s.font;
      measured += ctx.measureText(s.text.replace(/\u0001/g, "")).width;
    }

    const halign = xf?.horizontalAlignment ?? defaultAlign;
    const padX = 4;
    const need = measured + padX * 2;
    if (need <= ownRect.w) return;

    const align: "left" | "right" | "center" =
      halign === "center"
        ? "center"
        : halign === "right"
          ? "right"
          : halign === "left"
            ? "left"
            : defaultAlign === "right"
              ? "right"
              : defaultAlign === "center"
                ? "center"
                : "left";

    const leftEmpty = cell.c > 1 && !occupied.has(`${cell.r}:${cell.c - 1}`);
    const rightEmpty = cell.c < g.maxCol && !occupied.has(`${cell.r}:${cell.c + 1}`);

    if (align === "left" && rightEmpty) {
      out.add(`${cell.r}:${cell.c}:right`);
    } else if (align === "right" && leftEmpty) {
      out.add(`${cell.r}:${cell.c}:left`);
    } else if (align === "center") {
      const sideNeed = (need - ownRect.w) / 2;
      if (sideNeed > 0 && leftEmpty) out.add(`${cell.r}:${cell.c}:left`);
      if (sideNeed > 0 && rightEmpty) out.add(`${cell.r}:${cell.c}:right`);
    }
  });
  ctx.font = prevFont;
  return out;
}
