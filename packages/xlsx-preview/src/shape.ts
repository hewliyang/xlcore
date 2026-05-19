import type { Shape, ShapeNode, ShapeParagraph } from "./types.js";
import { getOrLoadImage } from "./imageCache.js";

// ---------- shape painter ----------
//
// v0 paints DrawingML autoshapes (`<xdr:sp>`) extracted by the Rust side
// as a flat list of nodes positioned in fractional bbox-of-the-anchor
// coordinates. Each leaf carries:
//   - fill / outline color and width
//   - prstGeom token (rect / roundRect / ellipse / leftArrow / ...)
//   - paragraphs with text runs (bold / size / color / font)
//
// Unknown presets fall back to a plain rectangle. Excel's preset
// vocabulary is huge (~200 shapes) — we only special-case the common
// ones (rect, roundRect, ellipse, arrows). The remit per PARITY.md is
// "a rect + fill + text v0 would make most decorative chrome legible";
// we go slightly past that.

const DEFAULT_FONT_PT = 11;
const PT_PER_PX = 0.75; // 1pt ≈ 1.333px at 96dpi → px = pt / 0.75
const PX_PER_EMU = 1 / 9525;

export function drawShape(
  ctx: CanvasRenderingContext2D,
  shape: Shape,
  rect: { x: number; y: number; w: number; h: number },
): void {
  for (const node of shape.nodes) {
    const nx = rect.x + node.relX * rect.w;
    const ny = rect.y + node.relY * rect.h;
    const nw = node.relW * rect.w;
    const nh = node.relH * rect.h;
    if (nw < 1 || nh < 1) continue;
    drawShapeNode(ctx, node, nx, ny, nw, nh);
  }
}

function drawShapeNode(
  ctx: CanvasRenderingContext2D,
  node: ShapeNode,
  x: number,
  y: number,
  w: number,
  h: number,
): void {
  // Honor `<a:xfrm rot="...">`. Excel stores rotation in 1/60000°.
  const rotation = node.rotation
    ? (node.rotation / 60000) * (Math.PI / 180)
    : 0;
  ctx.save();
  if (rotation) {
    ctx.translate(x + w / 2, y + h / 2);
    ctx.rotate(rotation);
    x = -w / 2;
    y = -h / 2;
  }

  // Inline picture node (nested `<xdr:pic>` inside `<xdr:grpSp>`).
  // Bypass fill / outline / text — just paint the bitmap (honoring
  // `<a:srcRect>` crop if present) and we're done.
  if (node.imageDataUri) {
    drawShapeImage(ctx, node, x, y, w, h);
    ctx.restore();
    return;
  }

  // Path geometry by preset.
  const preset = node.preset ?? "rect";
  pathForPreset(ctx, preset, x, y, w, h);

  if (node.fill) {
    ctx.fillStyle = node.fill;
    ctx.fill();
  }

  if (node.outlineColor) {
    const widthEmu = node.outlineWidthEmu;
    // OOXML default `<a:ln>` width when unspecified is 9525 EMU (1pt).
    // 0 EMU is a hairline (Excel renders as 0.5px).
    const widthPx =
      widthEmu == null
        ? 1.0
        : widthEmu === 0
          ? 0.5
          : Math.max(0.5, widthEmu * PX_PER_EMU);
    ctx.strokeStyle = node.outlineColor;
    ctx.lineWidth = widthPx;
    ctx.stroke();
  }

  if ((node.paragraphs?.length ?? 0) > 0) {
    drawShapeText(ctx, node, x, y, w, h);
  }

  ctx.restore();
}

function pathForPreset(
  ctx: CanvasRenderingContext2D,
  preset: string,
  x: number,
  y: number,
  w: number,
  h: number,
): void {
  ctx.beginPath();
  switch (preset) {
    case "ellipse":
    case "circle": {
      ctx.ellipse(x + w / 2, y + h / 2, w / 2, h / 2, 0, 0, Math.PI * 2);
      break;
    }
    case "roundRect": {
      // Excel's default rounded-rect adjust value is ~16.7% of the
      // shorter side (`adj1=16667` in OOXML's per-mil units).
      const r = Math.min(w, h) * 0.16;
      roundRectPath(ctx, x, y, w, h, r);
      break;
    }
    case "leftArrow":
      arrowPath(ctx, x, y, w, h, "left");
      break;
    case "rightArrow":
      arrowPath(ctx, x, y, w, h, "right");
      break;
    case "upArrow":
      arrowPath(ctx, x, y, w, h, "up");
      break;
    case "downArrow":
      arrowPath(ctx, x, y, w, h, "down");
      break;
    case "triangle":
      ctx.moveTo(x + w / 2, y);
      ctx.lineTo(x + w, y + h);
      ctx.lineTo(x, y + h);
      ctx.closePath();
      break;
    case "diamond":
      ctx.moveTo(x + w / 2, y);
      ctx.lineTo(x + w, y + h / 2);
      ctx.lineTo(x + w / 2, y + h);
      ctx.lineTo(x, y + h / 2);
      ctx.closePath();
      break;
    default:
      // Unknown / `rect` / any custom geometry: plain rectangle.
      ctx.rect(x, y, w, h);
  }
}

function roundRectPath(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  w: number,
  h: number,
  r: number,
): void {
  ctx.moveTo(x + r, y);
  ctx.lineTo(x + w - r, y);
  ctx.quadraticCurveTo(x + w, y, x + w, y + r);
  ctx.lineTo(x + w, y + h - r);
  ctx.quadraticCurveTo(x + w, y + h, x + w - r, y + h);
  ctx.lineTo(x + r, y + h);
  ctx.quadraticCurveTo(x, y + h, x, y + h - r);
  ctx.lineTo(x, y + r);
  ctx.quadraticCurveTo(x, y, x + r, y);
  ctx.closePath();
}

function arrowPath(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  w: number,
  h: number,
  dir: "left" | "right" | "up" | "down",
): void {
  // OOXML adj1 default ~50000 (head depth = 50% of shape on the arrow
  // axis); adj2 default ~50000 (tail thickness = 50% of cross-axis).
  // We hardcode these; per-shape avLst overrides aren't extracted yet.
  if (dir === "left" || dir === "right") {
    const head = w * 0.5;
    const tail = h * 0.5;
    const tailY1 = y + (h - tail) / 2;
    const tailY2 = tailY1 + tail;
    if (dir === "right") {
      ctx.moveTo(x, tailY1);
      ctx.lineTo(x + w - head, tailY1);
      ctx.lineTo(x + w - head, y);
      ctx.lineTo(x + w, y + h / 2);
      ctx.lineTo(x + w - head, y + h);
      ctx.lineTo(x + w - head, tailY2);
      ctx.lineTo(x, tailY2);
      ctx.closePath();
    } else {
      ctx.moveTo(x + w, tailY1);
      ctx.lineTo(x + head, tailY1);
      ctx.lineTo(x + head, y);
      ctx.lineTo(x, y + h / 2);
      ctx.lineTo(x + head, y + h);
      ctx.lineTo(x + head, tailY2);
      ctx.lineTo(x + w, tailY2);
      ctx.closePath();
    }
  } else {
    const head = h * 0.5;
    const tail = w * 0.5;
    const tailX1 = x + (w - tail) / 2;
    const tailX2 = tailX1 + tail;
    if (dir === "down") {
      ctx.moveTo(tailX1, y);
      ctx.lineTo(tailX1, y + h - head);
      ctx.lineTo(x, y + h - head);
      ctx.lineTo(x + w / 2, y + h);
      ctx.lineTo(x + w, y + h - head);
      ctx.lineTo(tailX2, y + h - head);
      ctx.lineTo(tailX2, y);
      ctx.closePath();
    } else {
      ctx.moveTo(tailX1, y + h);
      ctx.lineTo(tailX1, y + head);
      ctx.lineTo(x, y + head);
      ctx.lineTo(x + w / 2, y);
      ctx.lineTo(x + w, y + head);
      ctx.lineTo(tailX2, y + head);
      ctx.lineTo(tailX2, y + h);
      ctx.closePath();
    }
  }
}

function drawShapeImage(
  ctx: CanvasRenderingContext2D,
  node: ShapeNode,
  x: number,
  y: number,
  w: number,
  h: number,
): void {
  const uri = node.imageDataUri;
  if (!uri) return;
  const img = getOrLoadImage(uri);
  if (!img) {
    // Faint placeholder while we decode.
    ctx.fillStyle = "#f4f4f5";
    ctx.fillRect(x, y, w, h);
    return;
  }
  const naturalW = (img.naturalWidth ?? img.width ?? 0) || 0;
  const naturalH = (img.naturalHeight ?? img.height ?? 0) || 0;
  if (naturalW <= 0 || naturalH <= 0) {
    ctx.drawImage(img as CanvasImageSource, x, y, w, h);
    return;
  }
  // `<a:srcRect l t r b/>` crop, in 1/1000 percent of natural size.
  // l/t inset the source from top-left, r/b inset from the opposite
  // edge — i.e. source rect = [l*W..(1-r)*W, t*H..(1-b)*H].
  let sx = 0,
    sy = 0,
    sw = naturalW,
    sh = naturalH;
  const crop = node.imageSrcRect;
  if (crop && crop.length === 4) {
    const [l, t, r, b] = crop;
    const lf = (l ?? 0) / 100000;
    const tf = (t ?? 0) / 100000;
    const rf = (r ?? 0) / 100000;
    const bf = (b ?? 0) / 100000;
    sx = naturalW * lf;
    sy = naturalH * tf;
    sw = naturalW * Math.max(0, 1 - lf - rf);
    sh = naturalH * Math.max(0, 1 - tf - bf);
  }
  if (sw > 0 && sh > 0) {
    ctx.drawImage(img as CanvasImageSource, sx, sy, sw, sh, x, y, w, h);
  } else {
    ctx.drawImage(img as CanvasImageSource, x, y, w, h);
  }
}

function drawShapeText(
  ctx: CanvasRenderingContext2D,
  node: ShapeNode,
  x: number,
  y: number,
  w: number,
  h: number,
): void {
  // Paragraph stack with per-paragraph word-wrap. Wrap policy follows
  // `<a:bodyPr wrap="..."/>`: `square` (default) wraps on word
  // boundaries to fit the inner width; `none` lets text overflow
  // horizontally. Vertical clipping at innerH is honored regardless.
  //
  // Inset policy follows `<a:bodyPr lIns/tIns/rIns/bIns/>` when
  // present; otherwise we apply the DrawingML defaults (lIns/rIns =
  // 91440 EMU ≈ 9.6px @ 96dpi; tIns/bIns = 45720 EMU ≈ 4.8px). The
  // old fallback was a 4%-of-shape magic margin which both bled
  // unevenly across small shapes (narrow text boxes lost almost all
  // their inner width) and didn't match Excel's measured padding.
  const DEFAULT_LR_EMU = 91440;
  const DEFAULT_TB_EMU = 45720;
  const ins = node.textInsetsEmu;
  const lEmu = ins?.[0] ?? DEFAULT_LR_EMU;
  const tEmu = ins?.[1] ?? DEFAULT_TB_EMU;
  const rEmu = ins?.[2] ?? DEFAULT_LR_EMU;
  const bEmu = ins?.[3] ?? DEFAULT_TB_EMU;
  const lPad = lEmu * PX_PER_EMU;
  const tPad = tEmu * PX_PER_EMU;
  const rPad = rEmu * PX_PER_EMU;
  const bPad = bEmu * PX_PER_EMU;
  const innerX = x + lPad;
  const innerY = y + tPad;
  const innerW = Math.max(1, w - lPad - rPad);
  const innerH = Math.max(1, h - tPad - bPad);
  const wrap = node.textWrap !== "none";

  // Pre-wrap each paragraph into visual lines so we can vertically
  // anchor the whole block correctly when `textAnchor` is `ctr`/`b`.
  type WrappedLine = {
    runs: { r: ShapeParagraph["runs"][number]; width: number; font: string }[];
    align: ShapeParagraph["align"];
    lineHeight: number;
    width: number;
  };
  const lines: WrappedLine[] = [];
  let totalH = 0;
  for (const p of node.paragraphs ?? []) {
    const wrapped = wrapParagraph(ctx, p, innerW, wrap);
    for (const ln of wrapped) {
      lines.push(ln);
      totalH += ln.lineHeight;
    }
    if (wrapped.length === 0) {
      // Empty paragraph still occupies a line height (matches Excel).
      const lineH = paragraphLineHeight(p);
      lines.push({ runs: [], align: p.align, lineHeight: lineH, width: 0 });
      totalH += lineH;
    }
  }

  let cursorY: number;
  switch (node.textAnchor) {
    case "ctr":
      cursorY = innerY + (innerH - totalH) / 2;
      break;
    case "b":
      cursorY = innerY + innerH - totalH;
      break;
    default:
      cursorY = innerY;
  }
  if (cursorY < innerY) cursorY = innerY;

  for (const ln of lines) {
    if (cursorY + ln.lineHeight > innerY + innerH + 0.5) break;
    drawWrappedLine(ctx, ln, innerX, cursorY, innerW);
    cursorY += ln.lineHeight;
  }
}

function wrapParagraph(
  ctx: CanvasRenderingContext2D,
  p: ShapeParagraph,
  maxWidth: number,
  wrap: boolean,
): {
  runs: { r: ShapeParagraph["runs"][number]; width: number; font: string }[];
  align: ShapeParagraph["align"];
  lineHeight: number;
  width: number;
}[] {
  // Tokenize each run into (text, isSpace, isBreak) atoms. Hard line
  // breaks (`\n`, surfaced from `<a:br/>`) force a new line. Within a
  // run, we split on whitespace runs but keep the spaces attached to
  // the preceding word so adjacent runs don't lose their join.
  type Atom = {
    text: string;
    isBreak: boolean;
    r: ShapeParagraph["runs"][number];
    font: string;
  };
  const atoms: Atom[] = [];
  for (const r of p.runs ?? []) {
    const font = runFont(r);
    if (r.text === "\n") {
      atoms.push({ text: "", isBreak: true, r, font });
      continue;
    }
    // Split into segments at every whitespace boundary, keeping the
    // whitespace at the END of each segment. `[^\s]+\s*|\s+` handles
    // both leading-space and trailing-space cases.
    const segs = r.text.match(/\S+\s*|\s+/g) ?? [];
    for (const seg of segs) {
      atoms.push({ text: seg, isBreak: false, r, font });
    }
  }

  type Line = {
    runs: { r: ShapeParagraph["runs"][number]; width: number; font: string }[];
    align: ShapeParagraph["align"];
    lineHeight: number;
    width: number;
  };
  const lines: Line[] = [];
  let cur: Line | null = null;
  let maxFontPt = DEFAULT_FONT_PT;

  const startLine = () => {
    cur = { runs: [], align: p.align, lineHeight: 0, width: 0 };
    lines.push(cur);
    maxFontPt = DEFAULT_FONT_PT;
  };

  const finishLine = () => {
    if (!cur) return;
    cur.lineHeight = Math.ceil((maxFontPt / PT_PER_PX) * 1.2);
  };

  for (const a of atoms) {
    if (a.isBreak) {
      if (!cur) startLine();
      finishLine();
      startLine();
      continue;
    }
    if (!cur) startLine();
    ctx.font = a.font;
    const segW = ctx.measureText(a.text).width;
    const pt = a.r.size ?? DEFAULT_FONT_PT;
    if (pt > maxFontPt) maxFontPt = pt;
    // Decide whether to break before this atom. Don't break on the
    // first atom of a line (avoids infinite-narrow-box loops), and
    // skip the break check when wrapping is disabled.
    if (
      wrap &&
      cur!.runs.length > 0 &&
      cur!.width + segW > maxWidth &&
      !/^\s+$/.test(a.text) // pure-whitespace seg — keep on this line so it collapses at EOL
    ) {
      finishLine();
      startLine();
      ctx.font = a.font; // restart in case font changed (it didn't, but cheap)
      if (pt > maxFontPt) maxFontPt = pt;
    }
    // Merge with previous run on the same line if identical font.
    const last = cur!.runs[cur!.runs.length - 1];
    if (last && last.font === a.font && last.r === a.r) {
      last.r = { ...last.r, text: last.r.text + a.text };
      last.width += segW;
    } else {
      cur!.runs.push({
        r: { ...a.r, text: a.text },
        width: segW,
        font: a.font,
      });
    }
    cur!.width += segW;
  }
  finishLine();
  // Drop trailing empty line caused by a final `\n`.
  if (lines.length > 0 && lines[lines.length - 1]!.runs.length === 0) {
    lines.pop();
  }
  return lines;
}

function drawWrappedLine(
  ctx: CanvasRenderingContext2D,
  ln: {
    runs: { r: ShapeParagraph["runs"][number]; width: number; font: string }[];
    align: ShapeParagraph["align"];
    lineHeight: number;
    width: number;
  },
  x: number,
  y: number,
  w: number,
): void {
  if (ln.runs.length === 0) return;
  // Trim trailing-whitespace width from the alignment measurement so
  // right/center-aligned lines don't visually drift by half a space.
  const last = ln.runs[ln.runs.length - 1]!;
  const trailingMatch = last.r.text.match(/\s+$/);
  let alignWidth = ln.width;
  if (trailingMatch) {
    ctx.font = last.font;
    alignWidth -= ctx.measureText(trailingMatch[0]!).width;
  }
  let cursorX: number;
  switch (ln.align) {
    case "ctr":
      cursorX = x + (w - alignWidth) / 2;
      break;
    case "r":
      cursorX = x + w - alignWidth;
      break;
    default:
      cursorX = x;
  }
  const baselineY = y + ln.lineHeight * 0.82;
  for (const m of ln.runs) {
    ctx.font = m.font;
    ctx.textBaseline = "alphabetic";
    ctx.textAlign = "left";
    const color = m.r.color?.rgb ? `#${m.r.color.rgb.slice(-6)}` : "#000000";
    ctx.fillStyle = color;
    ctx.fillText(m.r.text, cursorX, baselineY);
    if (m.r.underline) {
      ctx.strokeStyle = color;
      ctx.lineWidth = 1;
      ctx.beginPath();
      ctx.moveTo(cursorX, baselineY + 2);
      ctx.lineTo(cursorX + m.width, baselineY + 2);
      ctx.stroke();
    }
    if (m.r.strike) {
      ctx.strokeStyle = color;
      ctx.lineWidth = 1;
      ctx.beginPath();
      ctx.moveTo(cursorX, baselineY - 4);
      ctx.lineTo(cursorX + m.width, baselineY - 4);
      ctx.stroke();
    }
    cursorX += m.width;
  }
}

function paragraphLineHeight(p: ShapeParagraph): number {
  let maxPt = DEFAULT_FONT_PT;
  for (const r of p.runs ?? []) {
    if (r.size && r.size > maxPt) maxPt = r.size;
  }
  // Line height ≈ 1.2 × font px height.
  return Math.ceil((maxPt / PT_PER_PX) * 1.2);
}

function runFont(r: {
  size?: number;
  bold?: boolean;
  italic?: boolean;
  fontName?: string;
}): string {
  const pt = r.size ?? DEFAULT_FONT_PT;
  const px = pt / PT_PER_PX;
  const family = r.fontName
    ? `"${r.fontName}", -apple-system, "Helvetica Neue", Arial, sans-serif`
    : '-apple-system, "Helvetica Neue", Arial, sans-serif';
  const weight = r.bold ? "700" : "400";
  const style = r.italic ? "italic " : "";
  return `${style}${weight} ${px}px ${family}`;
}
