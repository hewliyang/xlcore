import type { Sheet } from "./types.js";
import type { Grid } from "./grid.js";
import { buildMergeMaps, cellRect, mergedRect } from "./geometry.js";
import type { Visible } from "./renderTypes.js";

export function drawCfIcons(
  ctx: CanvasRenderingContext2D,
  sheet: Sheet,
  g: Grid,
  vis: Visible,
  cfIconDraw: Map<string, { iconSet: string; idx: number; n: number }>,
): void {
  if (cfIconDraw.size === 0) return;
  const { covered, topLeftOf } = buildMergeMaps(sheet);
  const ICON_PX = 12;
  const INSET_X = 3;
  for (const [k, info] of cfIconDraw) {
    if (covered.has(k)) continue;
    const [rs, cs] = k.split(":");
    const r = parseInt(rs!, 10),
      c = parseInt(cs!, 10);
    if (r < vis.firstRow || r > vis.lastRow) continue;
    if (c < vis.firstCol || c > vis.lastCol) continue;
    const rect = topLeftOf.has(k) ? mergedRect(g, topLeftOf.get(k)!) : cellRect(g, r, c);
    const x = rect.x + INSET_X;
    const y = rect.y + (rect.h - ICON_PX) / 2;
    drawIconGlyph(ctx, info.iconSet, info.idx, info.n, x, y, ICON_PX);
  }
}

function drawIconGlyph(
  ctx: CanvasRenderingContext2D,
  iconSet: string,
  idx: number,
  n: number,
  x: number,
  y: number,
  s: number,
): void {
  const redYelGreen = (i: number, total: number): string => {
    if (total <= 1) return "#63BE7B";
    const stops = ["#F8696B", "#FCB14E", "#FFEB84", "#B1D580", "#63BE7B"];
    if (total === 3) return [stops[0]!, stops[2]!, stops[4]!][i] ?? "#888";
    if (total === 4) return [stops[0]!, stops[1]!, stops[3]!, stops[4]!][i] ?? "#888";
    return stops[i] ?? "#888";
  };
  const grayScale = (i: number, total: number): string => {
    const t = total <= 1 ? 0 : i / (total - 1);
    const v = Math.round(60 + t * (210 - 60));
    return `rgb(${v},${v},${v})`;
  };
  const setLower = iconSet.toLowerCase();
  const isGray = setLower.includes("gray");
  const colorAt = (i: number) => (isGray ? grayScale(i, n) : redYelGreen(i, n));

  ctx.save();
  ctx.translate(x, y);

  const cx = s / 2,
    cy = s / 2;

  if (setLower.includes("arrow")) {
    const angleFor = (i: number, total: number): number => {
      if (total === 3) return [180, 90, 0][i] ?? 90;
      if (total === 4) return [180, 135, 45, 0][i] ?? 90;
      return [180, 135, 90, 45, 0][i] ?? 90;
    };
    const ang = (angleFor(idx, n) * Math.PI) / 180;
    ctx.translate(cx, cy);
    ctx.rotate(ang);
    ctx.fillStyle = colorAt(idx);

    const h = s * 0.45,
      w = s * 0.35,
      stem = s * 0.18;
    ctx.beginPath();
    ctx.moveTo(0, -h);
    ctx.lineTo(w, -h * 0.05);
    ctx.lineTo(stem / 2, -h * 0.05);
    ctx.lineTo(stem / 2, h);
    ctx.lineTo(-stem / 2, h);
    ctx.lineTo(-stem / 2, -h * 0.05);
    ctx.lineTo(-w, -h * 0.05);
    ctx.closePath();
    ctx.fill();
    ctx.restore();
    return;
  }

  if (
    setLower.includes("trafficlight") ||
    setLower.includes("signs") ||
    setLower.includes("flag")
  ) {
    ctx.fillStyle = colorAt(idx);
    if (setLower.includes("flag")) {
      ctx.beginPath();
      ctx.moveTo(s * 0.2, s * 0.1);
      ctx.lineTo(s * 0.85, s * 0.4);
      ctx.lineTo(s * 0.2, s * 0.7);
      ctx.closePath();
      ctx.fill();

      ctx.fillRect(s * 0.15, s * 0.1, s * 0.08, s * 0.8);
    } else if (setLower.includes("signs")) {
      ctx.beginPath();
      ctx.moveTo(cx, s * 0.1);
      ctx.lineTo(s * 0.9, cy);
      ctx.lineTo(cx, s * 0.9);
      ctx.lineTo(s * 0.1, cy);
      ctx.closePath();
      ctx.fill();
    } else {
      ctx.beginPath();
      ctx.arc(cx, cy, s * 0.42, 0, Math.PI * 2);
      ctx.fill();
      if (setLower.includes("trafficlights2") || setLower.includes("rimmed")) {
        ctx.lineWidth = 1;
        ctx.strokeStyle = "#222";
        ctx.stroke();
      }
    }
    ctx.restore();
    return;
  }

  if (setLower.includes("symbol")) {
    const circled = setLower === "3symbols";
    ctx.fillStyle = colorAt(idx);
    if (circled) {
      ctx.beginPath();
      ctx.arc(cx, cy, s * 0.45, 0, Math.PI * 2);
      ctx.fill();
    }
    ctx.strokeStyle = circled ? "#fff" : colorAt(idx);
    ctx.lineWidth = Math.max(1.2, s * 0.13);
    ctx.lineCap = "round";
    ctx.beginPath();
    if (idx === 2) {
      ctx.moveTo(s * 0.27, cy);
      ctx.lineTo(s * 0.45, s * 0.65);
      ctx.lineTo(s * 0.75, s * 0.32);
    } else if (idx === 1) {
      ctx.moveTo(cx, s * 0.25);
      ctx.lineTo(cx, s * 0.58);
      ctx.stroke();
      ctx.beginPath();
      ctx.arc(cx, s * 0.72, Math.max(1, s * 0.06), 0, Math.PI * 2);
      ctx.fillStyle = circled ? "#fff" : colorAt(idx);
      ctx.fill();
      ctx.restore();
      return;
    } else {
      ctx.moveTo(s * 0.3, s * 0.3);
      ctx.lineTo(s * 0.7, s * 0.7);
      ctx.moveTo(s * 0.7, s * 0.3);
      ctx.lineTo(s * 0.3, s * 0.7);
    }
    ctx.stroke();
    ctx.restore();
    return;
  }

  if (setLower.includes("rating") || setLower.includes("redtoblack")) {
    const filled = idx + 1;
    const gap = s * 0.08;
    const totalW = s * 0.9;
    const bw = (totalW - gap * (n - 1)) / n;
    const bx0 = (s - totalW) / 2;
    for (let i = 0; i < n; i++) {
      const bx = bx0 + i * (bw + gap);
      const filledHere = i < filled;
      ctx.fillStyle = filledHere ? "#444" : "#cccccc";
      const bh = s * 0.55 * (0.4 + (0.6 * (i + 1)) / n);
      ctx.fillRect(bx, s * 0.85 - bh, bw, bh);
    }
    ctx.restore();
    return;
  }

  if (setLower.includes("quarter")) {
    ctx.strokeStyle = "#333";
    ctx.fillStyle = "#333";
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.arc(cx, cy, s * 0.42, 0, Math.PI * 2);
    ctx.stroke();
    if (idx === 0) {
      ctx.restore();
      return;
    }

    const wedges: [number, number][] = [
      [-Math.PI / 2, 0],
      [0, Math.PI / 2],
      [Math.PI / 2, Math.PI],
      [Math.PI, (Math.PI * 3) / 2],
    ];
    const fill = Math.min(idx, 4);
    for (let i = 0; i < fill; i++) {
      ctx.beginPath();
      ctx.moveTo(cx, cy);
      ctx.arc(cx, cy, s * 0.42, wedges[i]![0], wedges[i]![1]);
      ctx.closePath();
      ctx.fill();
    }
    ctx.restore();
    return;
  }

  if (setLower.includes("box")) {
    const filled = idx + 1;
    const gap = s * 0.08;
    const totalW = s * 0.9;
    const bw = (totalW - gap * (n - 1)) / n;
    const bx0 = (s - totalW) / 2;
    const by = (s - bw) / 2;
    ctx.strokeStyle = "#444";
    for (let i = 0; i < n; i++) {
      const bx = bx0 + i * (bw + gap);
      ctx.fillStyle = i < filled ? "#5b8def" : "#dddddd";
      ctx.fillRect(bx, by, bw, bw);
      ctx.strokeRect(bx + 0.5, by + 0.5, bw - 1, bw - 1);
    }
    ctx.restore();
    return;
  }

  if (setLower.includes("triangle")) {
    ctx.fillStyle = colorAt(idx);
    ctx.beginPath();
    if (idx === 0) {
      ctx.moveTo(s * 0.15, s * 0.25);
      ctx.lineTo(s * 0.85, s * 0.25);
      ctx.lineTo(cx, s * 0.8);
    } else if (idx === 1) {
      ctx.fillRect(s * 0.18, cy - s * 0.07, s * 0.64, s * 0.14);
      ctx.restore();
      return;
    } else {
      ctx.moveTo(s * 0.15, s * 0.75);
      ctx.lineTo(s * 0.85, s * 0.75);
      ctx.lineTo(cx, s * 0.2);
    }
    ctx.closePath();
    ctx.fill();
    ctx.restore();
    return;
  }

  if (setLower.includes("star")) {
    const fill = idx / Math.max(1, n - 1);
    drawStarFill(ctx, cx, cy, s * 0.42, fill);
    ctx.restore();
    return;
  }

  ctx.fillStyle = colorAt(idx);
  ctx.beginPath();
  ctx.arc(cx, cy, s * 0.4, 0, Math.PI * 2);
  ctx.fill();
  ctx.restore();
}

function drawStarFill(
  ctx: CanvasRenderingContext2D,
  cx: number,
  cy: number,
  r: number,
  fillFrac: number,
): void {
  const pts: [number, number][] = [];
  for (let i = 0; i < 10; i++) {
    const ang = -Math.PI / 2 + (i * Math.PI) / 5;
    const rr = i % 2 === 0 ? r : r * 0.45;
    pts.push([cx + rr * Math.cos(ang), cy + rr * Math.sin(ang)]);
  }

  ctx.beginPath();
  pts.forEach(([x, y], i) => {
    if (i === 0) ctx.moveTo(x, y);
    else ctx.lineTo(x, y);
  });
  ctx.closePath();
  ctx.strokeStyle = "#aaa";
  ctx.lineWidth = 1;
  ctx.stroke();
  if (fillFrac <= 0) return;
  if (fillFrac >= 1) {
    ctx.fillStyle = "#f5b400";
    ctx.fill();
    return;
  }
  ctx.save();
  ctx.beginPath();
  pts.forEach(([x, y], i) => {
    if (i === 0) ctx.moveTo(x, y);
    else ctx.lineTo(x, y);
  });
  ctx.closePath();
  ctx.clip();
  ctx.fillStyle = "#f5b400";
  ctx.fillRect(cx - r, cy - r, 2 * r * fillFrac, 2 * r);
  ctx.restore();
}
