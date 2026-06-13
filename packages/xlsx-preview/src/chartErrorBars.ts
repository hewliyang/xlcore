import type { ChartSeries } from "./schema/ChartSeries.js";
import type { ChartErrorBars } from "./schema/ChartErrorBars.js";

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
      return [];
  }
}

function standardDeviation(ys: number[]): number {
  const n = ys.length;
  if (n < 2) return 0;
  const mean = ys.reduce((a, b) => a + b, 0) / n;
  const variance = ys.reduce((a, y) => a + (y - mean) ** 2, 0) / (n - 1);
  return Math.sqrt(variance);
}

function magnitudes(
  eb: ChartErrorBars,
  ys: number[],
): { plus: (i: number) => number; minus: (i: number) => number } {
  const val = eb.value ?? 0;
  switch (eb.errValType) {
    case "fixedval": {
      const m = (_: number) => val;
      return { plus: m, minus: m };
    }
    case "percentage": {
      const m = (i: number) => (Math.abs(ys[i] ?? 0) * val) / 100;
      return { plus: m, minus: m };
    }
    case "stddev": {
      const sd = standardDeviation(ys) * val;
      const m = (_: number) => sd;
      return { plus: m, minus: m };
    }
    case "stderr": {
      const se = standardDeviation(ys) / Math.sqrt(Math.max(1, ys.length));
      const m = (_: number) => se;
      return { plus: m, minus: m };
    }
    case "cust": {
      const plus = eb.plusValues ?? [];
      const minus = eb.minusValues ?? [];
      return {
        plus: (i) => plus[i] ?? plus[plus.length - 1] ?? 0,
        minus: (i) => minus[i] ?? minus[minus.length - 1] ?? 0,
      };
    }
    default: {
      const m = (_: number) => val;
      return { plus: m, minus: m };
    }
  }
}

export function drawErrorBars(
  ctx: CanvasRenderingContext2D,
  series: ChartSeries,
  xs: number[],
  ys: number[],
  xPix: (x: number) => number,
  yPix: (y: number) => number,
): void {
  const eb = series.errorBars;
  if (!eb) return;
  if (eb.errDir && eb.errDir !== "y") return;
  const n = Math.min(xs.length, ys.length);
  if (n === 0) return;

  const { plus, minus } = magnitudes(eb, ys);
  const drawPlus = eb.errBarType === "both" || eb.errBarType === "plus";
  const drawMinus = eb.errBarType === "both" || eb.errBarType === "minus";
  const cap = eb.noEndCap ? 0 : 4;

  ctx.save();
  ctx.strokeStyle = eb.color ?? series.color ?? "#404040";
  ctx.lineWidth = eb.lineWidthEmu != null ? Math.max(0.75, eb.lineWidthEmu / 12700) : 1;
  ctx.setLineDash(eb.lineDash != null ? dashFor(eb.lineDash) : []);

  for (let i = 0; i < n; i++) {
    const y = ys[i] ?? 0;
    const px = xPix(xs[i] ?? 0);
    const cy = yPix(y);
    if (drawPlus) {
      const top = yPix(y + plus(i));
      ctx.beginPath();
      ctx.moveTo(px, cy);
      ctx.lineTo(px, top);
      ctx.stroke();
      if (cap > 0) {
        ctx.beginPath();
        ctx.moveTo(px - cap, top);
        ctx.lineTo(px + cap, top);
        ctx.stroke();
      }
    }
    if (drawMinus) {
      const bot = yPix(y - minus(i));
      ctx.beginPath();
      ctx.moveTo(px, cy);
      ctx.lineTo(px, bot);
      ctx.stroke();
      if (cap > 0) {
        ctx.beginPath();
        ctx.moveTo(px - cap, bot);
        ctx.lineTo(px + cap, bot);
        ctx.stroke();
      }
    }
  }
  ctx.restore();
}
