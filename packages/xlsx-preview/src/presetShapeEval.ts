import { PRESET_SHAPES } from "./presetShapeData.generated.js";
import type { Formula, PresetShape, Token } from "./presetShapeData.types.js";

const DEG = 60000;
const ANG_90 = 90 * DEG;
const ANG_180 = 180 * DEG;
const ANG_270 = 270 * DEG;
const ANG_45 = 45 * DEG;
const ANG_135 = 135 * DEG;
const ANG_225 = 225 * DEG;
const ANG_315 = 315 * DEG;

function radFromAngleUnit(a: number): number {
  return (a / DEG) * (Math.PI / 180);
}

interface Env {
  symbols: Map<string, number>;
}

function makeEnv(preset: PresetShape, w: number, h: number, adjs?: number[]): Env {
  const symbols = new Map<string, number>();

  symbols.set("w", w);
  symbols.set("h", h);
  symbols.set("l", 0);
  symbols.set("t", 0);
  symbols.set("r", w);
  symbols.set("b", h);
  symbols.set("hc", w / 2);
  symbols.set("vc", h / 2);
  const ss = Math.min(w, h);
  const ls = Math.max(w, h);
  symbols.set("ss", ss);
  symbols.set("ls", ls);
  for (const n of [2, 3, 4, 5, 6, 8, 10, 12, 16, 32]) {
    symbols.set(`wd${n}`, w / n);
    symbols.set(`hd${n}`, h / n);
    symbols.set(`ssd${n}`, ss / n);
    symbols.set(`lsd${n}`, ls / n);
  }

  symbols.set("cd2", ANG_180);
  symbols.set("cd4", ANG_90);
  symbols.set("cd8", ANG_45);
  symbols.set("3cd4", ANG_270);
  symbols.set("3cd8", ANG_135);
  symbols.set("5cd8", ANG_225);
  symbols.set("7cd8", ANG_315);

  for (const gd of preset.av) {
    symbols.set(gd.name, evalFormula(gd.fmla, symbols));
  }
  if (adjs && adjs.length) {
    let i = 0;
    for (const gd of preset.av) {
      if (i >= adjs.length) break;
      if (Number.isFinite(adjs[i])) symbols.set(gd.name, adjs[i] as number);
      i++;
    }
  }

  for (const gd of preset.gd) {
    symbols.set(gd.name, evalFormula(gd.fmla, symbols));
  }

  return { symbols };
}

function resolveToken(tok: Token, symbols: Map<string, number>): number {
  if (/^-?\d+$/.test(tok)) return Number(tok);
  const v = symbols.get(tok);
  if (v !== undefined) return v;
  return 0;
}

function evalFormula(fmla: Formula, symbols: Map<string, number>): number {
  const a = (i: number) => resolveToken(fmla.args[i] as Token, symbols);
  switch (fmla.op) {
    case "val":
      return a(0);
    case "*/": {
      const va = a(0), vb = a(1), vc = a(2);
      return vc === 0 ? 0 : (va * vb) / vc;
    }
    case "+-":
      return a(0) + a(1) - a(2);
    case "+/": {
      const vc = a(2);
      return vc === 0 ? 0 : (a(0) + a(1)) / vc;
    }
    case "?:":
      return a(0) > 0 ? a(1) : a(2);
    case "abs":
      return Math.abs(a(0));
    case "sqrt": {
      const v = a(0);
      return v < 0 ? 0 : Math.sqrt(v);
    }
    case "max":
      return Math.max(a(0), a(1));
    case "min":
      return Math.min(a(0), a(1));
    case "mod": {
      const x = a(0), y = a(1), z = a(2);
      return Math.sqrt(x * x + y * y + z * z);
    }
    case "pin": {
      const lo = a(0), v = a(1), hi = a(2);
      return v < lo ? lo : v > hi ? hi : v;
    }
    case "sin":
      return a(0) * Math.sin(radFromAngleUnit(a(1)));
    case "cos":
      return a(0) * Math.cos(radFromAngleUnit(a(1)));
    case "tan":
      return a(0) * Math.tan(radFromAngleUnit(a(1)));
    case "at2": {
      const x = a(0), y = a(1);
      return (Math.atan2(y, x) * 180) / Math.PI * DEG;
    }
    case "cat2":
      return a(0) * Math.cos(Math.atan2(a(2), a(1)));
    case "sat2":
      return a(0) * Math.sin(Math.atan2(a(2), a(1)));
    default:
      return 0;
  }
}

export function tracePresetIntoPath(
  ctx: CanvasRenderingContext2D,
  presetName: string,
  x: number,
  y: number,
  w: number,
  h: number,
  adjs?: number[],
): boolean {
  const preset = (PRESET_SHAPES as Record<string, PresetShape | undefined>)[presetName];
  if (!preset) return false;
  if (preset.paths.length === 0) return false;
  const env = makeEnv(preset, w, h, adjs);

  for (const path of preset.paths) {
    const sx = path.w ? w / path.w : 1;
    const sy = path.h ? h / path.h : 1;
    const px = (tok: Token) => x + resolveToken(tok, env.symbols) * sx;
    const py = (tok: Token) => y + resolveToken(tok, env.symbols) * sy;
    let penX = x;
    let penY = y;
    for (const c of path.cmds) {
      switch (c.op) {
        case "M": {
          penX = px(c.x);
          penY = py(c.y);
          ctx.moveTo(penX, penY);
          break;
        }
        case "L": {
          penX = px(c.x);
          penY = py(c.y);
          ctx.lineTo(penX, penY);
          break;
        }
        case "Q": {
          const cpx = px(c.x1), cpy = py(c.y1);
          penX = px(c.x);
          penY = py(c.y);
          ctx.quadraticCurveTo(cpx, cpy, penX, penY);
          break;
        }
        case "C": {
          const c1x = px(c.x1), c1y = py(c.y1);
          const c2x = px(c.x2), c2y = py(c.y2);
          penX = px(c.x);
          penY = py(c.y);
          ctx.bezierCurveTo(c1x, c1y, c2x, c2y, penX, penY);
          break;
        }
        case "A": {
          const wR = resolveToken(c.wR, env.symbols) * sx;
          const hR = resolveToken(c.hR, env.symbols) * sy;
          const stAng = radFromAngleUnit(resolveToken(c.stAng, env.symbols));
          const swAng = radFromAngleUnit(resolveToken(c.swAng, env.symbols));
          const cx = penX - wR * Math.cos(stAng);
          const cy = penY - hR * Math.sin(stAng);
          const endAng = stAng + swAng;
          ctx.ellipse(cx, cy, Math.abs(wR), Math.abs(hR), 0, stAng, endAng, swAng < 0);
          penX = cx + wR * Math.cos(endAng);
          penY = cy + hR * Math.sin(endAng);
          break;
        }
        case "Z":
          ctx.closePath();
          break;
      }
    }
  }
  return true;
}

export function presetTextRect(
  presetName: string,
  w: number,
  h: number,
  adjs?: number[],
): { l: number; t: number; r: number; b: number } | undefined {
  const preset = (PRESET_SHAPES as Record<string, PresetShape | undefined>)[presetName];
  if (!preset || !preset.rect) return undefined;
  const env = makeEnv(preset, w, h, adjs);
  const r = preset.rect;
  return {
    l: resolveToken(r.l, env.symbols),
    t: resolveToken(r.t, env.symbols),
    r: resolveToken(r.r, env.symbols),
    b: resolveToken(r.b, env.symbols),
  };
}

export function hasPresetGeometry(name: string | undefined): boolean {
  if (!name) return false;
  const preset = (PRESET_SHAPES as Record<string, PresetShape | undefined>)[name];
  return !!preset && preset.paths.length > 0;
}
