import type { ShapeNode, ShapeParagraph } from "./types.js";
import {
  buildBulletGlyph,
  computeLineHeight,
  computeParaSpacing,
  drawBullet,
  NumberingState,
  paragraphRefFontPt,
} from "./shapeBullets.js";

const DEFAULT_FONT_PT = 11;
const PT_PER_PX = 0.75;
const PX_PER_EMU = 1 / 9525;

export function drawShapeText(
  ctx: CanvasRenderingContext2D,
  node: ShapeNode,
  x: number,
  y: number,
  w: number,
  h: number,
  flipH: boolean = false,
  flipV: boolean = false,
): void {
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
  const textRect = presetTextRect(node.preset, w, h);
  if (flipH) textRect.x = w - textRect.x - textRect.w;
  if (flipV) textRect.y = h - textRect.y - textRect.h;
  const baseX = x + textRect.x;
  const baseY = y + textRect.y;
  const baseW = textRect.w;
  const baseH = textRect.h;
  const innerXOrig = baseX + lPad;
  const innerYOrig = baseY + tPad;
  const innerWOrig = Math.max(1, baseW - lPad - rPad);
  const innerHOrig = Math.max(1, baseH - tPad - bPad);

  const vertDeg = (() => {
    switch (node.textVert) {
      case "vert":
      case "wordArtVert":
      case "eaVert":
      case "mongolianVert":
        return 90;
      case "vert270":
      case "wordArtVertRtl":
        return -90;
      default:
        return 0;
    }
  })();
  const bodyRotDeg = (node.textRotation ?? 0) / 60000;
  const effDeg = bodyRotDeg + vertDeg;
  const thetaRad = (effDeg * Math.PI) / 180;
  const modDeg = ((effDeg % 180) + 180) % 180;
  const isPerp = Math.abs(modDeg - 90) < 1;
  const hasRot = Math.abs(thetaRad) > 1e-6;

  const innerW = isPerp ? innerHOrig : innerWOrig;
  const innerH = isPerp ? innerWOrig : innerHOrig;
  const vertOverflow = node.textVertOverflow ?? "overflow";
  const horzOverflow = node.textHorzOverflow ?? "overflow";
  const clipNeeded = vertOverflow !== "overflow" || horzOverflow === "clip";
  const needSave = hasRot || clipNeeded;
  let innerX: number;
  let innerY: number;
  if (hasRot) {
    const cx = innerXOrig + innerWOrig / 2;
    const cy = innerYOrig + innerHOrig / 2;
    ctx.save();
    ctx.translate(cx, cy);
    ctx.rotate(thetaRad);
    innerX = -innerW / 2;
    innerY = -innerH / 2;
  } else {
    if (needSave) ctx.save();
    innerX = innerXOrig;
    innerY = innerYOrig;
  }
  if (clipNeeded) {
    ctx.beginPath();
    ctx.rect(innerX, innerY, innerW, innerH);
    ctx.clip();
  }
  const wrap = node.textWrap !== "none";

  const fontScale =
    node.textAutofit === "norm" && node.textFontScale != null
      ? Math.max(0.01, Math.min(1, node.textFontScale / 100000))
      : 1;
  const lineScale =
    node.textAutofit === "norm" && node.textLineSpaceReduction != null
      ? Math.max(0.1, 1 - node.textLineSpaceReduction / 100000)
      : 1;

  type WrappedLine = {
    runs: { r: ShapeParagraph["runs"][number]; width: number; font: string }[];
    align: ShapeParagraph["align"];
    lineHeight: number;
    width: number;
  };
  type LaidOutPara = {
    p: ShapeParagraph;
    marL: number;
    indent: number;
    spcBef: number;
    spcAft: number;
    bullet: ReturnType<typeof buildBulletGlyph>;
    lines: WrappedLine[];
    height: number; // sum of line heights only
  };
  const paras: LaidOutPara[] = [];
  const numbering = new NumberingState();
  let totalH = 0;
  for (const p of node.paragraphs ?? []) {
    const marL = Math.max(0, (p.marLEmu ?? 0) * PX_PER_EMU);
    const indent = (p.indentEmu ?? 0) * PX_PER_EMU;
    const refPt = paragraphRefFontPt(p);
    const lnSpcPx = computeLineHeight(p, refPt, fontScale, lineScale);
    const spcBef = computeParaSpacing(p.spaceBeforePct, p.spaceBeforePts, refPt, fontScale);
    const spcAft = computeParaSpacing(p.spaceAfterPct, p.spaceAfterPts, refPt, fontScale);
    const bullet = buildBulletGlyph(ctx, p, refPt, fontScale, numbering);
    const wrapWidth = Math.max(1, innerW - marL);
    const wrapped = wrapParagraph(ctx, p, wrapWidth, wrap, fontScale, lineScale, lnSpcPx);
    if (wrapped.length === 0) {
      wrapped.push({ runs: [], align: p.align, lineHeight: lnSpcPx, width: 0 });
    }
    const height = wrapped.reduce((acc, ln) => acc + ln.lineHeight, 0);
    paras.push({ p, marL, indent, spcBef, spcAft, bullet, lines: wrapped, height });
    totalH += spcBef + height + spcAft;
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

  const bottom = innerY + innerH;
  outer: for (const para of paras) {
    cursorY += para.spcBef;
    for (let i = 0; i < para.lines.length; i++) {
      const ln = para.lines[i]!;
      const isFirst = i === 0;
      const lineX = innerX + para.marL;
      const lineW = Math.max(1, innerW - para.marL);
      if (vertOverflow !== "overflow") {
        if (cursorY > bottom + 0.5) break outer;
        if (vertOverflow === "ellipsis") {
          const next = cursorY + ln.lineHeight;
          const moreFollows = i < para.lines.length - 1;
          if (
            (next > bottom + 0.5 ||
              (moreFollows && next + para.lines[i + 1]!.lineHeight > bottom + 0.5)) &&
            ln.runs.length > 0
          ) {
            if (isFirst && para.bullet) {
              drawBullet(
                ctx,
                para.bullet,
                innerX + para.marL + para.indent,
                cursorY,
                ln.lineHeight,
              );
            }
            drawWrappedLineWithEllipsis(ctx, ln, lineX, cursorY, lineW);
            break outer;
          }
        }
      }
      if (isFirst && para.bullet) {
        drawBullet(ctx, para.bullet, innerX + para.marL + para.indent, cursorY, ln.lineHeight);
      }
      drawWrappedLine(ctx, ln, lineX, cursorY, lineW);
      cursorY += ln.lineHeight;
    }
    cursorY += para.spcAft;
  }
  if (needSave) ctx.restore();
}

function drawWrappedLineWithEllipsis(
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
  const ELLIPSIS = "\u2026";
  const runs = ln.runs.slice();
  const measure = (text: string, font: string) => {
    ctx.save();
    ctx.font = font;
    const m = ctx.measureText(text).width;
    ctx.restore();
    return m;
  };
  while (runs.length > 0) {
    const tail = runs[runs.length - 1]!;
    let txt = tail.r.text + ELLIPSIS;
    let totalW = 0;
    for (let i = 0; i < runs.length - 1; i++) totalW += runs[i]!.width;
    let tailW = measure(txt, tail.font);
    while (txt.length > 1 && totalW + tailW > w + 0.5) {
      txt = txt.slice(0, -2) + ELLIPSIS;
      tailW = measure(txt, tail.font);
    }
    if (totalW + tailW <= w + 0.5 || runs.length === 1) {
      const newRuns = runs.slice(0, -1);
      newRuns.push({ ...tail, r: { ...tail.r, text: txt }, width: tailW });
      drawWrappedLine(ctx, { ...ln, runs: newRuns, width: totalW + tailW }, x, y, w);
      return;
    }
    runs.pop();
  }
  ctx.fillText(ELLIPSIS, x, y);
}

function presetTextRect(
  preset: string | undefined,
  w: number,
  h: number,
): { x: number; y: number; w: number; h: number } {
  switch (preset) {
    case "triangle": {
      const tw = w * 0.5;
      const th = h * 0.5;
      return { x: (w - tw) / 2, y: h * 0.5, w: tw, h: th };
    }
    case "diamond":
    case "flowChartDecision": {
      const tw = w * 0.5;
      const th = h * 0.5;
      return { x: (w - tw) / 2, y: (h - th) / 2, w: tw, h: th };
    }
    case "ellipse":
    case "circle": {
      const k = 0.7071;
      const tw = w * k;
      const th = h * k;
      return { x: (w - tw) / 2, y: (h - th) / 2, w: tw, h: th };
    }
    case "chevron": {
      const inset = h * 0.5;
      return { x: inset, y: 0, w: Math.max(1, w - inset * 2), h };
    }
    case "homePlate":
    case "pentagon": {
      const pt = Math.min(w * 0.5, h * 0.5);
      return { x: 0, y: 0, w: Math.max(1, w - pt), h };
    }
    case "hexagon": {
      const inset = w * 0.25;
      return { x: inset, y: 0, w: Math.max(1, w - inset * 2), h };
    }
    case "star5":
    case "star4":
    case "star6":
    case "star8": {
      const k = 0.5;
      const tw = w * k;
      const th = h * k;
      return { x: (w - tw) / 2, y: (h - th) / 2, w: tw, h: th };
    }
    case "leftArrow":
    case "rightArrow": {
      const head = w * 0.5;
      const ty = h * 0.25;
      if (preset === "rightArrow") {
        return { x: 0, y: ty, w: Math.max(1, w - head), h: h * 0.5 };
      }
      return { x: head, y: ty, w: Math.max(1, w - head), h: h * 0.5 };
    }
    case "upArrow":
    case "downArrow": {
      const head = h * 0.5;
      const tx = w * 0.25;
      if (preset === "downArrow") {
        return { x: tx, y: 0, w: w * 0.5, h: Math.max(1, h - head) };
      }
      return { x: tx, y: head, w: w * 0.5, h: Math.max(1, h - head) };
    }
    default:
      return { x: 0, y: 0, w, h };
  }
}

function wrapParagraph(
  ctx: CanvasRenderingContext2D,
  p: ShapeParagraph,
  maxWidth: number,
  wrap: boolean,
  fontScale: number = 1,
  lineScale: number = 1,
  lineHeightOverride?: number,
): {
  runs: { r: ShapeParagraph["runs"][number]; width: number; font: string }[];
  align: ShapeParagraph["align"];
  lineHeight: number;
  width: number;
}[] {
  type Atom = {
    text: string;
    isBreak: boolean;
    r: ShapeParagraph["runs"][number];
    font: string;
  };
  const atoms: Atom[] = [];
  for (const r of p.runs ?? []) {
    const font = runFont(r, fontScale);
    if (r.text === "\n") {
      atoms.push({ text: "", isBreak: true, r, font });
      continue;
    }

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
    cur.lineHeight =
      lineHeightOverride ?? Math.ceil(((maxFontPt * fontScale) / PT_PER_PX) * 1.2 * lineScale);
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

    if (wrap && cur!.runs.length > 0 && cur!.width + segW > maxWidth && !/^\s+$/.test(a.text)) {
      finishLine();
      startLine();
      ctx.font = a.font;
      if (pt > maxFontPt) maxFontPt = pt;
    }

    if (wrap && segW > maxWidth && !/^\s+$/.test(a.text)) {
      const chars = Array.from(a.text);
      let buf = "";
      let bufW = 0;
      const flushBuf = () => {
        if (!buf) return;
        const last = cur!.runs[cur!.runs.length - 1];
        if (last && last.font === a.font && last.r === a.r) {
          last.r = { ...last.r, text: last.r.text + buf };
          last.width += bufW;
        } else {
          cur!.runs.push({
            r: { ...a.r, text: buf },
            width: bufW,
            font: a.font,
          });
        }
        cur!.width += bufW;
        buf = "";
        bufW = 0;
      };
      for (const ch of chars) {
        const cw = ctx.measureText(ch).width;
        if (cur!.runs.length > 0 || buf.length > 0) {
          if (cur!.width + bufW + cw > maxWidth) {
            flushBuf();
            finishLine();
            startLine();
            ctx.font = a.font;
            if (pt > maxFontPt) maxFontPt = pt;
          }
        }
        buf += ch;
        bufW += cw;
      }
      flushBuf();
      continue;
    }

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
    const baselineRaw = m.r.baseline ?? 0;
    const isSuperSub = baselineRaw !== 0;
    const runPt = m.r.size ?? DEFAULT_FONT_PT;
    const baselineOffsetPx = isSuperSub ? -(baselineRaw / 100000) * (runPt / PT_PER_PX) : 0;
    const drawFont = isSuperSub
      ? m.font.replace(/(\d+(?:\.\d+)?)px/, (_, n) => `${Number(n) * 0.65}px`)
      : m.font;
    ctx.font = drawFont;
    ctx.textBaseline = "alphabetic";
    ctx.textAlign = "left";
    const color = m.r.color?.rgb ? `#${m.r.color.rgb.slice(-6)}` : "#000000";
    ctx.fillStyle = color;
    const drawY = baselineY + baselineOffsetPx;
    ctx.fillText(m.r.text, cursorX, drawY);
    if (m.r.underline) {
      ctx.strokeStyle = color;
      ctx.lineWidth = 1;
      ctx.beginPath();
      ctx.moveTo(cursorX, drawY + 2);
      ctx.lineTo(cursorX + m.width, drawY + 2);
      ctx.stroke();
    }
    if (m.r.strike) {
      ctx.strokeStyle = color;
      ctx.lineWidth = 1;
      ctx.beginPath();
      ctx.moveTo(cursorX, drawY - 4);
      ctx.lineTo(cursorX + m.width, drawY - 4);
      ctx.stroke();
    }
    cursorX += m.width;
  }
}

function runFont(
  r: {
    size?: number;
    bold?: boolean;
    italic?: boolean;
    fontName?: string;
  },
  fontScale: number = 1,
): string {
  const pt = (r.size ?? DEFAULT_FONT_PT) * fontScale;
  const px = pt / PT_PER_PX;
  const family = r.fontName
    ? `"${r.fontName}", -apple-system, "Helvetica Neue", Arial, sans-serif`
    : '-apple-system, "Helvetica Neue", Arial, sans-serif';
  const weight = r.bold ? "700" : "400";
  const style = r.italic ? "italic " : "";
  return `${style}${weight} ${px}px ${family}`;
}
