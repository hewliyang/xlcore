import type { ShapeNode } from "./schema/ShapeNode.js";

export function pathForPreset(
  ctx: CanvasRenderingContext2D,
  preset: string,
  x: number,
  y: number,
  w: number,
  h: number,
  node: ShapeNode,
): void {
  ctx.beginPath();
  switch (preset) {
    case "ellipse":
    case "circle": {
      ctx.ellipse(x + w / 2, y + h / 2, w / 2, h / 2, 0, 0, Math.PI * 2);
      break;
    }
    case "roundRect": {
      const adj = Math.max(0, Math.min(50000, node.adj1 ?? 16667)) / 100000;
      const r = Math.min(w, h) * adj;
      roundRectPath(ctx, x, y, w, h, r);
      break;
    }
    case "leftArrow":
      arrowPath(ctx, x, y, w, h, "left", node);
      break;
    case "rightArrow":
      arrowPath(ctx, x, y, w, h, "right", node);
      break;
    case "upArrow":
      arrowPath(ctx, x, y, w, h, "up", node);
      break;
    case "downArrow":
      arrowPath(ctx, x, y, w, h, "down", node);
      break;
    case "leftRightArrow":
      leftRightArrowPath(ctx, x, y, w, h, node);
      break;
    case "triangle":
      ctx.moveTo(x + w / 2, y);
      ctx.lineTo(x + w, y + h);
      ctx.lineTo(x, y + h);
      ctx.closePath();
      break;
    case "diamond":
    case "flowChartDecision":
      ctx.moveTo(x + w / 2, y);
      ctx.lineTo(x + w, y + h / 2);
      ctx.lineTo(x + w / 2, y + h);
      ctx.lineTo(x, y + h / 2);
      ctx.closePath();
      break;
    case "chevron": {
      const inset = h * 0.5;
      ctx.moveTo(x, y);
      ctx.lineTo(x + w - inset, y);
      ctx.lineTo(x + w, y + h / 2);
      ctx.lineTo(x + w - inset, y + h);
      ctx.lineTo(x, y + h);
      ctx.lineTo(x + inset, y + h / 2);
      ctx.closePath();
      break;
    }
    case "homePlate":
    case "pentagon": {
      const pt = Math.min(w * 0.5, h * 0.5);
      ctx.moveTo(x, y);
      ctx.lineTo(x + w - pt, y);
      ctx.lineTo(x + w, y + h / 2);
      ctx.lineTo(x + w - pt, y + h);
      ctx.lineTo(x, y + h);
      ctx.closePath();
      break;
    }
    case "hexagon":
      polygonPath(ctx, x, y, w, h, 6, 0);
      break;
    case "octagon":
      polygonPath(ctx, x, y, w, h, 8, Math.PI / 8);
      break;
    case "star5":
      starPath(ctx, x, y, w, h, 5, 0.38);
      break;
    case "star4":
      starPath(ctx, x, y, w, h, 4, 0.38);
      break;
    case "star6":
      starPath(ctx, x, y, w, h, 6, 0.38);
      break;
    case "star8":
      starPath(ctx, x, y, w, h, 8, 0.38);
      break;
    case "leftBrace":
      bracePath(ctx, x, y, w, h, "left", "brace", node);
      break;
    case "rightBrace":
      bracePath(ctx, x, y, w, h, "right", "brace", node);
      break;
    case "leftBracket":
      bracePath(ctx, x, y, w, h, "left", "bracket", node);
      break;
    case "rightBracket":
      bracePath(ctx, x, y, w, h, "right", "bracket", node);
      break;
    default:
      ctx.rect(x, y, w, h);
  }
}

function bracePath(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  w: number,
  h: number,
  side: "left" | "right",
  kind: "brace" | "bracket",
  node: ShapeNode,
): void {
  const rawAdj1 = node.adj1 ?? (kind === "brace" ? 8333 : 8333);
  const rawAdj2 = node.adj2 ?? 50000;
  const halfH = h / 2;
  let r = (Math.max(0, rawAdj1) / 100000) * h;
  if (r > halfH) r = halfH;
  let m = (Math.max(0, Math.min(100000, rawAdj2)) / 100000) * h;
  if (m < r) m = r;
  if (m > h - r) m = h - r;

  const backX = side === "left" ? x + w : x;
  const midX = side === "left" ? x + w / 2 : x + w / 2;
  const tipX = side === "left" ? x : x + w;

  ctx.moveTo(backX, y);
  ctx.quadraticCurveTo(midX, y, midX, y + r);
  if (kind === "brace") {
    ctx.lineTo(midX, y + m - r);
    ctx.quadraticCurveTo(midX, y + m, tipX, y + m);
    ctx.quadraticCurveTo(midX, y + m, midX, y + m + r);
    ctx.lineTo(midX, y + h - r);
  } else {
    ctx.lineTo(midX, y + h - r);
  }
  ctx.quadraticCurveTo(midX, y + h, backX, y + h);
}

function leftRightArrowPath(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  w: number,
  h: number,
  node: ShapeNode,
): void {
  const tailFrac = clamp01((node.adj1 ?? 50000) / 100000);
  const headFrac = clamp01((node.adj2 ?? 50000) / 100000);
  const tail = h * tailFrac;
  const head = Math.min(w * 0.5 * headFrac, w * 0.5);
  const tailY1 = y + (h - tail) / 2;
  const tailY2 = tailY1 + tail;
  ctx.moveTo(x, y + h / 2);
  ctx.lineTo(x + head, y);
  ctx.lineTo(x + head, tailY1);
  ctx.lineTo(x + w - head, tailY1);
  ctx.lineTo(x + w - head, y);
  ctx.lineTo(x + w, y + h / 2);
  ctx.lineTo(x + w - head, y + h);
  ctx.lineTo(x + w - head, tailY2);
  ctx.lineTo(x + head, tailY2);
  ctx.lineTo(x + head, y + h);
  ctx.closePath();
}

function polygonPath(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  w: number,
  h: number,
  sides: number,
  rotation: number,
): void {
  const cx = x + w / 2;
  const cy = y + h / 2;
  const rx = w / 2;
  const ry = h / 2;
  for (let i = 0; i < sides; i++) {
    const a = rotation + (i * 2 * Math.PI) / sides;
    const px = cx + rx * Math.cos(a);
    const py = cy + ry * Math.sin(a);
    if (i === 0) ctx.moveTo(px, py);
    else ctx.lineTo(px, py);
  }
  ctx.closePath();
}

function starPath(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  w: number,
  h: number,
  points: number,
  innerRatio: number,
): void {
  const cx = x + w / 2;
  const cy = y + h / 2;
  const rx = w / 2;
  const ry = h / 2;
  const start = -Math.PI / 2;
  const step = Math.PI / points;
  for (let i = 0; i < points * 2; i++) {
    const r = i % 2 === 0 ? 1 : innerRatio;
    const a = start + i * step;
    const px = cx + rx * r * Math.cos(a);
    const py = cy + ry * r * Math.sin(a);
    if (i === 0) ctx.moveTo(px, py);
    else ctx.lineTo(px, py);
  }
  ctx.closePath();
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
  node: ShapeNode,
): void {
  const tailFrac = clamp01((node.adj1 ?? 50000) / 100000);
  const headFrac = clamp01((node.adj2 ?? 50000) / 100000);
  if (dir === "left" || dir === "right") {
    const head = Math.min(w * headFrac, w);
    const tail = h * tailFrac;
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
    const head = Math.min(h * headFrac, h);
    const tail = w * tailFrac;
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

function clamp01(v: number): number {
  if (!Number.isFinite(v)) return 0;
  return v < 0 ? 0 : v > 1 ? 1 : v;
}

export function isBraceLikePreset(preset: string | undefined): boolean {
  return (
    preset === "leftBrace" ||
    preset === "rightBrace" ||
    preset === "leftBracket" ||
    preset === "rightBracket"
  );
}

export function isLinePreset(preset: string | undefined): boolean {
  if (!preset) return false;
  return (
    preset === "line" ||
    preset === "lineInv" ||
    preset.startsWith("straightConnector") ||
    preset.startsWith("bentConnector") ||
    preset.startsWith("curvedConnector")
  );
}
