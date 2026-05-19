import type { Shape, ShapeNode, ShapeParagraph } from "./types.js";
import { getOrLoadImage } from "./imageCache.js";

import { isBraceLikePreset, isLinePreset, pathForPreset } from "./shapePaths.js";
const DEFAULT_FONT_PT = 11;
const PT_PER_PX = 0.75;
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

    const isLine = node.isConnector === true || isLinePreset(node.preset);
    if (!isLine && (nw < 1 || nh < 1)) continue;
    if (isLine && nw < 0.5 && nh < 0.5) continue;
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
  const rotation = node.rotation ? (node.rotation / 60000) * (Math.PI / 180) : 0;
  ctx.save();
  if (rotation) {
    ctx.translate(x + w / 2, y + h / 2);
    ctx.rotate(rotation);
    x = -w / 2;
    y = -h / 2;
  }

  if (node.imageDataUri) {
    drawShapeImage(ctx, node, x, y, w, h);
    ctx.restore();
    return;
  }

  if (node.isConnector || isLinePreset(node.preset)) {
    drawConnector(ctx, node, x, y, w, h);
    ctx.restore();
    return;
  }

  const flipH = !!node.flipH;
  const flipV = !!node.flipV;
  if (flipH || flipV) {
    ctx.translate(x + w / 2, y + h / 2);
    ctx.scale(flipH ? -1 : 1, flipV ? -1 : 1);
    ctx.translate(-(x + w / 2), -(y + h / 2));
  }

  const preset = node.preset ?? "rect";
  pathForPreset(ctx, preset, x, y, w, h, node);

  if (node.fill) {
    ctx.fillStyle = node.fill;
    ctx.fill();
  }

  if (node.outlineColor) {
    const widthEmu = node.outlineWidthEmu;

    const widthPx =
      widthEmu == null ? 1.0 : widthEmu === 0 ? 0.5 : Math.max(0.5, widthEmu * PX_PER_EMU);
    ctx.strokeStyle = node.outlineColor;
    ctx.lineWidth = widthPx;
    const cap = ctx.lineCap;
    const join = ctx.lineJoin;
    if (isBraceLikePreset(preset)) {
      ctx.lineCap = "round";
      ctx.lineJoin = "round";
    }
    ctx.stroke();
    ctx.lineCap = cap;
    ctx.lineJoin = join;
  }

  if (flipH || flipV) {
    ctx.translate(x + w / 2, y + h / 2);
    ctx.scale(flipH ? -1 : 1, flipV ? -1 : 1);
    ctx.translate(-(x + w / 2), -(y + h / 2));
  }

  if ((node.paragraphs?.length ?? 0) > 0) {
    drawShapeText(ctx, node, x, y, w, h);
  }

  ctx.restore();
}

function drawConnector(
  ctx: CanvasRenderingContext2D,
  node: ShapeNode,
  x: number,
  y: number,
  w: number,
  h: number,
): void {
  const preset = node.preset ?? "line";
  const adj1 = (node.adj1 ?? 50000) / 100000;
  let pts: Array<[number, number]>;
  if (preset === "bentConnector3") {
    const axis =
      node.elbowAxis === "vertical"
        ? "v"
        : node.elbowAxis === "horizontal"
          ? "h"
          : w >= h
            ? "h"
            : "v";
    if (axis === "h") {
      const bx = w * adj1;
      pts = [
        [0, 0],
        [bx, 0],
        [bx, h],
        [w, h],
      ];
    } else {
      const by = h * adj1;
      pts = [
        [0, 0],
        [0, by],
        [w, by],
        [w, h],
      ];
    }
  } else {
    pts = [
      [0, 0],
      [w, h],
    ];
    if (preset === "lineInv") {
      pts = [
        [w, 0],
        [0, h],
      ];
    }
  }

  if (node.flipH) pts = pts.map(([px, py]) => [w - px, py]);
  if (node.flipV) pts = pts.map(([px, py]) => [px, h - py]);

  pts = pts.map(([px, py]) => [x + px, y + py]);

  const widthEmu = node.outlineWidthEmu;
  const widthPx =
    widthEmu == null ? 1.0 : widthEmu === 0 ? 0.75 : Math.max(0.5, widthEmu * PX_PER_EMU);
  const color = node.outlineColor ?? "#000000";
  ctx.strokeStyle = color;
  ctx.lineWidth = widthPx;
  ctx.lineCap = "butt";
  ctx.lineJoin = "miter";
  ctx.setLineDash(dashPattern(node.lineDash, widthPx));
  ctx.beginPath();
  ctx.moveTo(pts[0]![0], pts[0]![1]);
  for (let i = 1; i < pts.length; i++) {
    ctx.lineTo(pts[i]![0], pts[i]![1]);
  }
  ctx.stroke();

  ctx.setLineDash([]);

  if (node.headEnd) {
    drawArrowEnd(ctx, node.headEnd, pts[1]!, pts[0]!, color, widthPx);
  }
  if (node.tailEnd) {
    const last = pts.length - 1;
    drawArrowEnd(ctx, node.tailEnd, pts[last - 1]!, pts[last]!, color, widthPx);
  }
}

function dashPattern(token: string | undefined, w: number): number[] {
  if (!token) return [];
  switch (token) {
    case "solid":
      return [];
    case "dot":
    case "sysDot":
      return [w, w * 2];
    case "dash":
    case "sysDash":
      return [w * 4, w * 3];
    case "lgDash":
      return [w * 8, w * 3];
    case "dashDot":
    case "sysDashDot":
      return [w * 4, w * 3, w, w * 3];
    case "lgDashDot":
      return [w * 8, w * 3, w, w * 3];
    case "lgDashDotDot":
    case "sysDashDotDot":
      return [w * 8, w * 3, w, w * 3, w, w * 3];
    default:
      return [];
  }
}

function drawArrowEnd(
  ctx: CanvasRenderingContext2D,
  end: NonNullable<ShapeNode["headEnd"]>,
  from: [number, number],
  tip: [number, number],
  color: string,
  strokeW: number,
): void {
  const kind = end.kind ?? "none";
  if (kind === "none") return;

  const sizeMul = (tok: string | undefined): number => {
    switch (tok) {
      case "sm":
        return 2;
      case "lg":
        return 5;
      default:
        return 3.5;
    }
  };

  const baseStroke = Math.max(strokeW, 1);
  const lenPx = sizeMul(end.len) * baseStroke + 2;
  const widPx = sizeMul(end.w) * baseStroke + 1;
  const dx = tip[0] - from[0];
  const dy = tip[1] - from[1];
  const len = Math.hypot(dx, dy);
  if (len < 0.01) return;
  const ux = dx / len;
  const uy = dy / len;

  const px = -uy;
  const py = ux;

  const baseX = tip[0] - ux * lenPx;
  const baseY = tip[1] - uy * lenPx;
  const halfW = widPx / 2;

  ctx.save();
  ctx.fillStyle = color;
  ctx.strokeStyle = color;
  ctx.lineWidth = strokeW;
  ctx.setLineDash([]);
  switch (kind) {
    case "oval": {
      const cx = tip[0] - ux * (lenPx / 2);
      const cy = tip[1] - uy * (lenPx / 2);
      const angle = Math.atan2(uy, ux);
      ctx.beginPath();
      ctx.ellipse(cx, cy, lenPx / 2, widPx / 2, angle, 0, Math.PI * 2);
      ctx.fill();
      break;
    }
    case "diamond": {
      const midX = tip[0] - ux * (lenPx / 2);
      const midY = tip[1] - uy * (lenPx / 2);
      ctx.beginPath();
      ctx.moveTo(tip[0], tip[1]);
      ctx.lineTo(midX + px * halfW, midY + py * halfW);
      ctx.lineTo(baseX, baseY);
      ctx.lineTo(midX - px * halfW, midY - py * halfW);
      ctx.closePath();
      ctx.fill();
      break;
    }
    case "stealth": {
      const concaveX = baseX + ux * lenPx * 0.35;
      const concaveY = baseY + uy * lenPx * 0.35;
      ctx.beginPath();
      ctx.moveTo(tip[0], tip[1]);
      ctx.lineTo(baseX + px * halfW, baseY + py * halfW);
      ctx.lineTo(concaveX, concaveY);
      ctx.lineTo(baseX - px * halfW, baseY - py * halfW);
      ctx.closePath();
      ctx.fill();
      break;
    }
    case "arrow": {
      ctx.beginPath();
      ctx.moveTo(baseX + px * halfW, baseY + py * halfW);
      ctx.lineTo(tip[0], tip[1]);
      ctx.lineTo(baseX - px * halfW, baseY - py * halfW);
      ctx.stroke();
      break;
    }
    case "triangle":
    default: {
      ctx.beginPath();
      ctx.moveTo(tip[0], tip[1]);
      ctx.lineTo(baseX + px * halfW, baseY + py * halfW);
      ctx.lineTo(baseX - px * halfW, baseY - py * halfW);
      ctx.closePath();
      ctx.fill();
      break;
    }
  }
  ctx.restore();
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
  const baseX = x + textRect.x;
  const baseY = y + textRect.y;
  const baseW = textRect.w;
  const baseH = textRect.h;
  const innerX = baseX + lPad;
  const innerY = baseY + tPad;
  const innerW = Math.max(1, baseW - lPad - rPad);
  const innerH = Math.max(1, baseH - tPad - bPad);
  const wrap = node.textWrap !== "none";

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
    if (cursorY > innerY + innerH + 0.5) break;
    drawWrappedLine(ctx, ln, innerX, cursorY, innerW);
    cursorY += ln.lineHeight;
  }
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
    const font = runFont(r);
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
          cur!.runs.push({ r: { ...a.r, text: buf }, width: bufW, font: a.font });
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
