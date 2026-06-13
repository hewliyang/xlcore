import type { ChartSeries } from "./schema/ChartSeries.js";
import type { ChartTrendline } from "./schema/ChartTrendline.js";

function dashFor(dash: string | undefined): number[] {
  switch (dash) {
    case "dot":
      return [1, 3];
    case "dash":
      return [4, 3];
    case "lgDash":
      return [8, 3];
    case "dashDot":
      return [4, 3, 1, 3];
    case "lgDashDot":
      return [8, 3, 1, 3];
    case "sysDash":
      return [3, 1];
    case "sysDot":
      return [1, 1];
    default:
      return [6, 4];
  }
}

function linearFit(xs: number[], ys: number[]): ((x: number) => number) | null {
  const n = xs.length;
  if (n < 2) return null;
  let sx = 0,
    sy = 0,
    sxx = 0,
    sxy = 0;
  for (let i = 0; i < n; i++) {
    const x = xs[i]!,
      y = ys[i]!;
    sx += x;
    sy += y;
    sxx += x * x;
    sxy += x * y;
  }
  const den = n * sxx - sx * sx;
  if (den === 0) return null;
  const b = (n * sxy - sx * sy) / den;
  const a = (sy - b * sx) / n;
  return (x) => a + b * x;
}

function polyFit(xs: number[], ys: number[], order: number): ((x: number) => number) | null {
  const deg = Math.max(2, Math.min(6, order));
  const n = xs.length;
  if (n < deg + 1) return linearFit(xs, ys);
  const m = deg + 1;
  const A: number[][] = Array.from({ length: m }, () => new Array(m + 1).fill(0));
  const pow: number[] = new Array(2 * deg + 1).fill(0);
  for (let i = 0; i < n; i++) {
    let xp = 1;
    for (let p = 0; p <= 2 * deg; p++) {
      pow[p]! += xp;
      xp *= xs[i]!;
    }
  }
  for (let r = 0; r < m; r++) {
    for (let c = 0; c < m; c++) A[r]![c] = pow[r + c]!;
    let s = 0;
    for (let i = 0; i < n; i++) s += ys[i]! * xs[i]! ** r;
    A[r]![m] = s;
  }
  for (let col = 0; col < m; col++) {
    let piv = col;
    for (let r = col + 1; r < m; r++) if (Math.abs(A[r]![col]!) > Math.abs(A[piv]![col]!)) piv = r;
    if (Math.abs(A[piv]![col]!) < 1e-12) return linearFit(xs, ys);
    [A[col], A[piv]] = [A[piv]!, A[col]!];
    for (let r = 0; r < m; r++) {
      if (r === col) continue;
      const f = A[r]![col]! / A[col]![col]!;
      for (let c = col; c <= m; c++) A[r]![c]! -= f * A[col]![c]!;
    }
  }
  const coef = A.map((row, i) => row[m]! / row[i]!);
  return (x) => {
    let y = 0,
      xp = 1;
    for (let i = 0; i < m; i++) {
      y += coef[i]! * xp;
      xp *= x;
    }
    return y;
  };
}

function expFit(xs: number[], ys: number[]): ((x: number) => number) | null {
  const lx: number[] = [],
    ly: number[] = [];
  for (let i = 0; i < xs.length; i++) {
    if (ys[i]! > 0) {
      lx.push(xs[i]!);
      ly.push(Math.log(ys[i]!));
    }
  }
  const fit = linearFit(lx, ly);
  return fit ? (x) => Math.exp(fit(x)) : null;
}

function logFit(xs: number[], ys: number[]): ((x: number) => number) | null {
  const lx: number[] = [],
    ly: number[] = [];
  for (let i = 0; i < xs.length; i++) {
    if (xs[i]! > 0) {
      lx.push(Math.log(xs[i]!));
      ly.push(ys[i]!);
    }
  }
  const fit = linearFit(lx, ly);
  return fit ? (x) => (x > 0 ? fit(Math.log(x)) : Number.NaN) : null;
}

function powerFit(xs: number[], ys: number[]): ((x: number) => number) | null {
  const lx: number[] = [],
    ly: number[] = [];
  for (let i = 0; i < xs.length; i++) {
    if (xs[i]! > 0 && ys[i]! > 0) {
      lx.push(Math.log(xs[i]!));
      ly.push(Math.log(ys[i]!));
    }
  }
  const fit = linearFit(lx, ly);
  return fit ? (x) => (x > 0 ? Math.exp(fit(Math.log(x))) : Number.NaN) : null;
}

export function drawTrendlines(
  ctx: CanvasRenderingContext2D,
  series: ChartSeries,
  xs: number[],
  ys: number[],
  xPix: (x: number) => number,
  yPix: (y: number) => number,
): void {
  const tls = series.trendlines ?? [];
  if (tls.length === 0 || xs.length < 2) return;
  let xMin = xs[0]!,
    xMax = xs[0]!;
  for (const x of xs) {
    if (x < xMin) xMin = x;
    if (x > xMax) xMax = x;
  }
  for (const tl of tls) {
    ctx.save();
    ctx.strokeStyle = tl.color ?? series.color ?? "#595959";
    ctx.lineWidth = tl.lineWidthEmu != null ? Math.max(0.75, tl.lineWidthEmu / 12700) : 1.25;
    ctx.setLineDash(tl.lineDash != null ? dashFor(tl.lineDash) : dashFor("dash"));
    if (tl.type === "movingavg") {
      drawMovingAvg(ctx, tl, xs, ys, xPix, yPix);
    } else {
      drawFit(ctx, tl, xs, ys, xMin, xMax, xPix, yPix);
    }
    ctx.restore();
  }
}

function evaluatorFor(
  tl: ChartTrendline,
  xs: number[],
  ys: number[],
): ((x: number) => number) | null {
  switch (tl.type) {
    case "linear":
      return linearFit(xs, ys);
    case "poly":
      return polyFit(xs, ys, tl.polynomialOrder ?? 2);
    case "exp":
      return expFit(xs, ys);
    case "log":
      return logFit(xs, ys);
    case "power":
      return powerFit(xs, ys);
    default:
      return linearFit(xs, ys);
  }
}

function drawFit(
  ctx: CanvasRenderingContext2D,
  tl: ChartTrendline,
  xs: number[],
  ys: number[],
  xMin: number,
  xMax: number,
  xPix: (x: number) => number,
  yPix: (y: number) => number,
): void {
  const fit = evaluatorFor(tl, xs, ys);
  if (!fit) return;
  const span = xMax - xMin || 1;
  const from = xMin - (tl.backward ?? 0) * (span / Math.max(1, xs.length - 1));
  const to = xMax + (tl.forward ?? 0) * (span / Math.max(1, xs.length - 1));
  const STEPS = 80;
  ctx.beginPath();
  let pen = false;
  for (let i = 0; i <= STEPS; i++) {
    const x = from + ((to - from) * i) / STEPS;
    const y = fit(x);
    if (!Number.isFinite(y)) {
      pen = false;
      continue;
    }
    const px = xPix(x),
      py = yPix(y);
    if (!pen) {
      ctx.moveTo(px, py);
      pen = true;
    } else ctx.lineTo(px, py);
  }
  ctx.stroke();
}

function drawMovingAvg(
  ctx: CanvasRenderingContext2D,
  tl: ChartTrendline,
  xs: number[],
  ys: number[],
  xPix: (x: number) => number,
  yPix: (y: number) => number,
): void {
  const period = Math.max(2, tl.period ?? 2);
  ctx.beginPath();
  let pen = false;
  for (let i = period - 1; i < xs.length; i++) {
    let sum = 0;
    for (let j = i - period + 1; j <= i; j++) sum += ys[j]!;
    const px = xPix(xs[i]!),
      py = yPix(sum / period);
    if (!pen) {
      ctx.moveTo(px, py);
      pen = true;
    } else ctx.lineTo(px, py);
  }
  ctx.stroke();
}
