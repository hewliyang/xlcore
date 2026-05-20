import type { Shape, ShapeBlipFill, ShapeGradient, ShapeNode } from "./types.js";
import { getOrLoadImage } from "./imageCache.js";
import { isBraceLikePreset, isLinePreset, pathForPreset } from "./shapePaths.js";
import { drawShapeText } from "./shapeText.js";

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

  const shadow = node.outerShadow;
  const hasShadowedFill = !!shadow && (!!node.fill || !!node.fillGradient);
  if (hasShadowedFill && shadow) {
    applyShadow(ctx, shadow);
  }

  if (node.fillBlip) {
    if (shadow) {
      applyShadow(ctx, shadow);
      ctx.fillStyle = "rgba(0,0,0,0)";
      ctx.fill();
      clearShadow(ctx);
    }
    ctx.save();
    ctx.clip();
    drawBlipFillImage(ctx, node.fillBlip, x, y, w, h);
    ctx.restore();
    pathForPreset(ctx, preset, x, y, w, h, node);
  } else if (node.fillGradient && node.fillGradient.stops.length >= 2) {
    ctx.fillStyle = gradientFillStyle(ctx, node.fillGradient, x, y, w, h);
    ctx.fill();
  } else if (node.fill) {
    ctx.fillStyle = node.fill;
    ctx.fill();
  } else if (shadow) {
    applyShadow(ctx, shadow);
    ctx.fillStyle = "rgba(0,0,0,0)";
    ctx.fill();
  }

  if (shadow) {
    clearShadow(ctx);
  }

  if (node.outlineColor) {
    const widthEmu = node.outlineWidthEmu;

    const widthPx =
      widthEmu == null ? 1.0 : widthEmu === 0 ? 0.5 : Math.max(0.5, widthEmu * PX_PER_EMU);
    ctx.strokeStyle = node.outlineColor;
    ctx.lineWidth = widthPx;
    const savedCap = ctx.lineCap;
    const savedJoin = ctx.lineJoin;
    const explicitCap = mapLineCap(node.lineCap);
    const explicitJoin = mapLineJoin(node.lineJoin);
    if (explicitCap) {
      ctx.lineCap = explicitCap;
    } else if (isBraceLikePreset(preset)) {
      ctx.lineCap = "round";
    }
    if (explicitJoin) {
      ctx.lineJoin = explicitJoin;
    } else if (isBraceLikePreset(preset)) {
      ctx.lineJoin = "round";
    }
    ctx.setLineDash(dashPattern(node.lineDash, widthPx));
    ctx.stroke();
    ctx.setLineDash([]);
    ctx.lineCap = savedCap;
    ctx.lineJoin = savedJoin;
  }

  if (flipH || flipV) {
    ctx.translate(x + w / 2, y + h / 2);
    ctx.scale(flipH ? -1 : 1, flipV ? -1 : 1);
    ctx.translate(-(x + w / 2), -(y + h / 2));
  }

  if ((node.paragraphs?.length ?? 0) > 0) {
    drawShapeText(ctx, node, x, y, w, h, flipH, flipV);
  }

  ctx.restore();
}

function applyShadow(
  ctx: CanvasRenderingContext2D,
  shadow: {
    color: string;
    alpha: number;
    blurEmu: number;
    distEmu: number;
    dirDeg: number;
  },
): void {
  const blurPx = (shadow.blurEmu ?? 0) * PX_PER_EMU;
  const distPx = (shadow.distEmu ?? 0) * PX_PER_EMU;
  const rad = ((shadow.dirDeg ?? 0) * Math.PI) / 180;
  const dx = Math.cos(rad) * distPx;
  const dy = Math.sin(rad) * distPx;
  ctx.shadowBlur = blurPx;
  ctx.shadowOffsetX = dx;
  ctx.shadowOffsetY = dy;
  ctx.shadowColor = colorWithAlpha(shadow.color, shadow.alpha ?? 1);
}

function clearShadow(ctx: CanvasRenderingContext2D): void {
  ctx.shadowBlur = 0;
  ctx.shadowOffsetX = 0;
  ctx.shadowOffsetY = 0;
  ctx.shadowColor = "rgba(0,0,0,0)";
}

function colorWithAlpha(hex: string, alpha: number): string {
  const a = Math.max(0, Math.min(1, alpha));
  const m = /^#?([0-9a-fA-F]{6})$/.exec(hex);
  if (!m) return hex;
  const n = parseInt(m[1] ?? "0", 16);
  const r = (n >> 16) & 0xff;
  const g = (n >> 8) & 0xff;
  const b = n & 0xff;
  return `rgba(${r},${g},${b},${a})`;
}

function gradientFillStyle(
  ctx: CanvasRenderingContext2D,
  g: ShapeGradient,
  x: number,
  y: number,
  w: number,
  h: number,
): CanvasGradient | string {
  const stops = g.stops;
  const first = stops[0]!.color;
  if (g.kind === "path") {
    const r = g.fillToRect ?? [0, 0, 0, 0];
    const li = r[0] ?? 0;
    const ti = r[1] ?? 0;
    const ri = r[2] ?? 0;
    const bi = r[3] ?? 0;
    const ix = x + li * w;
    const iy = y + ti * h;
    const iw = Math.max(0, w * Math.max(0, 1 - li - ri));
    const ih = Math.max(0, h * Math.max(0, 1 - ti - bi));
    const cx = ix + iw / 2;
    const cy = iy + ih / 2;
    const r0 = Math.hypot(iw, ih) / 2;
    const corners: Array<[number, number]> = [
      [x, y],
      [x + w, y],
      [x, y + h],
      [x + w, y + h],
    ];
    const r1 = Math.max(...corners.map(([px, py]) => Math.hypot(px - cx, py - cy)));
    if (r1 <= r0 + 0.5) return first;
    const grad = ctx.createRadialGradient(cx, cy, r0, cx, cy, r1);
    for (const s of stops) {
      grad.addColorStop(Math.max(0, Math.min(1, s.pos)), s.color);
    }
    return grad;
  }
  const deg = g.angleDeg ?? 0;
  const theta = (deg * Math.PI) / 180;
  const dx = Math.cos(theta);
  const dy = Math.sin(theta);
  const projs = [0, w * dx, h * dy, w * dx + h * dy];
  const pmin = Math.min(...projs);
  const pmax = Math.max(...projs);
  const x0 = x + pmin * dx;
  const y0 = y + pmin * dy;
  const x1 = x + pmax * dx;
  const y1 = y + pmax * dy;
  if (Math.hypot(x1 - x0, y1 - y0) < 0.5) return first;
  const grad = ctx.createLinearGradient(x0, y0, x1, y1);
  for (const s of stops) {
    grad.addColorStop(Math.max(0, Math.min(1, s.pos)), s.color);
  }
  return grad;
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
  const adj2 = (node.adj2 ?? 50000) / 100000;
  const adj3 = (node.adj3 ?? 50000) / 100000;
  let pts: Array<[number, number]>;
  if (preset === "bentConnector2") {
    pts = [
      [0, 0],
      [w, 0],
      [w, h],
    ];
  } else if (preset === "bentConnector3") {
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
  } else if (preset === "bentConnector4") {
    const x1 = w * adj1;
    const y2 = h * adj2;
    pts = [
      [0, 0],
      [x1, 0],
      [x1, y2],
      [w, y2],
      [w, h],
    ];
  } else if (preset === "bentConnector5") {
    const x1 = w * adj1;
    const y2 = h * adj2;
    const x3 = w * adj3;
    pts = [
      [0, 0],
      [x1, 0],
      [x1, y2],
      [x3, y2],
      [x3, h],
      [w, h],
    ];
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
  ctx.lineCap = mapLineCap(node.lineCap) ?? "butt";
  ctx.lineJoin = mapLineJoin(node.lineJoin) ?? "miter";
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

function mapLineCap(token: string | undefined | null): CanvasLineCap | undefined {
  switch (token) {
    case "round":
    case "rnd":
      return "round";
    case "square":
    case "sq":
      return "square";
    case "flat":
      return "butt";
    default:
      return undefined;
  }
}

function mapLineJoin(token: string | undefined | null): CanvasLineJoin | undefined {
  switch (token) {
    case "round":
      return "round";
    case "bevel":
      return "bevel";
    case "miter":
      return "miter";
    default:
      return undefined;
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

function drawBlipFillImage(
  ctx: CanvasRenderingContext2D,
  blip: ShapeBlipFill,
  x: number,
  y: number,
  w: number,
  h: number,
): void {
  const uri = blip.dataUri;
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
  const crop = blip.srcRect;
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
