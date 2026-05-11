// src/columnar.ts
var KIND_NAMES = ["n", "s", "inline", "b", "e", "str", "f"];
var DECODED = Symbol.for("xlcore.columnar.decoded");
function decodeWorkbookLayout(layout) {
  for (const wire of layout.sheets) {
    const sheet = wire;
    const tagged = sheet;
    if (tagged[DECODED])
      continue;
    decodeSheet(sheet);
    tagged[DECODED] = true;
  }
  return layout;
}
function decodeSheet(sheet) {
  const wire = sheet;
  const c = wire.cells;
  sheet.decodedCells = {
    count: c.count,
    r: decodeU32(c.r),
    c: decodeU32(c.c),
    kind: decodeU8(c.kind),
    valueIdx: decodeI32(c.valueIdx),
    formulaIdx: decodeI32(c.formulaIdx),
    styleIdx: decodeI32(c.styleIdx),
    runsIdx: decodeI32(c.runsIdx),
    rowPtr: decodeU32(c.rowPtr)
  };
  const m = wire.rowMeta;
  const index = decodeU32(m.index);
  const byIndex = new Map;
  for (let i = 0;i < m.count; i++)
    byIndex.set(index[i] ?? 0, i);
  const outlineLevelB64 = m.outlineLevel ?? "";
  const outlineLevel = outlineLevelB64 ? decodeU8(outlineLevelB64) : new Uint8Array(0);
  sheet.decodedRowMeta = {
    count: m.count,
    index,
    heightPx: decodeF32(m.heightPx),
    styleIdx: decodeI32(m.styleIdx),
    hidden: decodeU8(m.hidden),
    outlineLevel,
    byIndex
  };
  sheet.cells = undefined;
  sheet.rowMeta = undefined;
}
function decodeBytes(b64) {
  const bin = atob(b64);
  const out = new Uint8Array(bin.length);
  for (let i = 0;i < bin.length; i++)
    out[i] = bin.charCodeAt(i);
  return out;
}
function decodeU8(b64) {
  return decodeBytes(b64);
}
function decodeU32(b64) {
  const bytes = decodeBytes(b64);
  const aligned = new ArrayBuffer(bytes.byteLength);
  new Uint8Array(aligned).set(bytes);
  return new Uint32Array(aligned);
}
function decodeI32(b64) {
  const bytes = decodeBytes(b64);
  const aligned = new ArrayBuffer(bytes.byteLength);
  new Uint8Array(aligned).set(bytes);
  return new Int32Array(aligned);
}
function decodeF32(b64) {
  const bytes = decodeBytes(b64);
  const aligned = new ArrayBuffer(bytes.byteLength);
  new Uint8Array(aligned).set(bytes);
  return new Float32Array(aligned);
}
function materializeCell(sheet, i) {
  const cells = sheet.decodedCells;
  const valueIdx = cells.valueIdx[i] ?? -1;
  const formulaIdx = cells.formulaIdx[i] ?? -1;
  const styleIdx = cells.styleIdx[i] ?? -1;
  const runsIdx = cells.runsIdx[i] ?? -1;
  const value = valueIdx >= 0 ? sheet.valuePool[valueIdx] : undefined;
  const formula = formulaIdx >= 0 ? sheet.formulaPool[formulaIdx] : undefined;
  const runs = runsIdx >= 0 ? sheet.inlineRuns[runsIdx] ?? [] : [];
  return {
    r: cells.r[i] ?? 0,
    c: cells.c[i] ?? 0,
    type: KIND_NAMES[cells.kind[i] ?? 0] ?? "n",
    value,
    formula,
    styleIndex: styleIdx >= 0 ? styleIdx : undefined,
    runs
  };
}
function iterCellsInRange(sheet, firstRow, lastRow, firstCol, lastCol, fn) {
  const meta = sheet.decodedRowMeta;
  const cells = sheet.decodedCells;
  if (meta.count === 0 || cells.count === 0)
    return;
  if (firstRow > lastRow || firstCol > lastCol)
    return;
  const startMeta = lowerBound(meta.index, firstRow, 0, meta.count);
  for (let m = startMeta;m < meta.count; m++) {
    const rowIdx = meta.index[m] ?? 0;
    if (rowIdx > lastRow)
      break;
    const start = cells.rowPtr[m] ?? 0;
    const end = cells.rowPtr[m + 1] ?? cells.count;
    if (start === end)
      continue;
    let i = lowerBound(cells.c, firstCol, start, end);
    for (;i < end; i++) {
      const col = cells.c[i] ?? 0;
      if (col > lastCol)
        break;
      fn(materializeCell(sheet, i), i);
    }
  }
}
function iterAllCells(sheet, fn) {
  const cells = sheet.decodedCells;
  for (let i = 0;i < cells.count; i++)
    fn(materializeCell(sheet, i), i);
}
function iterRows(sheet, fn) {
  const meta = sheet.decodedRowMeta;
  for (let i = 0;i < meta.count; i++) {
    const h = meta.heightPx[i] ?? Number.NaN;
    const s = meta.styleIdx[i] ?? -1;
    fn({
      index: meta.index[i] ?? 0,
      heightPx: Number.isNaN(h) ? undefined : h,
      styleIndex: s >= 0 ? s : undefined,
      hidden: (meta.hidden[i] ?? 0) !== 0
    });
  }
}
function findCell(sheet, r, c) {
  const meta = sheet.decodedRowMeta;
  const m = meta.byIndex.get(r);
  if (m === undefined)
    return;
  const cells = sheet.decodedCells;
  const start = cells.rowPtr[m] ?? 0;
  const end = cells.rowPtr[m + 1] ?? cells.count;
  let lo = start;
  let hi = end - 1;
  while (lo <= hi) {
    const mid = lo + hi >> 1;
    const col = cells.c[mid] ?? 0;
    if (col === c)
      return materializeCell(sheet, mid);
    if (col < c)
      lo = mid + 1;
    else
      hi = mid - 1;
  }
  return;
}
function lowerBound(arr, target, lo, hi) {
  while (lo < hi) {
    const mid = lo + hi >> 1;
    if ((arr[mid] ?? 0) < target)
      lo = mid + 1;
    else
      hi = mid;
  }
  return lo;
}

// src/color.ts
var INDEXED_PALETTE = {
  0: "#000000",
  1: "#ffffff",
  2: "#ff0000",
  3: "#00ff00",
  4: "#0000ff",
  5: "#ffff00",
  6: "#ff00ff",
  7: "#00ffff",
  8: "#000000",
  9: "#ffffff",
  10: "#ff0000",
  11: "#00ff00",
  12: "#0000ff",
  13: "#ffff00",
  14: "#ff00ff",
  15: "#00ffff",
  16: "#800000",
  17: "#008000",
  18: "#000080",
  19: "#808000",
  20: "#800080",
  21: "#008080",
  22: "#c0c0c0",
  23: "#808080",
  24: "#9999ff",
  25: "#993366",
  26: "#ffffcc",
  27: "#ccffff",
  28: "#660066",
  29: "#ff8080",
  30: "#0066cc",
  31: "#ccccff",
  32: "#000080",
  33: "#ff00ff",
  34: "#ffff00",
  35: "#00ffff",
  36: "#800080",
  37: "#800000",
  38: "#008080",
  39: "#0000ff",
  40: "#00ccff",
  41: "#ccffff",
  42: "#ccffcc",
  43: "#ffff99",
  44: "#99ccff",
  45: "#ff99cc",
  46: "#cc99ff",
  47: "#ffcc99",
  48: "#3366ff",
  49: "#33cccc",
  50: "#99cc00",
  51: "#ffcc00",
  52: "#ff9900",
  53: "#ff6600",
  54: "#666699",
  55: "#969696",
  56: "#003366",
  57: "#339966",
  58: "#003300",
  59: "#333300",
  60: "#993300",
  61: "#993366",
  62: "#333399",
  63: "#333333",
  64: "#000000",
  65: "#ffffff"
};
var DEFAULT_THEME_PALETTE = {
  0: "#ffffff",
  1: "#000000",
  2: "#e7e6e6",
  3: "#44546a",
  4: "#4472c4",
  5: "#ed7d31",
  6: "#a5a5a5",
  7: "#ffc000",
  8: "#5b9bd5",
  9: "#70ad47",
  10: "#0563c1",
  11: "#954f72"
};
var activeThemePalette = DEFAULT_THEME_PALETTE;
function setActiveTheme(theme) {
  if (!theme || !theme.colors || theme.colors.length === 0) {
    activeThemePalette = DEFAULT_THEME_PALETTE;
    return;
  }
  const map = { ...DEFAULT_THEME_PALETTE };
  theme.colors.forEach((hex, i) => {
    if (hex && /^[0-9a-fA-F]{6}$/.test(hex))
      map[i] = "#" + hex.toLowerCase();
  });
  activeThemePalette = map;
}
function activeThemeColor(index, fallback) {
  return activeThemePalette[index] ?? fallback;
}
function parseRgbString(rgb) {
  if (rgb.length === 8)
    return "#" + rgb.slice(2);
  if (rgb.length === 6)
    return "#" + rgb;
  return null;
}
function applyTint(hex, tint) {
  const r = parseInt(hex.slice(1, 3), 16) / 255;
  const g = parseInt(hex.slice(3, 5), 16) / 255;
  const b = parseInt(hex.slice(5, 7), 16) / 255;
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const l = (max + min) / 2;
  let h = 0;
  let s = 0;
  if (max !== min) {
    const d = max - min;
    s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
    switch (max) {
      case r:
        h = (g - b) / d + (g < b ? 6 : 0);
        break;
      case g:
        h = (b - r) / d + 2;
        break;
      default:
        h = (r - g) / d + 4;
    }
    h /= 6;
  }
  let l2 = tint < 0 ? l * (1 + tint) : l * (1 - tint) + tint;
  if (l2 < 0)
    l2 = 0;
  if (l2 > 1)
    l2 = 1;
  let r2, g2, b2;
  if (s === 0) {
    r2 = g2 = b2 = l2;
  } else {
    const q = l2 < 0.5 ? l2 * (1 + s) : l2 + s - l2 * s;
    const p = 2 * l2 - q;
    const hue2rgb = (t) => {
      if (t < 0)
        t += 1;
      if (t > 1)
        t -= 1;
      if (t < 1 / 6)
        return p + (q - p) * 6 * t;
      if (t < 1 / 2)
        return q;
      if (t < 2 / 3)
        return p + (q - p) * (2 / 3 - t) * 6;
      return p;
    };
    r2 = hue2rgb(h + 1 / 3);
    g2 = hue2rgb(h);
    b2 = hue2rgb(h - 1 / 3);
  }
  const toHex = (v) => Math.round(v * 255).toString(16).padStart(2, "0");
  return "#" + toHex(r2) + toHex(g2) + toHex(b2);
}
function colorToCss(c, fallback = "#000000") {
  if (!c)
    return fallback;
  let base = null;
  if (c.rgb)
    base = parseRgbString(c.rgb);
  else if (c.theme !== undefined)
    base = activeThemePalette[c.theme] ?? null;
  else if (c.indexed !== undefined)
    base = INDEXED_PALETTE[c.indexed] ?? null;
  if (!base)
    return fallback;
  if (c.tint && c.tint !== 0)
    return applyTint(base, c.tint);
  return base;
}
// src/grid.ts
var HEADER_H = 22;
var HEADER_W = 44;
var OUTLINE_GUTTER_STEP = 12;
var OUTLINE_GUTTER_PAD = 4;
var SHEET_MAX_COL = 16384;
var SHEET_MAX_ROW = 1048576;
function buildGrid(sheet, colOverrides, rowOverrides, requiredFarX, requiredFarY) {
  let minCols = Math.max(sheet.maxCol, 1);
  let minRows = Math.max(sheet.maxRow, 1);
  const viewportOnly = requiredFarX !== undefined || requiredFarY !== undefined;
  if (viewportOnly) {
    minCols = 1;
    minRows = 1;
  }
  if (sheet.drawings) {
    for (const d of sheet.drawings) {
      minCols = Math.max(minCols, d.anchor.toCol + 2);
      minRows = Math.max(minRows, d.anchor.toRow + 2);
    }
  }
  let maxCol = Math.min(minCols + 2, SHEET_MAX_COL);
  let maxRow = Math.min(minRows + 5, SHEET_MAX_ROW);
  const colSpecW = new Map;
  for (const c of sheet.cols) {
    const w = c.hidden ? 0 : c.widthPx;
    for (let i = c.min;i <= c.max; i++)
      colSpecW.set(i, w);
  }
  if (colOverrides)
    for (const [c, w] of colOverrides)
      colSpecW.set(c, Math.max(0, w));
  const widthOf = (c) => colSpecW.get(c) ?? sheet.defaultColWidthPx;
  const rowSpecH = new Map;
  iterRows(sheet, (row) => {
    if (row.hidden)
      rowSpecH.set(row.index, 0);
    else if (row.heightPx !== undefined)
      rowSpecH.set(row.index, row.heightPx);
  });
  if (rowOverrides)
    for (const [r, h] of rowOverrides)
      rowSpecH.set(r, Math.max(0, h));
  const heightOf = (r) => rowSpecH.get(r) ?? sheet.defaultRowHeightPx;
  let rowOutlineDepth = 0;
  if (sheet.decodedRowMeta && sheet.decodedRowMeta.outlineLevel.length > 0) {
    for (let i = 0;i < sheet.decodedRowMeta.outlineLevel.length; i++) {
      const v = sheet.decodedRowMeta.outlineLevel[i] ?? 0;
      if (v > rowOutlineDepth)
        rowOutlineDepth = v;
    }
  }
  let colOutlineDepth = 0;
  for (const c of sheet.cols) {
    const v = c.outlineLevel ?? 0;
    if (v > colOutlineDepth)
      colOutlineDepth = v;
  }
  const rowGutterW = rowOutlineDepth > 0 ? OUTLINE_GUTTER_PAD * 2 + (rowOutlineDepth + 1) * OUTLINE_GUTTER_STEP : 0;
  const colGutterH = colOutlineDepth > 0 ? OUTLINE_GUTTER_PAD * 2 + (colOutlineDepth + 1) * OUTLINE_GUTTER_STEP : 0;
  const originX = HEADER_W + rowGutterW;
  const originY = HEADER_H + colGutterH;
  const colW = [0];
  const colX = [0, originX];
  for (let c = 1;c <= maxCol; c++) {
    const w = widthOf(c);
    colW[c] = w;
    colX[c + 1] = (colX[c] ?? originX) + w;
  }
  while (requiredFarX !== undefined && maxCol < SHEET_MAX_COL && (colX[maxCol + 1] ?? originX) < requiredFarX) {
    maxCol++;
    const w = widthOf(maxCol);
    colW[maxCol] = w;
    colX[maxCol + 1] = (colX[maxCol] ?? originX) + w;
  }
  const rowH = [0];
  const rowY = [0, originY];
  for (let r = 1;r <= maxRow; r++) {
    const h = heightOf(r);
    rowH[r] = h;
    rowY[r + 1] = (rowY[r] ?? originY) + h;
  }
  while (requiredFarY !== undefined && maxRow < SHEET_MAX_ROW && (rowY[maxRow + 1] ?? originY) < requiredFarY) {
    maxRow++;
    const h = heightOf(maxRow);
    rowH[maxRow] = h;
    rowY[maxRow + 1] = (rowY[maxRow] ?? originY) + h;
  }
  return {
    colX,
    colW,
    rowY,
    rowH,
    totalW: colX[maxCol + 1] ?? originX,
    totalH: rowY[maxRow + 1] ?? originY,
    maxCol,
    maxRow,
    rowGutterW,
    colGutterH,
    originX,
    originY,
    rowOutlineDepth,
    colOutlineDepth
  };
}
function colLabel(n) {
  let s = "";
  while (n > 0) {
    const r = (n - 1) % 26;
    s = String.fromCharCode(65 + r) + s;
    n = Math.floor((n - 1) / 26);
  }
  return s;
}
var PX_PER_EMU = 1 / 9525;
function anchorToRect(d, g) {
  const a = d.anchor;
  const fromX = colEdge(g, a.fromCol + 1) + a.fromColOffEmu * PX_PER_EMU;
  const fromY = rowEdge(g, a.fromRow + 1) + a.fromRowOffEmu * PX_PER_EMU;
  const toX = colEdge(g, a.toCol + 1) + a.toColOffEmu * PX_PER_EMU;
  const toY = rowEdge(g, a.toRow + 1) + a.toRowOffEmu * PX_PER_EMU;
  const w = toX - fromX;
  const h = toY - fromY;
  if (w <= 1 || h <= 1)
    return null;
  return { x: fromX, y: fromY, w, h };
}
function colEdge(g, c) {
  if (c >= 1 && c < g.colX.length)
    return g.colX[c] ?? g.originX;
  const lastIdx = g.colX.length - 1;
  const last = g.colX[lastIdx] ?? g.originX;
  const prev = g.colX[lastIdx - 1] ?? g.originX;
  const w = Math.max(40, last - prev);
  return last + (c - lastIdx) * w;
}
function rowEdge(g, r) {
  if (r >= 1 && r < g.rowY.length)
    return g.rowY[r] ?? g.originY;
  const lastIdx = g.rowY.length - 1;
  const last = g.rowY[lastIdx] ?? g.originY;
  const prev = g.rowY[lastIdx - 1] ?? g.originY;
  const h = Math.max(20, last - prev);
  return last + (r - lastIdx) * h;
}
// src/renderConstants.ts
var GRID_COLOR = "#d9d9d9";
var HEADER_BG = "#f3f4f6";
var HEADER_FG = "#374151";
var HEADER_BORDER = "#cbd5e1";
var GUTTER_LINE = "#6b7280";
var SELECTION_STROKE = "#137e43";
var SELECTION_FILL = "rgba(19, 126, 67, 0.10)";
var HEADER_HIGHLIGHT = "#caead8";

// src/geometry.ts
function drawGridLines(ctx, sheet, g, vis) {
  if (!sheet.showGridLines)
    return;
  const top = g.rowY[vis.firstRow] ?? g.originY;
  const bot = g.rowY[vis.lastRow + 1] ?? g.totalH;
  const left = g.colX[vis.firstCol] ?? g.originX;
  const right = g.colX[vis.lastCol + 1] ?? g.totalW;
  ctx.strokeStyle = GRID_COLOR;
  ctx.lineWidth = 1;
  ctx.beginPath();
  for (let c = vis.firstCol;c <= vis.lastCol + 1; c++) {
    const x = Math.round(g.colX[c] ?? 0) + 0.5;
    ctx.moveTo(x, top);
    ctx.lineTo(x, bot);
  }
  for (let r = vis.firstRow;r <= vis.lastRow + 1; r++) {
    const y = Math.round(g.rowY[r] ?? 0) + 0.5;
    ctx.moveTo(left, y);
    ctx.lineTo(right, y);
  }
  ctx.stroke();
}
function cellRect(g, r, c) {
  return { x: g.colX[c] ?? 0, y: g.rowY[r] ?? 0, w: g.colW[c] ?? 0, h: g.rowH[r] ?? 0 };
}
function mergedRect(g, m) {
  const x = g.colX[m.c1] ?? 0;
  const y = g.rowY[m.r1] ?? 0;
  return {
    x,
    y,
    w: (g.colX[m.c2 + 1] ?? x) - x,
    h: (g.rowY[m.r2 + 1] ?? y) - y
  };
}
function buildMergeMaps(sheet) {
  const covered = new Set;
  const topLeftOf = new Map;
  for (const m of sheet.merges) {
    for (let r = m.r1;r <= m.r2; r++) {
      for (let c = m.c1;c <= m.c2; c++) {
        const k = `${r}:${c}`;
        topLeftOf.set(k, m);
        if (!(r === m.r1 && c === m.c1))
          covered.add(k);
      }
    }
  }
  return { covered, topLeftOf };
}
function rectFor(sheet, g, r, c, topLeftOf) {
  const m = topLeftOf.get(`${r}:${c}`);
  return m ? mergedRect(g, m) : cellRect(g, r, c);
}

// src/panes.ts
function frozenExtent(sheet, g) {
  const fz = sheet.freeze;
  const splitX = fz && fz.leftCol > 1 ? fz.leftCol : 1;
  const splitY = fz && fz.topRow > 1 ? fz.topRow : 1;
  const pcw = splitX > 1 ? (g.colX[splitX] ?? g.originX) - g.originX : 0;
  const prh = splitY > 1 ? (g.rowY[splitY] ?? g.originY) - g.originY : 0;
  return { splitX, splitY, pcw, prh };
}
function splitPanes(sheet, g, vp, canvasW, canvasH) {
  const { splitX, splitY, pcw, prh } = frozenExtent(sheet, g);
  const hasH = splitX > 1;
  const hasV = splitY > 1;
  const vpx = vp ? vp.x : 0;
  const vpy = vp ? vp.y : 0;
  const panes = [];
  {
    const cx = g.originX + pcw;
    const cy = g.originY + prh;
    const cw = Math.max(0, canvasW - cx);
    const ch = Math.max(0, canvasH - cy);
    const tx = -vpx;
    const ty = -vpy;
    const vis = paneVisible(g, cx, cy, cw, ch, tx, ty);
    if (hasH)
      vis.firstCol = Math.max(vis.firstCol, splitX);
    if (hasV)
      vis.firstRow = Math.max(vis.firstRow, splitY);
    panes.push({ cx, cy, cw, ch, tx, ty, vis, kind: "br" });
  }
  if (hasV) {
    const cx = g.originX + pcw;
    const cy = g.originY;
    const cw = Math.max(0, canvasW - cx);
    const ch = prh;
    const tx = -vpx;
    const ty = 0;
    const vis = paneVisible(g, cx, cy, cw, ch, tx, ty);
    if (hasH)
      vis.firstCol = Math.max(vis.firstCol, splitX);
    vis.firstRow = 1;
    vis.lastRow = Math.min(vis.lastRow, splitY - 1);
    panes.push({ cx, cy, cw, ch, tx, ty, vis, kind: "tr" });
  }
  if (hasH) {
    const cx = g.originX;
    const cy = g.originY + prh;
    const cw = pcw;
    const ch = Math.max(0, canvasH - cy);
    const tx = 0;
    const ty = -vpy;
    const vis = paneVisible(g, cx, cy, cw, ch, tx, ty);
    vis.firstCol = 1;
    vis.lastCol = Math.min(vis.lastCol, splitX - 1);
    if (hasV)
      vis.firstRow = Math.max(vis.firstRow, splitY);
    panes.push({ cx, cy, cw, ch, tx, ty, vis, kind: "bl" });
  }
  if (hasH && hasV) {
    const cx = g.originX;
    const cy = g.originY;
    const cw = pcw;
    const ch = prh;
    const vis = {
      firstCol: 1,
      lastCol: splitX - 1,
      firstRow: 1,
      lastRow: splitY - 1
    };
    panes.push({ cx, cy, cw, ch, tx: 0, ty: 0, vis, kind: "tl" });
  }
  return panes;
}
function paneVisible(g, cx, cy, cw, ch, tx, ty) {
  const ax1 = cx - tx;
  const ay1 = cy - ty;
  return visibleRange(g, ax1, ay1, ax1 + cw, ay1 + ch);
}
function frozenDims(sheet, g) {
  return frozenExtent(sheet, g);
}
function visibleRange(g, x1, y1, x2, y2) {
  let firstCol = 1, lastCol = g.maxCol;
  for (let c = 1;c <= g.maxCol; c++) {
    const right = g.colX[c + 1] ?? g.colX[c] ?? 0;
    if (right > x1) {
      firstCol = c;
      break;
    }
  }
  for (let c = firstCol;c <= g.maxCol; c++) {
    const left = g.colX[c] ?? 0;
    if (left >= x2) {
      lastCol = c - 1;
      break;
    }
    lastCol = c;
  }
  let firstRow = 1, lastRow = g.maxRow;
  for (let r = 1;r <= g.maxRow; r++) {
    const bot = g.rowY[r + 1] ?? g.rowY[r] ?? 0;
    if (bot > y1) {
      firstRow = r;
      break;
    }
  }
  for (let r = firstRow;r <= g.maxRow; r++) {
    const top = g.rowY[r] ?? 0;
    if (top >= y2) {
      lastRow = r - 1;
      break;
    }
    lastRow = r;
  }
  return { firstCol, lastCol, firstRow, lastRow };
}
// src/selection.ts
function resolveSelection(opts, g) {
  if (opts.selection) {
    const s = opts.selection;
    return {
      r1: clamp(Math.min(s.r1, s.r2), 1, g.maxRow),
      r2: clamp(Math.max(s.r1, s.r2), 1, g.maxRow),
      c1: clamp(Math.min(s.c1, s.c2), 1, g.maxCol),
      c2: clamp(Math.max(s.c1, s.c2), 1, g.maxCol)
    };
  }
  if (opts.activeCell) {
    const a = opts.activeCell;
    if (a.r < 1 || a.c < 1)
      return null;
    return { r1: a.r, r2: a.r, c1: a.c, c2: a.c };
  }
  return null;
}
function clamp(v, lo, hi) {
  return v < lo ? lo : v > hi ? hi : v;
}
function drawSelection(ctx, sheet, g, sel, active) {
  const x1 = g.colX[sel.c1] ?? 0;
  const x2 = g.colX[sel.c2 + 1] ?? x1;
  const y1 = g.rowY[sel.r1] ?? 0;
  const y2 = g.rowY[sel.r2 + 1] ?? y1;
  if (x2 <= x1 || y2 <= y1)
    return;
  ctx.save();
  ctx.fillStyle = SELECTION_FILL;
  const isSingle = sel.r1 === sel.r2 && sel.c1 === sel.c2;
  if (!isSingle) {
    if (active && active.r >= sel.r1 && active.r <= sel.r2 && active.c >= sel.c1 && active.c <= sel.c2) {
      const { topLeftOf } = buildMergeMaps(sheet);
      const m = topLeftOf.get(`${active.r}:${active.c}`);
      const ar = m ? mergedRect(g, m) : cellRect(g, active.r, active.c);
      const { x: ax1, y: ay1 } = ar, ax2 = ar.x + ar.w, ay2 = ar.y + ar.h;
      if (ay1 > y1)
        ctx.fillRect(x1, y1, x2 - x1, ay1 - y1);
      if (ay2 < y2)
        ctx.fillRect(x1, ay2, x2 - x1, y2 - ay2);
      if (ax1 > x1)
        ctx.fillRect(x1, ay1, ax1 - x1, ay2 - ay1);
      if (ax2 < x2)
        ctx.fillRect(ax2, ay1, x2 - ax2, ay2 - ay1);
    } else {
      ctx.fillRect(x1, y1, x2 - x1, y2 - y1);
    }
  }
  ctx.strokeStyle = SELECTION_STROKE;
  ctx.lineWidth = 2;
  ctx.setLineDash([]);
  ctx.strokeRect(x1 + 1, y1 + 1, x2 - x1 - 2, y2 - y1 - 2);
  ctx.fillStyle = SELECTION_STROKE;
  ctx.fillRect(x2 - 4, y2 - 4, 6, 6);
  ctx.strokeStyle = "#ffffff";
  ctx.lineWidth = 1;
  ctx.strokeRect(x2 - 3.5, y2 - 3.5, 5, 5);
  ctx.restore();
}

// src/chartUtils.ts
var AXIS_FONT_SIZE = 10;
var LEGEND_FONT_SIZE = 11;
var GRIDLINE_COLOR = "#e5e7eb";
var AXIS_LABEL_COLOR = "#52525b";
var DATA_LABEL_FONT_SIZE = 9;
var DATA_LABEL_COLOR = "#1f2937";
function valueRange(rows) {
  let minV = 0, maxV = 0;
  for (const r of rows) {
    for (const v of r) {
      if (v > maxV)
        maxV = v;
      if (v < minV)
        minV = v;
    }
  }
  return { minV, maxV };
}
function buildStackedRows(series, categoryCount, percent) {
  const tops = series.map((_) => new Array(categoryCount).fill(0));
  for (let i = 0;i < categoryCount; i++) {
    let total = 0;
    if (percent) {
      for (const s of series)
        total += Math.max(0, s.values[i] ?? 0);
      if (total <= 0)
        total = 1;
    }
    let acc = 0;
    for (let si = 0;si < series.length; si++) {
      const raw = series[si].values[i] ?? 0;
      const v = percent ? Math.max(0, raw) / total * 100 : raw;
      acc += v;
      tops[si][i] = acc;
    }
  }
  return tops;
}
function drawAxisFrame(ctx, chart, rect, ticks, minV, maxV, horizontal, percent) {
  ctx.font = `${AXIS_FONT_SIZE}px -apple-system, "Helvetica Neue", Arial, sans-serif`;
  const labelStrings = ticks.map((t) => percent ? `${Math.round(t)}%` : formatAxisValue(t, chart.valueFormat));
  const yAxisW = Math.max(...labelStrings.map((s) => ctx.measureText(s).width)) + 8;
  const xAxisH = AXIS_FONT_SIZE + 8;
  const inner = horizontal ? { x: rect.x + yAxisW, y: rect.y, w: rect.w - yAxisW, h: rect.h - xAxisH } : { x: rect.x + yAxisW, y: rect.y, w: rect.w - yAxisW, h: rect.h - xAxisH };
  ctx.fillStyle = AXIS_LABEL_COLOR;
  ctx.strokeStyle = GRIDLINE_COLOR;
  ctx.lineWidth = 1;
  ctx.textAlign = horizontal ? "center" : "right";
  ctx.textBaseline = "middle";
  for (let ti = 0;ti < ticks.length; ti++) {
    const t = ticks[ti];
    const frac = (t - minV) / (maxV - minV);
    if (horizontal) {
      const x = inner.x + frac * inner.w;
      ctx.beginPath();
      ctx.moveTo(Math.round(x) + 0.5, inner.y);
      ctx.lineTo(Math.round(x) + 0.5, inner.y + inner.h);
      ctx.stroke();
      ctx.fillText(labelStrings[ti], x, inner.y + inner.h + xAxisH / 2);
    } else {
      const y = inner.y + (1 - frac) * inner.h;
      ctx.beginPath();
      ctx.moveTo(inner.x, Math.round(y) + 0.5);
      ctx.lineTo(inner.x + inner.w, Math.round(y) + 0.5);
      ctx.stroke();
      ctx.fillText(labelStrings[ti], inner.x - 4, y);
    }
  }
  ctx.strokeStyle = "#9ca3af";
  ctx.beginPath();
  ctx.moveTo(inner.x, Math.round(inner.y + inner.h) + 0.5);
  ctx.lineTo(inner.x + inner.w, Math.round(inner.y + inner.h) + 0.5);
  ctx.moveTo(Math.round(inner.x) + 0.5, inner.y);
  ctx.lineTo(Math.round(inner.x) + 0.5, inner.y + inner.h);
  ctx.stroke();
  return inner;
}
function drawCategoryAxis(ctx, chart, inner, categoryCount, horizontal) {
  ctx.font = `${AXIS_FONT_SIZE}px -apple-system, "Helvetica Neue", Arial, sans-serif`;
  ctx.fillStyle = AXIS_LABEL_COLOR;
  ctx.textAlign = "center";
  ctx.textBaseline = horizontal ? "middle" : "top";
  const denom = Math.max(1, categoryCount - 1);
  for (let i = 0;i < categoryCount; i++) {
    const x = inner.x + i / denom * inner.w;
    const label = chart.categories[i] ?? `${i + 1}`;
    ctx.fillText(label, x, inner.y + inner.h + 4);
  }
}
function withAlpha(color, alpha) {
  const m = /^#([0-9a-f]{6})$/i.exec(color);
  if (!m)
    return color;
  const hex = m[1];
  const r = parseInt(hex.slice(0, 2), 16);
  const g = parseInt(hex.slice(2, 4), 16);
  const b = parseInt(hex.slice(4, 6), 16);
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}
function drawLegend(ctx, series, rect) {
  ctx.font = `${LEGEND_FONT_SIZE}px -apple-system, "Helvetica Neue", Arial, sans-serif`;
  ctx.textBaseline = "middle";
  const swatchW = 10;
  const itemPad = 16;
  const widths = series.map((s) => swatchW + 6 + ctx.measureText(s.name || "").width);
  const totalW = widths.reduce((a, b) => a + b, 0) + itemPad * (series.length - 1);
  let x = rect.x + (rect.w - totalW) / 2;
  const y = rect.y + rect.h / 2;
  for (let i = 0;i < series.length; i++) {
    const s = series[i];
    ctx.fillStyle = s.color ?? "#4472C4";
    ctx.fillRect(x, y - swatchW / 2, swatchW, swatchW);
    ctx.fillStyle = AXIS_LABEL_COLOR;
    ctx.textAlign = "left";
    ctx.fillText(s.name || `Series ${i + 1}`, x + swatchW + 4, y);
    x += widths[i] + itemPad;
  }
}
function drawPlaceholderPlot(ctx, chart, rect) {
  ctx.fillStyle = "#f4f4f5";
  ctx.fillRect(rect.x, rect.y, rect.w, rect.h);
  ctx.fillStyle = AXIS_LABEL_COLOR;
  ctx.font = `12px -apple-system, "Helvetica Neue", Arial, sans-serif`;
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  const label = `${chart.type} chart (renderer v0 stub)`;
  ctx.fillText(label, rect.x + rect.w / 2, rect.y + rect.h / 2);
}
function effectiveLabels(chart, s) {
  return s.dataLabels ?? chart.dataLabels;
}
function buildLabelText(dl, chart, series, categoryIdx, value, categoryTotal) {
  const sep = dl.separator ?? ", ";
  const parts = [];
  if (dl.showSeriesName && series.name)
    parts.push(series.name);
  if (dl.showCategory) {
    const cats = chart.categories ?? [];
    const c = cats[categoryIdx];
    if (c != null && c !== "")
      parts.push(c);
  }
  if (dl.showPercent && categoryTotal > 0) {
    const pct = value / categoryTotal * 100;
    parts.push(`${Math.round(pct)}%`);
  }
  if (dl.showValue) {
    const fmt = dl.numFmt ?? chart.valueFormat;
    parts.push(formatAxisValue(value, fmt));
  }
  return parts.join(sep);
}
function drawLabel(ctx, text, x, y, align = "center", baseline = "middle") {
  if (!text)
    return;
  ctx.font = `${DATA_LABEL_FONT_SIZE}px -apple-system, "Helvetica Neue", Arial, sans-serif`;
  ctx.textAlign = align;
  ctx.textBaseline = baseline;
  ctx.lineWidth = 3;
  ctx.strokeStyle = "rgba(255,255,255,0.85)";
  ctx.lineJoin = "round";
  ctx.strokeText(text, x, y);
  ctx.lineWidth = 1;
  ctx.fillStyle = DATA_LABEL_COLOR;
  ctx.fillText(text, x, y);
}
function niceTicks(min, max, count) {
  if (max === min) {
    max = min + 1;
  }
  const range = niceNum(max - min, false);
  const step = niceNum(range / Math.max(1, count - 1), true);
  const niceMin = Math.floor(min / step) * step;
  const niceMax = Math.ceil(max / step) * step;
  const out = [];
  for (let v = niceMin;v <= niceMax + step / 2; v += step) {
    out.push(parseFloat(v.toPrecision(12)));
  }
  return out;
}
function niceNum(range, round) {
  const exp = Math.floor(Math.log10(Math.max(0.000000000001, Math.abs(range))));
  const f = range / Math.pow(10, exp);
  let nf;
  if (round) {
    if (f < 1.5)
      nf = 1;
    else if (f < 3)
      nf = 2;
    else if (f < 7)
      nf = 5;
    else
      nf = 10;
  } else {
    if (f <= 1)
      nf = 1;
    else if (f <= 2)
      nf = 2;
    else if (f <= 5)
      nf = 5;
    else
      nf = 10;
  }
  return nf * Math.pow(10, exp);
}
function formatAxisValue(v, fmt) {
  if (!fmt || fmt === "General")
    return formatGeneral(v);
  const stripped = fmt.replace(/\[[^\]]*\]/g, "");
  const section = stripped.split(";")[0] ?? stripped;
  const decimals = decimalsIn(section);
  if (section.includes("%"))
    return (v * 100).toFixed(decimals) + "%";
  if (section.includes("$")) {
    const grouped = section.includes(",") || section.includes("#,##");
    return "$" + (grouped ? withGrouping(v, decimals) : v.toFixed(decimals));
  }
  if (section.includes(","))
    return withGrouping(v, decimals);
  if (section.includes("0") || section.includes("#"))
    return v.toFixed(decimals);
  return formatGeneral(v);
}
function formatGeneral(v) {
  if (Number.isInteger(v) && Math.abs(v) < 1000000000000000)
    return v.toString();
  return parseFloat(v.toPrecision(8)).toString();
}
function decimalsIn(fmt) {
  const i = fmt.indexOf(".");
  if (i < 0)
    return 0;
  let n = 0;
  for (let j = i + 1;j < fmt.length; j++) {
    const ch = fmt[j];
    if (ch === "0" || ch === "#")
      n++;
    else
      break;
  }
  return n;
}
function withGrouping(v, decimals) {
  const neg = v < 0;
  const abs = Math.abs(v).toFixed(decimals);
  const [intPart, frac] = abs.split(".");
  const grouped = (intPart ?? "0").replace(/\B(?=(\d{3})+(?!\d))/g, ",");
  return (neg ? "-" : "") + grouped + (frac ? "." + frac : "");
}

// src/chart.ts
var TITLE_PAD = 8;
var TITLE_FONT_SIZE = 14;
var AXIS_FONT_SIZE2 = 10;
var LEGEND_FONT_SIZE2 = 11;
var PLOT_PAD_LEFT = 8;
var PLOT_PAD_RIGHT = 12;
var AXIS_TICK_COUNT = 5;
var GRIDLINE_COLOR2 = "#e5e7eb";
var AXIS_LABEL_COLOR2 = "#52525b";
var TITLE_COLOR = "#262626";
function drawChart(ctx, chart, rect) {
  ctx.fillStyle = "#ffffff";
  ctx.fillRect(rect.x, rect.y, rect.w, rect.h);
  ctx.strokeStyle = "#d4d4d8";
  ctx.lineWidth = 1;
  ctx.strokeRect(rect.x + 0.5, rect.y + 0.5, rect.w - 1, rect.h - 1);
  let cursorY = rect.y + TITLE_PAD;
  if (chart.title) {
    ctx.fillStyle = TITLE_COLOR;
    ctx.font = `${TITLE_FONT_SIZE}px -apple-system, "Helvetica Neue", Arial, sans-serif`;
    ctx.textAlign = "center";
    ctx.textBaseline = "top";
    ctx.fillText(chart.title, rect.x + rect.w / 2, cursorY);
    cursorY += TITLE_FONT_SIZE + TITLE_PAD;
  }
  const legendH = chart.series.length > 0 ? LEGEND_FONT_SIZE2 + 14 : 0;
  const legendRect = {
    x: rect.x,
    y: rect.y + rect.h - legendH,
    w: rect.w,
    h: legendH
  };
  const plotRect = {
    x: rect.x + PLOT_PAD_LEFT,
    y: cursorY,
    w: rect.w - PLOT_PAD_LEFT - PLOT_PAD_RIGHT,
    h: rect.y + rect.h - cursorY - legendH - 4
  };
  if (plotRect.w <= 20 || plotRect.h <= 20)
    return;
  switch (chart.type) {
    case "column":
    case "bar":
      drawBarColumnChart(ctx, chart, plotRect);
      break;
    case "line":
      drawLineChart(ctx, chart, plotRect);
      break;
    case "area":
      drawAreaChart(ctx, chart, plotRect);
      break;
    case "pie":
    case "doughnut":
      drawPieChart(ctx, chart, plotRect);
      break;
    case "scatter":
      drawScatterChart(ctx, chart, plotRect);
      break;
    default:
      drawPlaceholderPlot(ctx, chart, plotRect);
  }
  if (legendH > 0)
    drawLegend(ctx, chart.series, legendRect);
}
function drawBarColumnChart(ctx, chart, rect) {
  const horizontal = chart.type === "bar";
  const stacked = chart.grouping === "stacked" || chart.grouping === "percentstacked";
  const series = chart.series.filter((s) => s.values.length > 0);
  if (series.length === 0) {
    drawPlaceholderPlot(ctx, chart, rect);
    return;
  }
  const categoryCount = Math.max(...series.map((s) => s.values.length), chart.categories.length);
  if (categoryCount === 0)
    return;
  let minV = 0, maxV = 0;
  if (stacked) {
    for (let i = 0;i < categoryCount; i++) {
      let pos = 0, neg = 0;
      for (const s of series) {
        const v = s.values[i] ?? 0;
        if (v >= 0)
          pos += v;
        else
          neg += v;
      }
      if (pos > maxV)
        maxV = pos;
      if (neg < minV)
        minV = neg;
    }
  } else {
    for (const s of series) {
      for (const v of s.values) {
        if (v > maxV)
          maxV = v;
        if (v < minV)
          minV = v;
      }
    }
  }
  if (minV > 0)
    minV = 0;
  if (maxV < 0)
    maxV = 0;
  if (minV === 0 && maxV === 0)
    maxV = 1;
  const ticks = niceTicks(minV, maxV, AXIS_TICK_COUNT);
  minV = ticks[0];
  maxV = ticks[ticks.length - 1];
  ctx.font = `${AXIS_FONT_SIZE2}px -apple-system, "Helvetica Neue", Arial, sans-serif`;
  const labelStrings = ticks.map((t) => formatAxisValue(t, chart.valueFormat));
  const yAxisW = Math.max(...labelStrings.map((s) => ctx.measureText(s).width)) + 8;
  const xAxisH = AXIS_FONT_SIZE2 + 8;
  const innerRect = horizontal ? { x: rect.x + yAxisW, y: rect.y, w: rect.w - yAxisW, h: rect.h - xAxisH } : { x: rect.x + yAxisW, y: rect.y, w: rect.w - yAxisW, h: rect.h - xAxisH };
  ctx.fillStyle = AXIS_LABEL_COLOR2;
  ctx.strokeStyle = GRIDLINE_COLOR2;
  ctx.lineWidth = 1;
  ctx.textAlign = horizontal ? "center" : "right";
  ctx.textBaseline = "middle";
  for (let ti = 0;ti < ticks.length; ti++) {
    const t = ticks[ti];
    const frac = (t - minV) / (maxV - minV);
    if (horizontal) {
      const x = innerRect.x + frac * innerRect.w;
      ctx.beginPath();
      ctx.moveTo(Math.round(x) + 0.5, innerRect.y);
      ctx.lineTo(Math.round(x) + 0.5, innerRect.y + innerRect.h);
      ctx.stroke();
      ctx.fillText(labelStrings[ti], x, innerRect.y + innerRect.h + xAxisH / 2);
    } else {
      const y = innerRect.y + (1 - frac) * innerRect.h;
      ctx.beginPath();
      ctx.moveTo(innerRect.x, Math.round(y) + 0.5);
      ctx.lineTo(innerRect.x + innerRect.w, Math.round(y) + 0.5);
      ctx.stroke();
      ctx.fillText(labelStrings[ti], innerRect.x - 4, y);
    }
  }
  const groupSize = stacked ? 1 : series.length;
  const groupGap = horizontal ? innerRect.h / categoryCount : innerRect.w / categoryCount;
  const barGapFrac = 0.25;
  const innerGapFrac = 0.05;
  const usableGroup = groupGap * (1 - barGapFrac);
  const barSize = stacked ? usableGroup : usableGroup * (1 - innerGapFrac * (groupSize - 1)) / groupSize;
  const zeroFrac = (0 - minV) / (maxV - minV);
  const zeroY = innerRect.y + (1 - zeroFrac) * innerRect.h;
  ctx.fillStyle = AXIS_LABEL_COLOR2;
  ctx.textAlign = "center";
  ctx.textBaseline = horizontal ? "middle" : "top";
  for (let i = 0;i < categoryCount; i++) {
    const center = horizontal ? innerRect.y + (i + 0.5) * groupGap : innerRect.x + (i + 0.5) * groupGap;
    const label = chart.categories[i] ?? `${i + 1}`;
    if (horizontal) {
      ctx.textAlign = "right";
      ctx.fillText(label, innerRect.x - 4, center);
    } else {
      ctx.fillText(label, center, innerRect.y + innerRect.h + 4);
    }
  }
  ctx.textAlign = "left";
  if (stacked) {
    for (let i = 0;i < categoryCount; i++) {
      const groupCenter = horizontal ? innerRect.y + (i + 0.5) * groupGap : innerRect.x + (i + 0.5) * groupGap;
      let pos = 0, neg = 0;
      let catTotal = 0;
      for (const s of series)
        catTotal += Math.max(0, s.values[i] ?? 0);
      for (const s of series) {
        const v = s.values[i] ?? 0;
        const start = v >= 0 ? pos : neg;
        const end = v >= 0 ? pos + v : neg + v;
        if (v >= 0)
          pos += v;
        else
          neg += v;
        const sFrac = (start - minV) / (maxV - minV);
        const eFrac = (end - minV) / (maxV - minV);
        ctx.fillStyle = s.color ?? "#4472C4";
        let bx = 0, by = 0, bw = 0, bh = 0;
        if (horizontal) {
          const xa = innerRect.x + sFrac * innerRect.w;
          const xb = innerRect.x + eFrac * innerRect.w;
          bx = Math.min(xa, xb);
          by = groupCenter - barSize / 2;
          bw = Math.abs(xb - xa);
          bh = barSize;
        } else {
          const ya = innerRect.y + (1 - sFrac) * innerRect.h;
          const yb = innerRect.y + (1 - eFrac) * innerRect.h;
          bx = groupCenter - barSize / 2;
          by = Math.min(ya, yb);
          bw = barSize;
          bh = Math.abs(yb - ya);
        }
        ctx.fillRect(bx, by, bw, bh);
        const dl = effectiveLabels(chart, s);
        if (dl) {
          const text = buildLabelText(dl, chart, s, i, v, catTotal);
          drawLabel(ctx, text, bx + bw / 2, by + bh / 2);
        }
      }
    }
  } else {
    for (let i = 0;i < categoryCount; i++) {
      for (let si = 0;si < series.length; si++) {
        const s = series[si];
        const v = s.values[i] ?? 0;
        const frac = (v - minV) / (maxV - minV);
        ctx.fillStyle = s.color ?? "#4472C4";
        let bx = 0, by = 0, bw = 0, bh = 0;
        if (horizontal) {
          const groupTop = innerRect.y + i * groupGap + (groupGap - usableGroup) / 2;
          const top = groupTop + si * (barSize + barSize * innerGapFrac);
          const x1 = innerRect.x + (0 - minV) / (maxV - minV) * innerRect.w;
          const x2 = innerRect.x + frac * innerRect.w;
          bx = Math.min(x1, x2);
          by = top;
          bw = Math.abs(x2 - x1);
          bh = barSize;
        } else {
          const groupLeft = innerRect.x + i * groupGap + (groupGap - usableGroup) / 2;
          const left = groupLeft + si * (barSize + barSize * innerGapFrac);
          const yTop = innerRect.y + (1 - frac) * innerRect.h;
          const yBot = zeroY;
          bx = left;
          by = Math.min(yTop, yBot);
          bw = barSize;
          bh = Math.abs(yBot - yTop);
        }
        ctx.fillRect(bx, by, bw, bh);
        const dl = effectiveLabels(chart, s);
        if (dl) {
          const text = buildLabelText(dl, chart, s, i, v, 0);
          const pos = dl.position ?? "outEnd";
          let lx = bx + bw / 2, ly = by + bh / 2;
          const PAD = 3;
          if (horizontal) {
            if (pos === "outEnd") {
              lx = v >= 0 ? bx + bw + PAD : bx - PAD;
            } else if (pos === "inEnd") {
              lx = v >= 0 ? bx + bw - PAD : bx + PAD;
            } else if (pos === "inBase") {
              lx = v >= 0 ? bx + PAD : bx + bw - PAD;
            }
            const align = pos === "outEnd" ? v >= 0 ? "left" : "right" : pos === "inEnd" ? v >= 0 ? "right" : "left" : pos === "inBase" ? v >= 0 ? "left" : "right" : "center";
            drawLabel(ctx, text, lx, ly, align, "middle");
          } else {
            if (pos === "outEnd") {
              ly = v >= 0 ? by - PAD : by + bh + PAD;
            } else if (pos === "inEnd") {
              ly = v >= 0 ? by + PAD : by + bh - PAD;
            } else if (pos === "inBase") {
              ly = v >= 0 ? by + bh - PAD : by + PAD;
            }
            const baseline = pos === "outEnd" ? v >= 0 ? "bottom" : "top" : pos === "inEnd" ? v >= 0 ? "top" : "bottom" : pos === "inBase" ? v >= 0 ? "bottom" : "top" : "middle";
            drawLabel(ctx, text, lx, ly, "center", baseline);
          }
        }
      }
    }
  }
  ctx.strokeStyle = "#9ca3af";
  ctx.beginPath();
  ctx.moveTo(innerRect.x, Math.round(zeroY) + 0.5);
  ctx.lineTo(innerRect.x + innerRect.w, Math.round(zeroY) + 0.5);
  ctx.moveTo(Math.round(innerRect.x) + 0.5, innerRect.y);
  ctx.lineTo(Math.round(innerRect.x) + 0.5, innerRect.y + innerRect.h);
  ctx.stroke();
}
function drawLineChart(ctx, chart, rect) {
  const series = chart.series.filter((s) => s.values.length > 0);
  if (series.length === 0) {
    drawPlaceholderPlot(ctx, chart, rect);
    return;
  }
  const categoryCount = Math.max(...series.map((s) => s.values.length), chart.categories.length);
  if (categoryCount === 0)
    return;
  const stacked = chart.grouping === "stacked" || chart.grouping === "percentstacked";
  const percent = chart.grouping === "percentstacked";
  const stackedSeries = stacked ? buildStackedRows(series, categoryCount, percent) : series.map((s) => Array.from({ length: categoryCount }, (_, i) => s.values[i] ?? 0));
  let { minV, maxV } = valueRange(stackedSeries);
  if (minV > 0)
    minV = 0;
  if (maxV < 0)
    maxV = 0;
  if (minV === maxV) {
    maxV = minV + 1;
  }
  const ticks = niceTicks(minV, maxV, AXIS_TICK_COUNT);
  minV = ticks[0];
  maxV = ticks[ticks.length - 1];
  const inner = drawAxisFrame(ctx, chart, rect, ticks, minV, maxV, false, percent);
  drawCategoryAxis(ctx, chart, inner, categoryCount, false);
  const xStep = inner.w / Math.max(1, categoryCount - 1);
  const yFor = (v) => inner.y + (1 - (v - minV) / (maxV - minV)) * inner.h;
  for (let si = 0;si < series.length; si++) {
    const s = series[si];
    const data = stackedSeries[si];
    ctx.strokeStyle = s.color ?? "#4472C4";
    ctx.lineWidth = 2;
    ctx.beginPath();
    for (let i = 0;i < categoryCount; i++) {
      const x = inner.x + i * xStep;
      const y = yFor(data[i] ?? 0);
      if (i === 0)
        ctx.moveTo(x, y);
      else
        ctx.lineTo(x, y);
    }
    ctx.stroke();
    ctx.fillStyle = s.color ?? "#4472C4";
    for (let i = 0;i < categoryCount; i++) {
      const x = inner.x + i * xStep;
      const y = yFor(data[i] ?? 0);
      ctx.beginPath();
      ctx.arc(x, y, 3, 0, Math.PI * 2);
      ctx.fill();
    }
    const dl = effectiveLabels(chart, s);
    if (dl) {
      const pos = dl.position ?? "t";
      const PAD = 5;
      for (let i = 0;i < categoryCount; i++) {
        const v = s.values[i] ?? 0;
        const text = buildLabelText(dl, chart, s, i, v, 0);
        if (!text)
          continue;
        const x = inner.x + i * xStep;
        const y = yFor(data[i] ?? 0);
        let lx = x, ly = y;
        let baseline = "bottom";
        if (pos === "b") {
          ly = y + PAD;
          baseline = "top";
        } else if (pos === "ctr") {
          baseline = "middle";
        } else if (pos === "l") {
          lx = x - PAD;
          baseline = "middle";
        } else if (pos === "r") {
          lx = x + PAD;
          baseline = "middle";
        } else {
          ly = y - PAD;
          baseline = "bottom";
        }
        const align = pos === "l" ? "right" : pos === "r" ? "left" : "center";
        drawLabel(ctx, text, lx, ly, align, baseline);
      }
    }
  }
  ctx.lineWidth = 1;
}
function drawAreaChart(ctx, chart, rect) {
  const series = chart.series.filter((s) => s.values.length > 0);
  if (series.length === 0) {
    drawPlaceholderPlot(ctx, chart, rect);
    return;
  }
  const categoryCount = Math.max(...series.map((s) => s.values.length), chart.categories.length);
  if (categoryCount === 0)
    return;
  const stacked = chart.grouping !== "standard";
  const percent = chart.grouping === "percentstacked";
  const tops = stacked ? buildStackedRows(series, categoryCount, percent) : series.map((s) => Array.from({ length: categoryCount }, (_, i) => s.values[i] ?? 0));
  const bottoms = stacked ? series.map((_, si) => si === 0 ? new Array(categoryCount).fill(0) : tops[si - 1].slice()) : series.map((_) => new Array(categoryCount).fill(0));
  let { minV, maxV } = valueRange([...tops, ...bottoms]);
  if (minV > 0)
    minV = 0;
  if (maxV < 0)
    maxV = 0;
  if (minV === maxV)
    maxV = minV + 1;
  const ticks = niceTicks(minV, maxV, AXIS_TICK_COUNT);
  minV = ticks[0];
  maxV = ticks[ticks.length - 1];
  const inner = drawAxisFrame(ctx, chart, rect, ticks, minV, maxV, false, percent);
  drawCategoryAxis(ctx, chart, inner, categoryCount, false);
  const xStep = inner.w / Math.max(1, categoryCount - 1);
  const yFor = (v) => inner.y + (1 - (v - minV) / (maxV - minV)) * inner.h;
  for (let si = 0;si < series.length; si++) {
    const s = series[si];
    const top = tops[si];
    const bot = bottoms[si];
    ctx.fillStyle = withAlpha(s.color ?? "#4472C4", stacked ? 0.85 : 0.55);
    ctx.beginPath();
    for (let i = 0;i < categoryCount; i++) {
      const x = inner.x + i * xStep;
      const y = yFor(top[i] ?? 0);
      if (i === 0)
        ctx.moveTo(x, y);
      else
        ctx.lineTo(x, y);
    }
    for (let i = categoryCount - 1;i >= 0; i--) {
      const x = inner.x + i * xStep;
      const y = yFor(bot[i] ?? 0);
      ctx.lineTo(x, y);
    }
    ctx.closePath();
    ctx.fill();
    ctx.strokeStyle = s.color ?? "#4472C4";
    ctx.lineWidth = 1.5;
    ctx.beginPath();
    for (let i = 0;i < categoryCount; i++) {
      const x = inner.x + i * xStep;
      const y = yFor(top[i] ?? 0);
      if (i === 0)
        ctx.moveTo(x, y);
      else
        ctx.lineTo(x, y);
    }
    ctx.stroke();
    const dl = effectiveLabels(chart, s);
    if (dl) {
      const PAD = 4;
      for (let i = 0;i < categoryCount; i++) {
        const v = s.values[i] ?? 0;
        const text = buildLabelText(dl, chart, s, i, v, 0);
        if (!text)
          continue;
        const x = inner.x + i * xStep;
        const y = yFor(top[i] ?? 0);
        drawLabel(ctx, text, x, y - PAD, "center", "bottom");
      }
    }
  }
  ctx.lineWidth = 1;
}
function drawPieChart(ctx, chart, rect) {
  const ser = chart.series[0];
  if (!ser || ser.values.length === 0) {
    drawPlaceholderPlot(ctx, chart, rect);
    return;
  }
  const total = ser.values.reduce((a, b) => a + Math.max(0, b), 0);
  if (total <= 0) {
    drawPlaceholderPlot(ctx, chart, rect);
    return;
  }
  const cx = rect.x + rect.w / 2;
  const cy = rect.y + rect.h / 2;
  const r = Math.min(rect.w, rect.h) / 2 - 8;
  const innerR = chart.type === "doughnut" ? r * 0.55 : 0;
  const palette = ["#4472C4", "#ED7D31", "#A5A5A5", "#FFC000", "#5B9BD5", "#70AD47"];
  const pointColors = ser.pointColors ?? [];
  const slices = [];
  let start = -Math.PI / 2;
  for (let i = 0;i < ser.values.length; i++) {
    const v = Math.max(0, ser.values[i] ?? 0);
    if (v <= 0)
      continue;
    const sweep = v / total * Math.PI * 2;
    const end = start + sweep;
    const explicit = pointColors[i];
    ctx.fillStyle = explicit && explicit.length > 0 ? explicit : palette[i % palette.length];
    ctx.beginPath();
    ctx.moveTo(cx, cy);
    ctx.arc(cx, cy, r, start, end);
    ctx.closePath();
    ctx.fill();
    ctx.strokeStyle = "#ffffff";
    ctx.lineWidth = 1.5;
    ctx.stroke();
    slices.push({ mid: (start + end) / 2, idx: i, v });
    start = end;
  }
  if (innerR > 0) {
    ctx.fillStyle = "#ffffff";
    ctx.beginPath();
    ctx.arc(cx, cy, innerR, 0, Math.PI * 2);
    ctx.fill();
  }
  ctx.lineWidth = 1;
  const dl = effectiveLabels(chart, ser);
  if (dl) {
    const pos = dl.position ?? "outEnd";
    const labelR = pos === "outEnd" || pos === "bestFit" ? r + 12 : pos === "ctr" ? (innerR + r) / 2 : r - 12;
    for (const sl of slices) {
      const text = buildLabelText(dl, chart, ser, sl.idx, sl.v, total);
      if (!text)
        continue;
      const lx = cx + Math.cos(sl.mid) * labelR;
      const ly = cy + Math.sin(sl.mid) * labelR;
      const align = pos === "outEnd" || pos === "bestFit" ? Math.cos(sl.mid) >= 0 ? "left" : "right" : "center";
      drawLabel(ctx, text, lx, ly, align, "middle");
    }
  }
}
function drawScatterChart(ctx, chart, rect) {
  const series = chart.series.filter((s) => s.values.length > 0);
  if (series.length === 0) {
    drawPlaceholderPlot(ctx, chart, rect);
    return;
  }
  const xCache = series.map((s) => {
    const xs = s.xValues ?? [];
    if (xs.length > 0)
      return xs.slice();
    return s.values.map((_, i) => {
      const c = chart.categories[i];
      const n = c == null ? i + 1 : parseFloat(c);
      return Number.isFinite(n) ? n : i + 1;
    });
  });
  let xMin = Infinity, xMax = -Infinity;
  let yMin = Infinity, yMax = -Infinity;
  for (let si = 0;si < series.length; si++) {
    const xs = xCache[si];
    const ys = series[si].values;
    const n = Math.min(xs.length, ys.length);
    for (let i = 0;i < n; i++) {
      const x = xs[i], y = ys[i];
      if (x < xMin)
        xMin = x;
      if (x > xMax)
        xMax = x;
      if (y < yMin)
        yMin = y;
      if (y > yMax)
        yMax = y;
    }
  }
  if (!Number.isFinite(xMin) || !Number.isFinite(yMin)) {
    drawPlaceholderPlot(ctx, chart, rect);
    return;
  }
  if (xMin === xMax) {
    xMax = xMin + 1;
  }
  if (yMin === yMax) {
    yMax = yMin + 1;
  }
  const xTicks = niceTicks(xMin, xMax, AXIS_TICK_COUNT);
  const yTicks = niceTicks(yMin, yMax, AXIS_TICK_COUNT);
  xMin = xTicks[0];
  xMax = xTicks[xTicks.length - 1];
  yMin = yTicks[0];
  yMax = yTicks[yTicks.length - 1];
  const inner = drawAxisFrame(ctx, chart, rect, yTicks, yMin, yMax, false, false);
  ctx.font = `${AXIS_FONT_SIZE2}px -apple-system, "Helvetica Neue", Arial, sans-serif`;
  ctx.fillStyle = AXIS_LABEL_COLOR2;
  ctx.textAlign = "center";
  ctx.textBaseline = "top";
  for (const t of xTicks) {
    const frac = (t - xMin) / (xMax - xMin);
    const x = inner.x + frac * inner.w;
    ctx.fillText(formatGeneral(t), x, inner.y + inner.h + 4);
  }
  const style = chart.scatterStyle;
  const drawLines = style === "line" || style === "lineMarker";
  const drawSmooth = style === "smooth" || style === "smoothMarker";
  const drawMarkers = style == null || style === "marker" || style === "lineMarker" || style === "smoothMarker";
  for (let si = 0;si < series.length; si++) {
    const s = series[si];
    const xs = xCache[si];
    const ys = s.values;
    const n = Math.min(xs.length, ys.length);
    if (n === 0)
      continue;
    const color = s.color ?? "#4472C4";
    ctx.fillStyle = color;
    ctx.strokeStyle = color;
    const dl = effectiveLabels(chart, s);
    const pts = [];
    for (let i = 0;i < n; i++) {
      const px = inner.x + (xs[i] - xMin) / (xMax - xMin) * inner.w;
      const py = inner.y + (1 - (ys[i] - yMin) / (yMax - yMin)) * inner.h;
      pts.push({ x: px, y: py, v: ys[i], i });
    }
    if ((drawLines || drawSmooth) && pts.length >= 2) {
      const sorted = pts.slice().sort((a, b) => a.x - b.x);
      ctx.lineWidth = 1.5;
      ctx.beginPath();
      ctx.moveTo(sorted[0].x, sorted[0].y);
      if (drawSmooth) {
        for (let k = 0;k < sorted.length - 1; k++) {
          const p0 = sorted[Math.max(0, k - 1)];
          const p1 = sorted[k];
          const p2 = sorted[k + 1];
          const p3 = sorted[Math.min(sorted.length - 1, k + 2)];
          const cp1x = p1.x + (p2.x - p0.x) / 6;
          const cp1y = p1.y + (p2.y - p0.y) / 6;
          const cp2x = p2.x - (p3.x - p1.x) / 6;
          const cp2y = p2.y - (p3.y - p1.y) / 6;
          ctx.bezierCurveTo(cp1x, cp1y, cp2x, cp2y, p2.x, p2.y);
        }
      } else {
        for (let k = 1;k < sorted.length; k++) {
          ctx.lineTo(sorted[k].x, sorted[k].y);
        }
      }
      ctx.stroke();
    }
    for (const p of pts) {
      if (drawMarkers) {
        ctx.beginPath();
        ctx.arc(p.x, p.y, 3.5, 0, Math.PI * 2);
        ctx.fill();
      }
      if (dl) {
        const text = buildLabelText(dl, chart, s, p.i, p.v, 0);
        if (text)
          drawLabel(ctx, text, p.x, p.y - 6, "center", "bottom");
      }
    }
  }
}

// src/drawings.ts
var imageCache = new Map;
function getOrLoadImage(uri) {
  const cached = imageCache.get(uri);
  if (cached)
    return imageHasSize(cached) ? cached : null;
  const img = new Image;
  const bytes = dataUriBytes(uri);
  if (bytes) {
    img.src = bytes;
    imageCache.set(uri, img);
    return imageHasSize(img) ? img : null;
  }
  img.decoding = "async";
  img.onload = () => {
    try {
      globalThis.dispatchEvent?.(new Event("xlcore-image-ready"));
    } catch {}
  };
  img.src = uri;
  imageCache.set(uri, img);
  return null;
}
function imageHasSize(img) {
  const measured = img;
  return (img.naturalWidth ?? measured.width ?? 0) > 0 && (img.naturalHeight ?? measured.height ?? 0) > 0;
}
function dataUriBytes(uri) {
  if (!uri.startsWith("data:"))
    return null;
  const comma = uri.indexOf(",");
  if (comma < 0 || !uri.slice(0, comma).includes(";base64"))
    return null;
  const BufferCtor = globalThis.Buffer;
  return BufferCtor?.from(uri.slice(comma + 1), "base64") ?? null;
}
function drawDrawings(ctx, sheet, g) {
  if (!sheet.drawings || sheet.drawings.length === 0)
    return;
  for (const d of sheet.drawings) {
    const rect = anchorToRect(d, g);
    if (!rect)
      continue;
    if (d.kind === "chart" && d.chart) {
      drawChart(ctx, d.chart, rect);
    } else if (d.kind === "image" && d.image) {
      const img = getOrLoadImage(d.image.dataUri);
      if (img) {
        ctx.drawImage(img, rect.x, rect.y, rect.w, rect.h);
      } else {
        ctx.fillStyle = "#f4f4f5";
        ctx.fillRect(rect.x, rect.y, rect.w, rect.h);
      }
    } else {
      ctx.fillStyle = "#f4f4f5";
      ctx.fillRect(rect.x, rect.y, rect.w, rect.h);
      ctx.strokeStyle = "#d4d4d8";
      ctx.strokeRect(rect.x + 0.5, rect.y + 0.5, rect.w - 1, rect.h - 1);
    }
  }
}

// src/numfmtDate.ts
var MONTHS_LONG = [
  "January",
  "February",
  "March",
  "April",
  "May",
  "June",
  "July",
  "August",
  "September",
  "October",
  "November",
  "December"
];
var MONTHS_SHORT = [
  "Jan",
  "Feb",
  "Mar",
  "Apr",
  "May",
  "Jun",
  "Jul",
  "Aug",
  "Sep",
  "Oct",
  "Nov",
  "Dec"
];
var DAYS_LONG = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
var DAYS_SHORT = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
function serialToParts(serial) {
  const totalSeconds = serial * 86400;
  const totalMs = Math.round(serial * 86400 * 1000);
  const date = new Date(Date.UTC(1899, 11, 30) + totalMs);
  const y = date.getUTCFullYear();
  const mo = date.getUTCMonth() + 1;
  const d = date.getUTCDate();
  const h = date.getUTCHours();
  const mi = date.getUTCMinutes();
  const s = date.getUTCSeconds();
  const ms = date.getUTCMilliseconds();
  const weekday = date.getUTCDay();
  return {
    y,
    mo,
    d,
    h,
    mi,
    s,
    ms,
    weekday,
    totalHours: Math.floor(totalSeconds / 3600),
    totalMinutes: Math.floor(totalSeconds / 60),
    totalSeconds: Math.floor(totalSeconds),
    isPM: h >= 12
  };
}
function pad2(n) {
  return n.toString().padStart(2, "0");
}
function renderDate(value, sec) {
  const p = serialToParts(value);
  const has12h = sec.tokens.some((t) => t.kind === "ampm");
  let s = "";
  for (let i = 0;i < sec.tokens.length; i++) {
    const t = sec.tokens[i];
    if (t.kind === "lit") {
      s += t.s;
      continue;
    }
    if (t.kind === "ampm") {
      const pm = p.isPM;
      if (t.abbreviated)
        s += pm ? t.upper ? "P" : "p" : t.upper ? "A" : "a";
      else
        s += pm ? t.upper ? "PM" : "pm" : t.upper ? "AM" : "am";
      continue;
    }
    if (t.kind === "elapsed") {
      const v = t.field === "h" ? p.totalHours : t.field === "m" ? p.totalMinutes : p.totalSeconds;
      s += v.toString().padStart(t.width, "0");
      continue;
    }
    if (t.kind === "date") {
      switch (t.field) {
        case "yyyy":
        case "yyy":
          s += p.y.toString().padStart(4, "0");
          break;
        case "yy":
        case "y":
          s += pad2(p.y % 100);
          break;
        case "mmmmm":
          s += (MONTHS_LONG[p.mo - 1] ?? "")[0] ?? "";
          break;
        case "mmmm":
          s += MONTHS_LONG[p.mo - 1] ?? "";
          break;
        case "mmm":
          s += MONTHS_SHORT[p.mo - 1] ?? "";
          break;
        case "mm":
        case "m": {
          const isMinutes = isMinuteContext(sec.tokens, i);
          if (isMinutes)
            s += t.field === "mm" ? pad2(p.mi) : p.mi.toString();
          else
            s += t.field === "mm" ? pad2(p.mo) : p.mo.toString();
          break;
        }
        case "dddd":
          s += DAYS_LONG[p.weekday] ?? "";
          break;
        case "ddd":
          s += DAYS_SHORT[p.weekday] ?? "";
          break;
        case "dd":
          s += pad2(p.d);
          break;
        case "d":
          s += p.d.toString();
          break;
        case "hh":
        case "h": {
          let hr = p.h;
          if (has12h) {
            hr = hr % 12;
            if (hr === 0)
              hr = 12;
          }
          s += t.field === "hh" ? pad2(hr) : hr.toString();
          break;
        }
        case "ss":
        case "s": {
          let sec1 = t.field === "ss" ? pad2(p.s) : p.s.toString();
          if (sec.tokens[i + 1]?.kind === "dot") {
            const digitToks = [];
            let j = i + 2;
            while (j < sec.tokens.length && sec.tokens[j].kind === "digit") {
              digitToks.push(sec.tokens[j]);
              j++;
            }
            if (digitToks.length > 0) {
              const f = (p.ms / 1000).toFixed(digitToks.length).slice(2);
              sec1 += "." + f;
              i = j - 1;
            }
          }
          s += sec1;
          break;
        }
      }
      continue;
    }
  }
  return s;
}
function isMinuteContext(tokens, idx) {
  for (let i = idx - 1;i >= 0; i--) {
    const t = tokens[i];
    if (t.kind === "date" && /^h{1,2}$/.test(t.field))
      return true;
    if (t.kind === "elapsed" && t.field === "h")
      return true;
    if (t.kind === "date" || t.kind === "elapsed")
      break;
  }
  for (let i = idx + 1;i < tokens.length; i++) {
    const t = tokens[i];
    if (t.kind === "date" && /^s{1,2}$/.test(t.field))
      return true;
    if (t.kind === "elapsed" && t.field === "s")
      return true;
    if (t.kind === "date" || t.kind === "elapsed")
      break;
  }
  return false;
}

// src/numfmtFraction.ts
function renderFraction(value, sec) {
  const sign = value < 0 ? "-" : "";
  const av = Math.abs(value);
  const intPart = sec.fractionIntPlaces > 0 ? Math.floor(av) : 0;
  const fracPart = sec.fractionIntPlaces > 0 ? av - intPart : av;
  let num = 0, den = 1;
  if (sec.fractionDenom > 0) {
    den = sec.fractionDenom;
    num = Math.round(fracPart * den);
    if (num === den) {
      if (sec.fractionIntPlaces > 0) {
        return formatFractionFinal(sign, intPart + 1, 0, den, sec);
      } else {
        return formatFractionFinal(sign, 0, den, den, sec);
      }
    }
  } else {
    const maxDen = Math.pow(10, Math.max(1, sec.fractionDenomQs)) - 1;
    [num, den] = bestFraction(fracPart, maxDen);
  }
  return formatFractionFinal(sign, intPart, num, den, sec);
}
function formatFractionFinal(sign, intPart, num, den, sec) {
  if (sec.fractionIntPlaces > 0) {
    const hideInt = intPart === 0 && sec.fractionHideZeroInt;
    if (num === 0)
      return hideInt ? sign + "0" : sign + String(intPart);
    if (hideInt)
      return sign + String(num) + "/" + String(den);
    return sign + String(intPart) + " " + String(num) + "/" + String(den);
  }
  return sign + String(num) + "/" + String(den);
}
function bestFraction(x, maxDen) {
  if (x === 0)
    return [0, 1];
  let lo = [0, 1];
  let hi = [1, 1];
  let best = [0, 1];
  let bestErr = Math.abs(x);
  for (let i = 0;i < 100; i++) {
    const mn = lo[0] + hi[0];
    const md = lo[1] + hi[1];
    if (md > maxDen)
      break;
    const m = mn / md;
    const err = Math.abs(x - m);
    if (err < bestErr) {
      best = [mn, md];
      bestErr = err;
    }
    if (m < x)
      lo = [mn, md];
    else if (m > x)
      hi = [mn, md];
    else
      return [mn, md];
  }
  return best;
}

// src/numfmtScientific.ts
function renderScientific(value, sec) {
  if (value === 0) {
    const mantissa = 0 .toFixed(sec.fracPlaces);
    const e = "0".padStart(sec.expDigits, "0");
    const sign2 = sec.expSign === "+" ? "+" : "";
    return mantissa + (sec.expUpper ? "E" : "e") + sign2 + e;
  }
  const sign = value < 0 ? "-" : "";
  const v = Math.abs(value);
  const rawExp = Math.floor(Math.log10(v));
  const exp = rawExp - (Math.max(1, sec.intPlaces) - 1);
  const mant = v / Math.pow(10, exp);
  const mantStr = mant.toFixed(sec.fracPlaces);
  const expStr = Math.abs(exp).toString().padStart(sec.expDigits, "0");
  const expSign = exp < 0 ? "-" : sec.expSign === "+" ? "+" : "";
  return sign + mantStr + (sec.expUpper ? "E" : "e") + expSign + expStr;
}

// src/numfmtNumberParts.ts
function renderIntegerTokens(tokens, intDigits, grouping) {
  let digits = intDigits.replace(/^0+(?=\d)/, "");
  if (digits === "")
    digits = "0";
  if (grouping)
    digits = digits.replace(/\B(?=(\d{3})+(?!\d))/g, ",");
  const placeholders = [];
  for (let i = 0;i < tokens.length; i++)
    if (tokens[i].kind === "digit")
      placeholders.push(i);
  const firstDigit = placeholders[0] ?? -1;
  const lastDigit = placeholders[placeholders.length - 1] ?? -1;
  const isGroupingMarker = (idx, t) => {
    if (!grouping || t.kind !== "lit" || t.s !== ",")
      return false;
    return idx > firstDigit && idx < lastDigit;
  };
  const out = new Array(tokens.length);
  let di = digits.length - 1;
  for (let pi = placeholders.length - 1;pi >= 0; pi--) {
    const tIdx = placeholders[pi];
    const t = tokens[tIdx];
    if (pi === 0) {
      const rest = di >= 0 ? digits.slice(0, di + 1) : "";
      if (rest)
        out[tIdx] = rest;
      else if (t.ch === "0")
        out[tIdx] = "0";
      else if (t.ch === "?")
        out[tIdx] = " ";
      else
        out[tIdx] = "";
      di = -1;
    } else {
      if (di >= 0) {
        let ch = digits[di];
        di--;
        while (ch === "," && di >= 0) {
          ch = digits[di];
          di--;
        }
        if (ch === ",")
          ch = "";
        out[tIdx] = ch;
      } else {
        out[tIdx] = t.ch === "0" ? "0" : t.ch === "?" ? " " : "";
      }
    }
  }
  let s = "";
  for (let i = 0;i < tokens.length; i++) {
    const t = tokens[i];
    if (t.kind === "digit")
      s += out[i] ?? "";
    else if (t.kind === "lit") {
      if (isGroupingMarker(i, t))
        continue;
      s += t.s;
    } else if (t.kind === "percent")
      s += "%";
    else if (t.kind === "fill")
      s += FILL_SENTINEL;
  }
  return s;
}
function renderFractionalTokens(tokens, fracDigits) {
  let s = "";
  let di = 0;
  for (const t of tokens) {
    if (t.kind === "digit") {
      if (di < fracDigits.length) {
        s += fracDigits[di];
        di++;
      } else if (t.ch === "0")
        s += "0";
      else if (t.ch === "?")
        s += " ";
    } else if (t.kind === "lit") {
      s += t.s;
    } else if (t.kind === "percent") {
      s += "%";
    } else if (t.kind === "fill") {
      s += FILL_SENTINEL;
    }
  }
  return s;
}

// src/numfmt.ts
var FILL_SENTINEL = "\x01";
var FORMAT_CACHE = new Map;
function formatValue(value, fmt) {
  const f = (fmt ?? "").trim();
  if (!f || f.toLowerCase() === "general")
    return { text: formatGeneral2(value) };
  let sections;
  try {
    sections = FORMAT_CACHE.get(f) ?? parseFormat(f);
    FORMAT_CACHE.set(f, sections);
  } catch {
    return { text: formatGeneral2(value) };
  }
  const sec = pickSection(sections, value);
  if (!sec)
    return { text: formatGeneral2(value) };
  try {
    const text = renderSection(value, sec);
    const fills = sec.tokens.flatMap((t) => t.kind === "fill" ? [t.ch] : []);
    const out = { text, color: sec.color };
    if (fills.length > 0)
      out.fills = fills;
    return out;
  } catch {
    return { text: formatGeneral2(value) };
  }
}
function formatGeneral2(v) {
  if (!isFinite(v))
    return String(v);
  if (Number.isInteger(v) && Math.abs(v) < 1000000000000000)
    return v.toString();
  return parseFloat(v.toPrecision(11)).toString();
}
function parseFormat(fmt) {
  const rawSections = splitTopLevel(fmt, ";");
  return rawSections.map(parseSection);
}
function splitTopLevel(s, sep) {
  const out = [];
  let cur = "";
  let i = 0;
  while (i < s.length) {
    const c = s[i];
    if (c === '"') {
      cur += c;
      i++;
      while (i < s.length && s[i] !== '"') {
        cur += s[i];
        i++;
      }
      if (i < s.length) {
        cur += s[i];
        i++;
      }
      continue;
    }
    if (c === "[") {
      cur += c;
      i++;
      while (i < s.length && s[i] !== "]") {
        cur += s[i];
        i++;
      }
      if (i < s.length) {
        cur += s[i];
        i++;
      }
      continue;
    }
    if (c === "\\") {
      cur += c;
      if (i + 1 < s.length) {
        cur += s[i + 1];
        i += 2;
      } else
        i++;
      continue;
    }
    if (c === sep) {
      out.push(cur);
      cur = "";
      i++;
      continue;
    }
    cur += c;
    i++;
  }
  out.push(cur);
  return out;
}
var COLOR_NAMES = {
  black: "#000000",
  white: "#ffffff",
  red: "#ff0000",
  green: "#008000",
  blue: "#0000ff",
  yellow: "#ffff00",
  magenta: "#ff00ff",
  cyan: "#00ffff"
};
var COLOR_BY_INDEX = {
  1: "#000000",
  2: "#ffffff",
  3: "#ff0000",
  4: "#00ff00",
  5: "#0000ff",
  6: "#ffff00",
  7: "#ff00ff",
  8: "#00ffff",
  9: "#800000",
  10: "#008000",
  11: "#000080",
  12: "#808000",
  13: "#800080",
  14: "#008080",
  15: "#c0c0c0",
  16: "#808080",
  17: "#9999ff",
  18: "#993366",
  19: "#ffffcc",
  20: "#ccffff",
  21: "#660066",
  22: "#ff8080",
  23: "#0066cc",
  24: "#ccccff",
  25: "#000080",
  26: "#ff00ff",
  27: "#ffff00",
  28: "#00ffff",
  29: "#800080",
  30: "#800000",
  31: "#008080",
  32: "#0000ff",
  33: "#00ccff",
  34: "#ccffff",
  35: "#ccffcc",
  36: "#ffff99",
  37: "#99ccff",
  38: "#ff99cc",
  39: "#cc99ff",
  40: "#ffcc99",
  41: "#3366ff",
  42: "#33cccc",
  43: "#99cc00",
  44: "#ffcc00",
  45: "#ff9900",
  46: "#ff6600",
  47: "#666699",
  48: "#969696",
  49: "#003366",
  50: "#339966",
  51: "#003300",
  52: "#333300",
  53: "#993300",
  54: "#993366",
  55: "#333399",
  56: "#333333"
};
function parseSection(raw) {
  let s = raw;
  let color;
  let condition;
  while (true) {
    const m = /^\[([^\]]+)\]/.exec(s);
    if (!m)
      break;
    const inner = m[1];
    const lower = inner.toLowerCase();
    if (COLOR_NAMES[lower] !== undefined) {
      color = COLOR_NAMES[lower];
      s = s.slice(m[0].length);
      continue;
    }
    const cm = /^color(\d{1,2})$/i.exec(inner);
    if (cm) {
      const n = parseInt(cm[1], 10);
      color = COLOR_BY_INDEX[n] ?? "#000000";
      s = s.slice(m[0].length);
      continue;
    }
    const cond = /^(<=|>=|<>|=|<|>)\s*(-?\d+(?:\.\d+)?)$/.exec(inner);
    if (cond) {
      condition = {
        op: cond[1],
        value: parseFloat(cond[2])
      };
      s = s.slice(m[0].length);
      continue;
    }
    break;
  }
  const tokens = tokenize(s);
  let flavor = "literal";
  let intPlaces = 0, fracPlaces = 0;
  let hasGrouping = false;
  let scale = 1;
  let fractionDenom = 0, fractionDenomQs = 0, fractionIntPlaces = 0;
  let fractionHideZeroInt = false;
  let expSign = "";
  let expDigits = 0;
  let expUpper = true;
  const dotIdx = tokens.findIndex((t) => t.kind === "dot");
  const slashIdx = findFractionSlash(tokens);
  const expIdx = tokens.findIndex((t) => t.kind === "exp");
  const hasDate = tokens.some((t) => t.kind === "date" || t.kind === "elapsed" || t.kind === "ampm");
  const hasText = tokens.some((t) => t.kind === "text");
  const hasDigit = tokens.some((t) => t.kind === "digit");
  if (hasDate)
    flavor = "date";
  else if (slashIdx >= 0 && hasDigit)
    flavor = "fraction";
  else if (expIdx >= 0 && hasDigit)
    flavor = "scientific";
  else if (hasDigit)
    flavor = "number";
  else if (hasText)
    flavor = "text";
  else
    flavor = "literal";
  if (flavor === "number") {
    const before = dotIdx < 0 ? tokens : tokens.slice(0, dotIdx);
    const after = dotIdx < 0 ? [] : tokens.slice(dotIdx + 1);
    intPlaces = before.filter((t) => t.kind === "digit").length;
    fracPlaces = after.filter((t) => t.kind === "digit").length;
    hasGrouping = hasGroupingComma(before);
    const lastDigitIdx = lastIndexWhere(tokens, (t) => t.kind === "digit");
    if (lastDigitIdx >= 0) {
      let commaScale = 0;
      for (let i = lastDigitIdx + 1;i < tokens.length; i++) {
        const t = tokens[i];
        if (t.kind === "lit") {
          let stripped = 0;
          while (stripped < t.s.length && t.s[stripped] === ",")
            stripped++;
          if (stripped > 0) {
            commaScale += stripped;
            t.s = t.s.slice(stripped);
          }
          if (t.s.length > 0)
            break;
        } else if (t.kind === "percent" || t.kind === "dot")
          continue;
        else
          break;
      }
      scale *= Math.pow(0.001, commaScale);
    }
    if (tokens.some((t) => t.kind === "percent"))
      scale *= 100;
  } else if (flavor === "fraction") {
    const slashTok = tokens[slashIdx];
    const slashPos = slashTok.s.indexOf("/");
    const beforeSlashStr = slashTok.s.slice(0, slashPos);
    const afterSlashStr = slashTok.s.slice(slashPos + 1);
    const before = tokens.slice(0, slashIdx);
    if (beforeSlashStr)
      before.push({ kind: "lit", s: beforeSlashStr });
    const after = [];
    if (afterSlashStr)
      after.push({ kind: "lit", s: afterSlashStr });
    after.push(...tokens.slice(slashIdx + 1));
    let lastSpaceIdx = -1;
    for (let i = 0;i < before.length; i++) {
      const t = before[i];
      if (t.kind === "lit" && /\s/.test(t.s))
        lastSpaceIdx = i;
    }
    if (lastSpaceIdx >= 0) {
      fractionIntPlaces = before.slice(0, lastSpaceIdx).filter((t) => t.kind === "digit").length;
      const intPHs = before.slice(0, lastSpaceIdx).filter((t) => t.kind === "digit");
      fractionHideZeroInt = intPHs.length > 0 && intPHs.every((t) => t.ch === "#");
    }
    let fixedNum = "";
    let qCount = 0;
    for (const t of after) {
      if (t.kind === "digit") {
        if (t.ch === "?")
          qCount++;
      } else if (t.kind === "lit") {
        const m = /^([0-9]+)/.exec(t.s);
        if (m)
          fixedNum += m[1];
      }
    }
    if (fixedNum)
      fractionDenom = parseInt(fixedNum, 10);
    fractionDenomQs = qCount;
  } else if (flavor === "scientific") {
    const expTok = tokens[expIdx];
    expSign = expTok.sign;
    expUpper = expTok.upper;
    const before = dotIdx < 0 || dotIdx > expIdx ? tokens.slice(0, expIdx) : tokens.slice(0, dotIdx);
    const after = dotIdx >= 0 && dotIdx < expIdx ? tokens.slice(dotIdx + 1, expIdx) : [];
    intPlaces = before.filter((t) => t.kind === "digit").length;
    fracPlaces = after.filter((t) => t.kind === "digit").length;
    expDigits = tokens.slice(expIdx + 1).filter((t) => t.kind === "digit").length;
  }
  return {
    tokens,
    color,
    condition,
    flavor,
    intPlaces,
    fracPlaces,
    hasGrouping,
    scale,
    fractionDenom,
    fractionDenomQs,
    fractionIntPlaces,
    fractionHideZeroInt,
    expSign,
    expDigits,
    expUpper
  };
}
function lastIndexWhere(arr, pred) {
  for (let i = arr.length - 1;i >= 0; i--)
    if (pred(arr[i]))
      return i;
  return -1;
}
function hasGroupingComma(toks) {
  let firstDigit = -1, lastDigit = -1;
  for (let i = 0;i < toks.length; i++) {
    if (toks[i].kind === "digit") {
      if (firstDigit < 0)
        firstDigit = i;
      lastDigit = i;
    }
  }
  if (firstDigit < 0 || firstDigit === lastDigit)
    return false;
  for (let i = firstDigit + 1;i < lastDigit; i++) {
    const t = toks[i];
    if (t.kind === "lit" && t.s.includes(","))
      return true;
  }
  return false;
}
function findFractionSlash(toks) {
  for (let i = 0;i < toks.length; i++) {
    const t = toks[i];
    if (t.kind !== "lit" || !t.s.includes("/"))
      continue;
    let leftOk = false;
    for (let j = i - 1;j >= 0; j--) {
      const u = toks[j];
      if (u.kind === "digit") {
        leftOk = true;
        break;
      }
      if (u.kind === "lit")
        continue;
      break;
    }
    if (!leftOk)
      continue;
    let rightOk = false;
    const afterSlash = t.s.slice(t.s.indexOf("/") + 1);
    if (/^[0-9]/.test(afterSlash))
      rightOk = true;
    if (!rightOk) {
      for (let j = i + 1;j < toks.length; j++) {
        const u = toks[j];
        if (u.kind === "digit") {
          rightOk = true;
          break;
        }
        if (u.kind === "lit") {
          if (/^[0-9]/.test(u.s)) {
            rightOk = true;
            break;
          }
          continue;
        }
        break;
      }
    }
    if (rightOk)
      return i;
  }
  return -1;
}
function tokenize(s) {
  const out = [];
  let i = 0;
  while (i < s.length) {
    const c = s[i];
    if (c === '"') {
      let lit = "";
      i++;
      while (i < s.length && s[i] !== '"') {
        lit += s[i];
        i++;
      }
      if (i < s.length)
        i++;
      if (lit)
        out.push({ kind: "lit", s: lit });
      continue;
    }
    if (c === "\\") {
      if (i + 1 < s.length) {
        out.push({ kind: "lit", s: s[i + 1] });
        i += 2;
      } else
        i++;
      continue;
    }
    if (c === "_") {
      i += i + 1 < s.length ? 2 : 1;
      out.push({ kind: "lit", s: " " });
      continue;
    }
    if (c === "*") {
      const ch = i + 1 < s.length ? s[i + 1] : " ";
      i += i + 1 < s.length ? 2 : 1;
      out.push({ kind: "fill", ch });
      continue;
    }
    if (c === "[") {
      let inner = "";
      i++;
      while (i < s.length && s[i] !== "]") {
        inner += s[i];
        i++;
      }
      if (i < s.length)
        i++;
      if (inner.startsWith("$")) {
        const sym = inner.slice(1).split("-")[0];
        if (sym)
          out.push({ kind: "lit", s: sym });
        continue;
      }
      const em = /^([hms])\1*$/i.exec(inner);
      if (em) {
        const field = inner[0].toLowerCase();
        out.push({ kind: "elapsed", field, width: inner.length });
        continue;
      }
      continue;
    }
    const dm = /^(yyyy|yyy|yy|y|mmmmm|mmmm|mmm|mm|m|dddd|ddd|dd|d|hh|h|ss|s)/i.exec(s.slice(i));
    if (dm) {
      const tok = dm[0].toLowerCase();
      out.push({ kind: "date", field: tok });
      i += tok.length;
      continue;
    }
    if (/^am\/pm/i.test(s.slice(i))) {
      out.push({ kind: "ampm", upper: s[i] === "A", abbreviated: false });
      i += 5;
      continue;
    }
    if (/^a\/p/i.test(s.slice(i))) {
      out.push({ kind: "ampm", upper: s[i] === "A", abbreviated: true });
      i += 3;
      continue;
    }
    if (c === "0" || c === "#" || c === "?") {
      out.push({ kind: "digit", ch: c });
      i++;
      continue;
    }
    if (c === ".") {
      out.push({ kind: "dot" });
      i++;
      continue;
    }
    if (c === "%") {
      out.push({ kind: "percent" });
      i++;
      continue;
    }
    if (c === "," || c === "/") {
      out.push({ kind: "lit", s: c });
      i++;
      continue;
    }
    if ((c === "E" || c === "e") && i + 1 < s.length) {
      const next = s[i + 1];
      if (next === "+" || next === "-") {
        out.push({ kind: "exp", sign: next, upper: c === "E" });
        i += 2;
        continue;
      }
    }
    if (c === "@") {
      out.push({ kind: "text" });
      i++;
      continue;
    }
    out.push({ kind: "lit", s: c });
    i++;
  }
  const merged = [];
  for (const t of out) {
    const prev = merged[merged.length - 1];
    if (t.kind === "lit" && prev && prev.kind === "lit")
      prev.s += t.s;
    else
      merged.push(t);
  }
  return merged;
}
function pickSection(sections, value) {
  if (sections.length === 0)
    return;
  const hasExplicitConds = sections.some((s) => s.condition);
  if (hasExplicitConds) {
    for (let i = 0;i < Math.min(2, sections.length); i++) {
      const s = sections[i];
      if (!s.condition)
        continue;
      if (matchesCond(value, s.condition))
        return s;
    }
    return sections[2] ?? sections[sections.length - 1];
  }
  if (sections.length === 1)
    return sections[0];
  if (value > 0)
    return sections[0];
  if (value < 0)
    return sections[1] ?? sections[0];
  return sections[2] ?? sections[0];
}
function matchesCond(v, c) {
  switch (c.op) {
    case ">":
      return v > c.value;
    case "<":
      return v < c.value;
    case ">=":
      return v >= c.value;
    case "<=":
      return v <= c.value;
    case "=":
      return v === c.value;
    case "<>":
      return v !== c.value;
  }
}
function renderSection(value, sec) {
  const litOrFill = (t) => t.kind === "lit" ? t.s : t.kind === "fill" ? FILL_SENTINEL : "";
  switch (sec.flavor) {
    case "literal":
      return sec.tokens.map(litOrFill).join("");
    case "text":
      return sec.tokens.map(litOrFill).join("");
    case "number":
      return renderNumber(value, sec);
    case "date":
      return renderDate(value, sec);
    case "fraction":
      return renderFraction(value, sec);
    case "scientific":
      return renderScientific(value, sec);
  }
}
function renderNumber(value, sec) {
  const sign = value < 0 ? "-" : "";
  const v = value * sec.scale;
  const absStr = Math.abs(v).toFixed(sec.fracPlaces);
  const dotPos = absStr.indexOf(".");
  const intDigits = dotPos < 0 ? absStr : absStr.slice(0, dotPos);
  const fracDigits = dotPos < 0 ? "" : absStr.slice(dotPos + 1);
  const dotIdx = sec.tokens.findIndex((t) => t.kind === "dot");
  const beforeDot = dotIdx < 0 ? sec.tokens : sec.tokens.slice(0, dotIdx);
  const afterDot = dotIdx < 0 ? [] : sec.tokens.slice(dotIdx + 1);
  const intRendered = renderIntegerTokens(beforeDot, intDigits, sec.hasGrouping);
  const fracRendered = renderFractionalTokens(afterDot, fracDigits);
  sec.intPlaces;
  const sectionEncodesNeg = sec.tokens.some((t) => t.kind === "lit" && (t.s.includes("(") || t.s.includes("-")));
  const finalSign = sectionEncodesNeg ? "" : sign;
  let out = "";
  if (dotIdx < 0)
    out = intRendered;
  else
    out = intRendered + "." + fracRendered;
  return finalSign + out;
}

// src/cellText.ts
var BUILTIN_NUMFMT = {
  0: "General",
  1: "0",
  2: "0.00",
  3: "#,##0",
  4: "#,##0.00",
  5: "$#,##0_);($#,##0)",
  6: "$#,##0_);[Red]($#,##0)",
  7: "$#,##0.00_);($#,##0.00)",
  8: "$#,##0.00_);[Red]($#,##0.00)",
  9: "0%",
  10: "0.00%",
  11: "0.00E+00",
  12: "# ?/?",
  13: "# ??/??",
  14: "m/d/yyyy",
  15: "d-mmm-yy",
  16: "d-mmm",
  17: "mmm-yy",
  18: "h:mm AM/PM",
  19: "h:mm:ss AM/PM",
  20: "h:mm",
  21: "h:mm:ss",
  22: "m/d/yyyy h:mm",
  37: "#,##0;(#,##0)",
  38: "#,##0;[Red](#,##0)",
  39: "#,##0.00;(#,##0.00)",
  40: "#,##0.00;[Red](#,##0.00)",
  41: '_(* #,##0_);_(* (#,##0);_(* "-"_);_(@_)',
  42: '_("$"* #,##0_);_("$"* (#,##0);_("$"* "-"_);_(@_)',
  43: '_(* #,##0.00_);_(* (#,##0.00);_(* "-"??_);_(@_)',
  44: '_("$"* #,##0.00_);_("$"* (#,##0.00);_("$"* "-"??_);_(@_)',
  45: "mm:ss",
  46: "[h]:mm:ss",
  47: "mm:ss.0",
  48: "##0.0E+0",
  49: "@"
};
var NUMFMT_CODE_CACHE = new WeakMap;
var COL_STYLE_CACHE = new WeakMap;
function colStyleMap(sheet) {
  let m = COL_STYLE_CACHE.get(sheet);
  if (m)
    return m;
  m = new Map;
  for (const col of sheet.cols) {
    if (col.styleIndex === undefined)
      continue;
    for (let i = col.min - 1;i <= col.max - 1; i++)
      m.set(i, col.styleIndex);
  }
  COL_STYLE_CACHE.set(sheet, m);
  return m;
}
function resolveCellXf(cell, sheet, layout) {
  const xfs = layout.styles.cellXfs;
  if (cell.styleIndex !== undefined)
    return xfs[cell.styleIndex];
  const meta = sheet.decodedRowMeta;
  const rowSlot = meta.byIndex.get(cell.r);
  if (rowSlot !== undefined) {
    const sIdx = meta.styleIdx[rowSlot] ?? -1;
    if (sIdx >= 0)
      return xfs[sIdx];
  }
  const colXf = colStyleMap(sheet).get(cell.c);
  if (colXf !== undefined)
    return xfs[colXf];
  return xfs[0];
}
function numFmtCode(layout, id) {
  let cache = NUMFMT_CODE_CACHE.get(layout);
  if (!cache) {
    cache = new Map;
    for (const nf of layout.styles.numFmts)
      cache.set(nf.id, nf.formatCode);
    NUMFMT_CODE_CACHE.set(layout, cache);
  }
  return cache.get(id) ?? BUILTIN_NUMFMT[id];
}
function resolveCellText(cell, layout, xf) {
  const v = cell.value ?? "";
  switch (cell.type) {
    case "s": {
      const idx = parseInt(v, 10);
      const s = layout.sharedStrings[idx] ?? "";
      return { text: s, defaultAlign: "left" };
    }
    case "inline":
    case "str":
      return { text: v, defaultAlign: "left" };
    case "b":
      return { text: v === "1" ? "TRUE" : "FALSE", defaultAlign: "center" };
    case "e":
      return { text: v, defaultAlign: "center" };
    case "f":
    case "n": {
      if (!v)
        return { text: "", defaultAlign: "right" };
      const n = Number(v);
      if (Number.isNaN(n))
        return { text: v, defaultAlign: "left" };
      const numFmtId = xf?.numFmtId;
      let code;
      if (numFmtId !== undefined) {
        code = numFmtCode(layout, numFmtId);
      }
      const r = formatValue(n, code);
      return {
        text: r.text,
        defaultAlign: "right",
        formatColor: r.color,
        fills: r.fills
      };
    }
    default:
      return { text: v, defaultAlign: "left" };
  }
}
function cellTextValue(cell, layout) {
  if (cell.value === undefined)
    return "";
  switch (cell.type) {
    case "s": {
      const idx = parseInt(cell.value, 10);
      return layout.sharedStrings[idx] ?? "";
    }
    case "inline":
    case "str":
      return cell.value;
    default:
      return cell.value;
  }
}
function cellNumericValue(cell) {
  if (cell.value === undefined)
    return null;
  if (cell.type === "n" || cell.type === "f" || cell.type === "b") {
    const n = parseFloat(cell.value);
    return Number.isNaN(n) ? null : n;
  }
  return null;
}

// src/canvasFactory.ts
var createOffscreenCanvas = null;
function makeOffscreenCanvas(width, height) {
  if (createOffscreenCanvas)
    return createOffscreenCanvas(width, height);
  if (typeof document !== "undefined") {
    const canvas = document.createElement("canvas");
    canvas.width = width;
    canvas.height = height;
    return canvas;
  }
  throw new Error("no offscreen canvas factory configured");
}

// src/cellPaint.ts
var PATTERN_TILES_8X8 = {
  gray125: [136, 0, 34, 0, 136, 0, 34, 0],
  gray0625: [136, 0, 0, 0, 34, 0, 0, 0],
  lightGray: [136, 34, 136, 34, 136, 34, 136, 34],
  mediumGray: [170, 85, 170, 85, 170, 85, 170, 85],
  darkGray: [119, 221, 119, 221, 119, 221, 119, 221],
  lightHorizontal: [255, 0, 0, 0, 255, 0, 0, 0],
  darkHorizontal: [255, 255, 0, 0, 255, 255, 0, 0],
  lightVertical: [17, 17, 17, 17, 17, 17, 17, 17],
  darkVertical: [51, 51, 51, 51, 51, 51, 51, 51],
  lightDown: [1, 2, 4, 8, 16, 32, 64, 128],
  darkDown: [3, 6, 12, 24, 48, 96, 192, 129],
  lightUp: [128, 64, 32, 16, 8, 4, 2, 1],
  darkUp: [192, 96, 48, 24, 12, 6, 3, 129],
  lightGrid: [255, 17, 17, 17, 255, 17, 17, 17],
  darkGrid: [255, 255, 51, 51, 255, 255, 51, 51],
  lightTrellis: [129, 66, 36, 24, 24, 36, 66, 129],
  darkTrellis: [195, 102, 60, 24, 24, 60, 102, 195]
};
var patternCache = new Map;
function buildPattern(ctx, patternType, fgCss, bgCss) {
  const key = `${patternType}|${fgCss}|${bgCss ?? ""}`;
  const hit = patternCache.get(key);
  if (hit !== undefined)
    return hit;
  const tile = PATTERN_TILES_8X8[patternType];
  if (!tile) {
    patternCache.set(key, null);
    return null;
  }
  const off = makeOffscreenCanvas(8, 8);
  const octx = off.getContext("2d");
  if (bgCss) {
    octx.fillStyle = bgCss;
    octx.fillRect(0, 0, 8, 8);
  }
  octx.fillStyle = fgCss;
  for (let y = 0;y < 8; y++) {
    const row = tile[y] ?? 0;
    for (let x = 0;x < 8; x++) {
      if (row & 1 << x)
        octx.fillRect(x, y, 1, 1);
    }
  }
  const pat = ctx.createPattern(off, "repeat");
  patternCache.set(key, pat);
  return pat;
}
function collectStops(fill) {
  const stops = (fill.gradientStops ?? []).map((s) => ({
    pos: Math.max(0, Math.min(1, s.position ?? 0)),
    css: colorToCss(s.color, "#ffffff")
  }));
  if (stops.length >= 2)
    return stops;
  const c1 = fill.fgColor ? colorToCss(fill.fgColor, "#ffffff") : null;
  const c2 = fill.bgColor ? colorToCss(fill.bgColor, "#ffffff") : c1;
  if (!c1 || !c2)
    return [];
  if (stops.length === 1) {
    const s = stops[0];
    return s.pos < 0.5 ? [s, { pos: 1, css: c2 }] : [{ pos: 0, css: c1 }, s];
  }
  return [
    { pos: 0, css: c1 },
    { pos: 1, css: c2 }
  ];
}
function paintGradientFill(ctx, rect, fill) {
  const stops = collectStops(fill);
  if (stops.length === 0)
    return;
  const type = fill.gradientType ?? "linear";
  if (type === "path") {
    const li = Math.max(0, Math.min(1, fill.gradientLeft ?? 0));
    const ri = Math.max(0, Math.min(1, fill.gradientRight ?? 0));
    const ti = Math.max(0, Math.min(1, fill.gradientTop ?? 0));
    const bi = Math.max(0, Math.min(1, fill.gradientBottom ?? 0));
    const ix = rect.x + li * rect.w;
    const iy = rect.y + ti * rect.h;
    const iw = Math.max(0, rect.w * Math.max(0, 1 - li - ri));
    const ih = Math.max(0, rect.h * Math.max(0, 1 - ti - bi));
    ctx.fillStyle = stops[0].css;
    ctx.fillRect(rect.x, rect.y, rect.w, rect.h);
    const cx = ix + iw / 2;
    const cy = iy + ih / 2;
    const r0 = Math.hypot(iw, ih) / 2;
    const corners = [
      [rect.x, rect.y],
      [rect.x + rect.w, rect.y],
      [rect.x, rect.y + rect.h],
      [rect.x + rect.w, rect.y + rect.h]
    ];
    const r1 = Math.max(...corners.map(([x, y]) => Math.hypot(x - cx, y - cy)));
    if (r1 <= r0 + 0.5)
      return;
    const grad2 = ctx.createRadialGradient(cx, cy, r0, cx, cy, r1);
    for (const s of stops)
      grad2.addColorStop(s.pos, s.css);
    ctx.fillStyle = grad2;
    ctx.fillRect(rect.x, rect.y, rect.w, rect.h);
    return;
  }
  const deg = fill.gradientDegree ?? 0;
  const theta = deg * Math.PI / 180;
  const dx = Math.cos(theta);
  const dy = Math.sin(theta);
  const projs = [0, rect.w * dx, rect.h * dy, rect.w * dx + rect.h * dy];
  const pmin = Math.min(...projs);
  const pmax = Math.max(...projs);
  const x0 = rect.x + pmin * dx;
  const y0 = rect.y + pmin * dy;
  const x1 = rect.x + pmax * dx;
  const y1 = rect.y + pmax * dy;
  const grad = Math.hypot(x1 - x0, y1 - y0) < 0.5 ? null : ctx.createLinearGradient(x0, y0, x1, y1);
  if (grad) {
    for (const s of stops)
      grad.addColorStop(s.pos, s.css);
    ctx.fillStyle = grad;
  } else {
    ctx.fillStyle = stops[0].css;
  }
  ctx.fillRect(rect.x, rect.y, rect.w, rect.h);
}
function paintFill(ctx, rect, fill) {
  const pt = fill.patternType;
  if (!pt || pt === "none")
    return;
  if (pt === "solid" && fill.fgColor) {
    ctx.fillStyle = colorToCss(fill.fgColor, "#ffffff");
    ctx.fillRect(rect.x, rect.y, rect.w, rect.h);
    return;
  }
  if (pt === "gradient") {
    paintGradientFill(ctx, rect, fill);
    return;
  }
  if (PATTERN_TILES_8X8[pt]) {
    const fgCss = colorToCss(fill.fgColor, "#000000");
    const bgCss = fill.bgColor ? colorToCss(fill.bgColor, "#ffffff") : null;
    if (bgCss) {
      ctx.fillStyle = bgCss;
      ctx.fillRect(rect.x, rect.y, rect.w, rect.h);
    }
    const pat = buildPattern(ctx, pt, fgCss, bgCss);
    if (!pat)
      return;
    ctx.save();
    ctx.translate(rect.x, rect.y);
    ctx.fillStyle = pat;
    ctx.fillRect(0, 0, rect.w, rect.h);
    ctx.restore();
  }
}
var COL_STYLE_1BASED = new WeakMap;
function colStyleMap1Based(sheet) {
  let m = COL_STYLE_1BASED.get(sheet);
  if (m)
    return m;
  m = new Map;
  for (const col of sheet.cols) {
    if (col.styleIndex === undefined)
      continue;
    for (let i = col.min;i <= col.max; i++)
      m.set(i, col.styleIndex);
  }
  COL_STYLE_1BASED.set(sheet, m);
  return m;
}
function drawDefaultFills(ctx, sheet, layout, g, vis) {
  const styles = layout.styles;
  const xfs = styles.cellXfs;
  const fillFor = (xfId) => {
    const xf = xfs[xfId];
    if (!xf)
      return;
    return xf.fillId !== undefined ? styles.fills[xf.fillId] : undefined;
  };
  const colMap = colStyleMap1Based(sheet);
  if (colMap.size > 0) {
    const colFirst = Math.max(1, vis.firstCol);
    const colLast = Math.min(sheet.maxCol, vis.lastCol);
    const rowFirst = Math.max(1, vis.firstRow);
    const rowLast = Math.min(sheet.maxRow, vis.lastRow);
    if (colFirst <= colLast && rowFirst <= rowLast) {
      const yTop = g.rowY[rowFirst] ?? 0;
      const yBot = g.rowY[rowLast + 1] ?? yTop;
      const h = yBot - yTop;
      if (h > 0) {
        for (let c = colFirst;c <= colLast; c++) {
          const xfId = colMap.get(c);
          if (xfId === undefined)
            continue;
          const fill = fillFor(xfId);
          if (!fill)
            continue;
          const x = g.colX[c] ?? 0;
          const w = (g.colX[c + 1] ?? x) - x;
          if (w <= 0)
            continue;
          paintFill(ctx, { x, y: yTop, w, h }, fill);
        }
      }
    }
  }
  const meta = sheet.decodedRowMeta;
  if (meta.count > 0 && sheet.maxCol >= 1) {
    const colFirst = Math.max(1, vis.firstCol);
    const colLast = Math.min(sheet.maxCol, vis.lastCol);
    if (colFirst <= colLast) {
      const xLeft = g.colX[colFirst] ?? 0;
      const xRight = g.colX[colLast + 1] ?? xLeft;
      const w = xRight - xLeft;
      if (w > 0) {
        for (let i = 0;i < meta.count; i++) {
          const r = meta.index[i] ?? 0;
          if (r < vis.firstRow || r > vis.lastRow)
            continue;
          const sIdx = meta.styleIdx[i] ?? -1;
          if (sIdx < 0)
            continue;
          const fill = fillFor(sIdx);
          if (!fill)
            continue;
          const y = g.rowY[r] ?? 0;
          const h = (g.rowY[r + 1] ?? y) - y;
          if (h <= 0)
            continue;
          paintFill(ctx, { x: xLeft, y, w, h }, fill);
        }
      }
    }
  }
}
function drawCellBackgrounds(ctx, sheet, layout, g, vis) {
  const styles = layout.styles;
  const { covered, topLeftOf } = buildMergeMaps(sheet);
  for (const m of sheet.merges) {
    if (m.r2 < vis.firstRow || m.r1 > vis.lastRow)
      continue;
    if (m.c2 < vis.firstCol || m.c1 > vis.lastCol)
      continue;
    const tl = findCell(sheet, m.r1, m.c1);
    if (!tl)
      continue;
    const xf = resolveCellXf(tl, sheet, layout);
    if (!xf)
      continue;
    const fill = xf.fillId !== undefined ? styles.fills[xf.fillId] : undefined;
    if (!fill)
      continue;
    paintFill(ctx, mergedRect(g, m), fill);
  }
  iterCellsInRange(sheet, vis.firstRow, vis.lastRow, vis.firstCol, vis.lastCol, (cell) => {
    const k = `${cell.r}:${cell.c}`;
    if (covered.has(k))
      return;
    if (topLeftOf.has(k))
      return;
    const xf = resolveCellXf(cell, sheet, layout);
    if (!xf)
      return;
    const fill = xf.fillId !== undefined ? styles.fills[xf.fillId] : undefined;
    if (!fill)
      return;
    paintFill(ctx, cellRect(g, cell.r, cell.c), fill);
  });
}
function borderWidth(line) {
  switch (line.style) {
    case "thin":
    case "hair":
    case "dotted":
    case "dashed":
    case "dashDot":
    case "dashDotDot":
      return 1;
    case "medium":
    case "mediumDashed":
    case "mediumDashDot":
    case "mediumDashDotDot":
    case "slantDashDot":
      return 2;
    case "thick":
      return 3;
    case "double":
      return 1;
    default:
      return 1;
  }
}
function borderDash(style) {
  switch (style) {
    case "dotted":
      return [1, 1];
    case "hair":
      return [1, 2];
    case "dashed":
    case "mediumDashed":
      return [3, 2];
    case "dashDot":
    case "mediumDashDot":
    case "slantDashDot":
      return [3, 1, 1, 1];
    case "dashDotDot":
    case "mediumDashDotDot":
      return [3, 1, 1, 1, 1, 1];
    default:
      return null;
  }
}
function drawBorderLine(ctx, x1, y1, x2, y2, line) {
  const w = borderWidth(line);
  ctx.strokeStyle = colorToCss(line.color, "#000000");
  ctx.lineWidth = w;
  const dash = borderDash(line.style);
  ctx.setLineDash(dash ?? []);
  ctx.beginPath();
  ctx.moveTo(x1, y1);
  ctx.lineTo(x2, y2);
  ctx.stroke();
  if (line.style === "double") {
    ctx.beginPath();
    const horizontal = y1 === y2;
    if (horizontal) {
      ctx.moveTo(x1, y1 + 2);
      ctx.lineTo(x2, y2 + 2);
    } else {
      ctx.moveTo(x1 + 2, y1);
      ctx.lineTo(x2 + 2, y2);
    }
    ctx.stroke();
  }
  ctx.setLineDash([]);
}
function drawDiagonalBorders(ctx, x, y, w, h, b) {
  if (!b.diagonal)
    return;
  if (!b.diagonalUp && !b.diagonalDown)
    return;
  ctx.save();
  ctx.beginPath();
  ctx.rect(x, y, w, h);
  ctx.clip();
  if (b.diagonalDown)
    drawBorderLine(ctx, x, y, x + w, y + h, b.diagonal);
  if (b.diagonalUp)
    drawBorderLine(ctx, x, y + h, x + w, y, b.diagonal);
  ctx.restore();
}
function drawCellBorders(ctx, sheet, layout, g, vis) {
  const styles = layout.styles;
  const { covered, topLeftOf } = buildMergeMaps(sheet);
  for (const m of sheet.merges) {
    if (m.r2 < vis.firstRow || m.r1 > vis.lastRow)
      continue;
    if (m.c2 < vis.firstCol || m.c1 > vis.lastCol)
      continue;
    const tl = findCell(sheet, m.r1, m.c1);
    if (!tl)
      continue;
    const xf = resolveCellXf(tl, sheet, layout);
    if (!xf || xf.borderId === undefined)
      continue;
    const b = styles.borders[xf.borderId];
    if (!b)
      continue;
    const { x, y, w, h } = mergedRect(g, m);
    if (b.top)
      drawBorderLine(ctx, x, y, x + w, y, b.top);
    if (b.bottom)
      drawBorderLine(ctx, x, y + h, x + w, y + h, b.bottom);
    if (b.left)
      drawBorderLine(ctx, x, y, x, y + h, b.left);
    if (b.right)
      drawBorderLine(ctx, x + w, y, x + w, y + h, b.right);
    drawDiagonalBorders(ctx, x, y, w, h, b);
  }
  iterCellsInRange(sheet, vis.firstRow, vis.lastRow, vis.firstCol, vis.lastCol, (cell) => {
    const k = `${cell.r}:${cell.c}`;
    const xf = resolveCellXf(cell, sheet, layout);
    if (!xf || xf.borderId === undefined)
      return;
    const b = styles.borders[xf.borderId];
    if (!b)
      return;
    const merge = topLeftOf.get(k);
    const isCovered = covered.has(k);
    if (isCovered && merge) {
      const cr = cellRect(g, cell.r, cell.c);
      const { x: x2, y: y2, w: w2, h: h2 } = cr;
      const onTop = cell.r === merge.r1;
      const onBottom = cell.r === merge.r2;
      const onLeft = cell.c === merge.c1;
      const onRight = cell.c === merge.c2;
      if (onTop && b.top)
        drawBorderLine(ctx, x2, y2, x2 + w2, y2, b.top);
      if (onBottom && b.bottom)
        drawBorderLine(ctx, x2, y2 + h2, x2 + w2, y2 + h2, b.bottom);
      if (onLeft && b.left)
        drawBorderLine(ctx, x2, y2, x2, y2 + h2, b.left);
      if (onRight && b.right)
        drawBorderLine(ctx, x2 + w2, y2, x2 + w2, y2 + h2, b.right);
      return;
    }
    if (merge)
      return;
    const rect = cellRect(g, cell.r, cell.c);
    const { x, y, w, h } = rect;
    if (b.top)
      drawBorderLine(ctx, x, y, x + w, y, b.top);
    if (b.bottom)
      drawBorderLine(ctx, x, y + h, x + w, y + h, b.bottom);
    if (b.left)
      drawBorderLine(ctx, x, y, x, y + h, b.left);
    if (b.right)
      drawBorderLine(ctx, x + w, y, x + w, y + h, b.right);
    drawDiagonalBorders(ctx, x, y, w, h, b);
  });
}

// src/textRenderer.ts
function resolveCellSpans(cell, text, layout, baseFont, baseColor, defaultFontFamily, defaultFontSizePt) {
  const baseSizePt = baseFont?.size ?? defaultFontSizePt;
  const baseName = resolveSchemeName(baseFont?.scheme, layout) ?? baseFont?.name ?? defaultFontFamily;
  const baseFamily = baseFont?.family;
  const baseBold = baseFont?.bold ?? false;
  const baseItalic = baseFont?.italic ?? false;
  const baseUnderline = baseFont?.underline ?? false;
  const baseUnderlineStyle = baseFont?.underlineStyle;
  const baseStrike = baseFont?.strike ?? false;
  let runs;
  if (cell.runs && cell.runs.length > 0) {
    runs = cell.runs;
  } else if (cell.type === "s" && layout.sharedStringRuns && cell.value !== undefined) {
    const idx = parseInt(cell.value, 10);
    const sr = layout.sharedStringRuns[idx];
    if (sr && sr.length > 0)
      runs = sr;
  }
  const baseVertAlign = baseFont?.vertAlign;
  if (!runs) {
    return [
      buildSpan(text, baseSizePt, baseName, baseFamily, baseBold, baseItalic, baseColor, baseUnderline, baseUnderlineStyle, baseStrike, baseVertAlign)
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
    return buildSpan(r.text, sizePt, name, family, bold, italic, color, underline, underlineStyle, strike, vertAlign);
  });
}
function resolveSchemeName(scheme, layout) {
  if (!scheme || scheme === "none")
    return;
  const t = layout.theme;
  if (!t)
    return;
  if (scheme === "major")
    return t.majorFont || undefined;
  if (scheme === "minor")
    return t.minorFont || undefined;
  return;
}
function buildSpan(text, sizePt, name, family, bold, italic, color, underline, underlineStyle, strike, vertAlign) {
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
    baselineShiftPx: baselineShiftPx || undefined
  };
}
function paintTextDecorations(ctx, span, x, baseline, width, accountingExtent) {
  if (!span.underline && !span.strike)
    return;
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
function ptToPx(pt) {
  return pt * 4 / 3;
}
function familyFallbackChain(family) {
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
function cssFont(name, sizePt, bold, italic, family) {
  const px = ptToPx(sizePt);
  return `${italic ? "italic " : ""}${bold ? "bold " : ""}${px}px "${name}", ${familyFallbackChain(family)}`;
}
function layoutSpans(ctx, spans, maxWidth, wrap) {
  const lines = [];
  let current = { pieces: [], width: 0, height: 0, ascent: 0 };
  const finishLine = () => {
    if (current.height === 0) {
      const fallback = spans[spans.length - 1]?.fontSizePx ?? 14;
      current.height = fallback * 1.2;
      current.ascent = fallback * 0.8;
    }
    lines.push(current);
    current = { pieces: [], width: 0, height: 0, ascent: 0 };
  };
  const pushPiece = (span, text, width) => {
    current.pieces.push({ span, text, width });
    current.width += width;
    const lh = span.fontSizePx * 1.2;
    if (lh > current.height)
      current.height = lh;
    const asc = span.fontSizePx * 0.8;
    if (asc > current.ascent)
      current.ascent = asc;
  };
  for (const span of spans) {
    ctx.font = span.font;
    const segs = span.text.split(`
`);
    for (let si = 0;si < segs.length; si++) {
      const seg = segs[si];
      if (seg.length > 0) {
        if (!wrap) {
          pushPiece(span, seg, ctx.measureText(seg).width);
        } else {
          const tokens = seg.match(/\s+|\S+/g) ?? [];
          let buf = "";
          let bufW = 0;
          for (const tok of tokens) {
            const tokW = ctx.measureText(tok).width;
            if (current.width + bufW + tokW > maxWidth && (current.pieces.length > 0 || buf.length > 0)) {
              if (buf.length > 0) {
                pushPiece(span, buf, bufW);
                buf = "";
                bufW = 0;
              }
              finishLine();
              ctx.font = span.font;
              if (/^\s+$/.test(tok))
                continue;
            }
            buf += tok;
            bufW += tokW;
          }
          if (buf.length > 0)
            pushPiece(span, buf, bufW);
        }
      }
      if (si < segs.length - 1)
        finishLine();
    }
  }
  if (current.pieces.length > 0 || current.height > 0 || lines.length === 0) {
    finishLine();
  }
  return lines;
}
function occupiedCellsInRange(sheet, layout, firstRow, lastRow, firstCol, lastCol) {
  const occupied = new Set;
  iterCellsInRange(sheet, firstRow, lastRow, firstCol, lastCol, (cell) => {
    if (hasContent(cell, sheet, layout))
      occupied.add(`${cell.r}:${cell.c}`);
  });
  return occupied;
}
function drawCellText(ctx, sheet, layout, g, vis, cfDxfs, cfTextSuppress, cfIconReserve) {
  const { covered, topLeftOf } = buildMergeMaps(sheet);
  const styles = layout.styles;
  const overflowFirstCol = Math.max(1, vis.firstCol - 8);
  const overflowLastCol = Math.min(g.maxCol, vis.lastCol + 8);
  const occupied = occupiedCellsInRange(sheet, layout, vis.firstRow, vis.lastRow, overflowFirstCol, overflowLastCol);
  for (const k of covered)
    occupied.add(k);
  iterCellsInRange(sheet, vis.firstRow, vis.lastRow, overflowFirstCol, overflowLastCol, (cell) => {
    const k = `${cell.r}:${cell.c}`;
    if (covered.has(k))
      return;
    if (cfTextSuppress.has(k))
      return;
    const xf = resolveCellXf(cell, sheet, layout);
    const resolved = resolveCellText(cell, layout, xf);
    let { text } = resolved;
    const { defaultAlign, formatColor, fills } = resolved;
    if (!text)
      return;
    const baseFontEntry = xf?.fontId !== undefined ? styles.fonts[xf.fontId] : undefined;
    const dxf = cfDxfs.get(k);
    let font = baseFontEntry;
    if (dxf) {
      font = {
        ...baseFontEntry ?? {},
        bold: dxf.bold ?? baseFontEntry?.bold ?? false,
        italic: dxf.italic ?? baseFontEntry?.italic ?? false,
        underline: dxf.underline ?? baseFontEntry?.underline ?? false,
        underlineStyle: dxf.underlineStyle ?? baseFontEntry?.underlineStyle,
        strike: dxf.strike ?? baseFontEntry?.strike ?? false,
        color: dxf.fontColor ?? baseFontEntry?.color
      };
    }
    const baseColor = (dxf?.fontColor ? colorToCss(dxf.fontColor, "#000000") : formatColor) ?? colorToCss(font?.color, "#000000");
    const halign = xf?.horizontalAlignment ?? defaultAlign;
    const valign = xf?.verticalAlignment ?? "bottom";
    const wrap = xf?.wrapText ?? false;
    const spans = resolveCellSpans(cell, text, layout, font, baseColor, styles.defaultFont, styles.defaultFontSize);
    const ownRect = rectFor(sheet, g, cell.r, cell.c, topLeftOf);
    const merge = topLeftOf.get(k);
    const isMerged = !!merge;
    const padX = 4;
    if (fills && fills.length > 0 && text.includes("\x01")) {
      const primary = spans[0];
      const prevFont = ctx.font;
      ctx.font = primary.font;
      const stripped = text.replace(/\u0001/g, "");
      const baseW = ctx.measureText(stripped).width;
      const innerW2 = Math.max(0, ownRect.w - padX * 2);
      let avail = innerW2 - baseW;
      const parts = text.split("\x01");
      const fillCount = parts.length - 1;
      if (fillCount > 0) {
        let assembled = parts[0];
        for (let fi = 0;fi < fillCount; fi++) {
          const ch = fills[fi] ?? fills[fills.length - 1] ?? " ";
          const chW = Math.max(0.5, ctx.measureText(ch).width);
          const slice = avail / (fillCount - fi);
          const n = Math.max(0, Math.floor(slice / chW));
          avail -= n * chW;
          assembled += ch.repeat(n) + parts[fi + 1];
        }
        text = assembled;
        if (spans.length === 1)
          spans[0] = { ...spans[0], text };
      }
      ctx.font = prevFont;
    }
    const textRot = xf?.textRotation ?? 0;
    if (textRot !== 0) {
      const span = spans[0];
      ctx.save();
      ctx.beginPath();
      ctx.rect(ownRect.x, ownRect.y, ownRect.w, ownRect.h);
      ctx.clip();
      ctx.font = span.font;
      ctx.fillStyle = span.color;
      ctx.textBaseline = "alphabetic";
      if (textRot === 255) {
        const lineH = span.fontSizePx * 1.05;
        const ascent2 = span.fontSizePx * 0.8;
        const cx = ownRect.x + ownRect.w / 2;
        const totalH2 = lineH * text.length;
        let blockTop2;
        switch (valign) {
          case "top":
            blockTop2 = ownRect.y + 2;
            break;
          case "center":
            blockTop2 = ownRect.y + (ownRect.h - totalH2) / 2;
            break;
          default:
            blockTop2 = ownRect.y + ownRect.h - totalH2 - 2;
        }
        const prevAlign = ctx.textAlign;
        ctx.textAlign = "center";
        for (let i = 0;i < text.length; i++) {
          const ch = text[i];
          ctx.fillText(ch, cx, blockTop2 + i * lineH + ascent2);
        }
        ctx.textAlign = prevAlign;
        ctx.restore();
        return;
      }
      const angleRad = textRot <= 90 ? -textRot * Math.PI / 180 : (textRot - 90) * Math.PI / 180;
      const tw = ctx.measureText(text).width;
      const ascent = span.fontSizePx * 0.8;
      const pad = 2;
      let anchorX = ownRect.x + pad;
      const anchorY = angleRad < 0 ? ownRect.y + ownRect.h - pad : ownRect.y + pad + ascent;
      if (halign === "center") {
        const projW = Math.abs(tw * Math.cos(angleRad));
        const slack = ownRect.w - projW - pad * 2;
        if (slack > 0)
          anchorX += slack / 2;
      } else if (halign === "right") {
        const projW = Math.abs(tw * Math.cos(angleRad));
        const slack = ownRect.w - projW - pad * 2;
        if (slack > 0)
          anchorX += slack;
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
    const effectiveAlign = halign === "center" ? "center" : halign === "right" ? "right" : halign === "left" ? "left" : defaultAlign === "right" ? "right" : "left";
    const indentLeft = effectiveAlign === "left" ? indentPx : 0;
    const indentRight = effectiveAlign === "right" ? indentPx : 0;
    ctx.font = spans[0]?.font ?? `${styles.defaultFontSize * 4 / 3}px sans-serif`;
    const flatHasNewline = text.indexOf(`
`) >= 0;
    const clip = { ...ownRect };
    if (iconReserve > 0) {
      clip.x += iconReserve;
      clip.w -= iconReserve;
    }
    if (!wrap) {
      let maxLineW = 0;
      let curW = 0;
      for (const s of spans) {
        ctx.font = s.font;
        const segs = s.text.split(`
`);
        for (let i = 0;i < segs.length; i++) {
          const w = ctx.measureText(segs[i]).width;
          curW += w;
          if (i < segs.length - 1) {
            if (curW > maxLineW)
              maxLineW = curW;
            curW = 0;
          }
        }
      }
      if (curW > maxLineW)
        maxLineW = curW;
      const need = maxLineW + padX * 2 + indentLeft + indentRight;
      const leftCol = isMerged ? merge.c1 : cell.c;
      const rightCol = isMerged ? merge.c2 : cell.c;
      if (need > ownRect.w) {
        if (halign === "left" || halign === "general" || halign === undefined && defaultAlign === "left") {
          let cc = rightCol + 1;
          while (cc <= g.maxCol && !occupied.has(`${cell.r}:${cc}`)) {
            clip.w += g.colW[cc] ?? 0;
            cc++;
            if (clip.w >= need)
              break;
          }
        } else if (halign === "right" || halign === undefined && defaultAlign === "right") {
          let cc = leftCol - 1;
          while (cc >= 1 && !occupied.has(`${cell.r}:${cc}`)) {
            const w = g.colW[cc] ?? 0;
            clip.x -= w;
            clip.w += w;
            cc--;
            if (clip.w >= need)
              break;
          }
        } else if (halign === "center" || defaultAlign === "center") {
          let cl = leftCol - 1, cr = rightCol + 1;
          while (clip.w < need && (cl >= 1 || cr <= g.maxCol)) {
            if (cr <= g.maxCol && !occupied.has(`${cell.r}:${cr}`)) {
              clip.w += g.colW[cr] ?? 0;
              cr++;
            } else if (cl >= 1 && !occupied.has(`${cell.r}:${cl}`)) {
              const w = g.colW[cl] ?? 0;
              clip.x -= w;
              clip.w += w;
              cl--;
            } else
              break;
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
      const span = spans[0];
      ctx.font = span.font;
      ctx.fillStyle = span.color;
      let display = span.text;
      if (ctx.measureText(display).width > innerW && innerW > 8) {
        const ell = "…";
        let lo = 0, hi = display.length;
        while (lo < hi) {
          const mid = lo + hi + 1 >> 1;
          if (ctx.measureText(display.slice(0, mid) + ell).width <= innerW)
            lo = mid;
          else
            hi = mid - 1;
        }
        display = display.slice(0, lo) + ell;
      }
      const tw = ctx.measureText(display).width;
      let tx;
      switch (halign) {
        case "center":
          tx = clip.x + (clip.w - tw) / 2;
          break;
        case "right":
          tx = clip.x + clip.w - padX - indentRight - tw;
          break;
        default:
          if (defaultAlign === "right" && !halign)
            tx = clip.x + clip.w - padX - indentRight - tw;
          else
            tx = textOriginX + padX + indentLeft;
      }
      const ascent = span.fontSizePx * 0.8;
      let ty;
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
        w: Math.max(0, clip.w - 2)
      });
      ctx.restore();
      return;
    }
    const lines = layoutSpans(ctx, spans, innerW, wrap);
    const totalH = lines.reduce((a, l) => a + l.height, 0);
    let blockTop;
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
      let lineX;
      switch (halign) {
        case "center":
          lineX = clip.x + (clip.w - line.width) / 2;
          break;
        case "right":
          lineX = clip.x + clip.w - padX - indentRight - line.width;
          break;
        default:
          if (defaultAlign === "right" && !halign)
            lineX = clip.x + clip.w - padX - indentRight - line.width;
          else
            lineX = textOriginX + padX + indentLeft;
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
          w: Math.max(0, clip.w - 2)
        });
        cursorX += piece.width;
      }
      lineTop += line.height;
    }
    ctx.restore();
  });
}
function hasContent(cell, sheet, layout) {
  const xf = resolveCellXf(cell, sheet, layout);
  const { text } = resolveCellText(cell, layout, xf);
  return text.length > 0;
}
function drawFreezeIndicators(ctx, sheet, g, canvasW, canvasH) {
  if (!sheet.freeze)
    return;
  const { pcw, prh } = frozenDims(sheet, g);
  ctx.save();
  ctx.strokeStyle = "#9ca3af";
  ctx.lineWidth = 1;
  ctx.beginPath();
  if (sheet.freeze.leftCol > 1) {
    const x = Math.round(g.originX + pcw) + 0.5;
    ctx.moveTo(x, 0);
    ctx.lineTo(x, canvasH);
  }
  if (sheet.freeze.topRow > 1) {
    const y = Math.round(g.originY + prh) + 0.5;
    ctx.moveTo(0, y);
    ctx.lineTo(canvasW, y);
  }
  ctx.stroke();
  ctx.restore();
}

// src/cfIconState.ts
var ICON_RESERVE_PX = 18;
function computeCfIconState(sheet, locks) {
  const cfIconReserve = new Map;
  const cfIconDraw = new Map;
  const cfIconSuppress = new Set;
  const cfs = sheet.conditionalFormats;
  if (!cfs || cfs.length === 0)
    return { cfIconReserve, cfIconDraw, cfIconSuppress };
  const cellNumeric = new Map;
  iterAllCells(sheet, (cell) => {
    if (cell.value === undefined)
      return;
    if (cell.type === "n" || cell.type === "f") {
      const n = parseFloat(cell.value);
      if (!Number.isNaN(n))
        cellNumeric.set(`${cell.r}:${cell.c}`, n);
    }
  });
  for (const cf of cfs) {
    const rule = cf.rules.filter((r) => r.kind === "iconSet" && r.iconSet).sort((a, b) => a.priority - b.priority)[0];
    if (!rule || !rule.iconSet)
      continue;
    const is = rule.iconSet;
    const n = is.cfvos.length;
    if (n < 3)
      continue;
    const values = [];
    for (const range of cf.ranges) {
      for (let r = range.r1;r <= range.r2; r++) {
        for (let c = range.c1;c <= range.c2; c++) {
          const v = cellNumeric.get(`${r}:${c}`);
          if (v !== undefined)
            values.push(v);
        }
      }
    }
    if (values.length === 0)
      continue;
    const dataMin = Math.min(...values);
    const dataMax = Math.max(...values);
    const sorted = [...values].sort((a, b) => a - b);
    const thresholds = is.cfvos.map((s, i) => resolveCfvoValue(s, dataMin, dataMax, sorted, i === 0));
    for (const range of cf.ranges) {
      for (let r = range.r1;r <= range.r2; r++) {
        for (let c = range.c1;c <= range.c2; c++) {
          const k = `${r}:${c}`;
          if (isCfLocked(locks, k, rule.priority))
            continue;
          const v = cellNumeric.get(k);
          if (v === undefined)
            continue;
          let idx = 0;
          for (let i = 1;i < n; i++) {
            if (v >= thresholds[i])
              idx = i;
          }
          if (is.reverse)
            idx = n - 1 - idx;
          cfIconReserve.set(k, ICON_RESERVE_PX);
          cfIconDraw.set(k, { iconSet: is.iconSet, idx, n });
          if (!is.showValue)
            cfIconSuppress.add(k);
        }
      }
    }
  }
  return { cfIconReserve, cfIconDraw, cfIconSuppress };
}

// src/conditionalFormatting.ts
var PREDICATE_KINDS = new Set([
  "cellIs",
  "top10",
  "aboveAverage",
  "duplicateValues",
  "uniqueValues",
  "containsText",
  "notContainsText",
  "beginsWith",
  "endsWith",
  "timePeriod"
]);
function computeCfStopLocks(sheet, layout) {
  const locks = new Map;
  const cfs = sheet.conditionalFormats;
  if (!cfs || cfs.length === 0)
    return locks;
  const cellByKey = new Map;
  iterAllCells(sheet, (cell) => {
    cellByKey.set(`${cell.r}:${cell.c}`, cell);
  });
  const entries = [];
  for (const cf of cfs)
    for (const rule of cf.rules)
      entries.push({ rule, ranges: cf.ranges });
  entries.sort((a, b) => a.rule.priority - b.rule.priority);
  for (const { rule, ranges } of entries) {
    if (!rule.stopIfTrue)
      continue;
    let matched;
    if (PREDICATE_KINDS.has(rule.kind)) {
      matched = computeRuleMatchSet(rule, ranges, cellByKey, layout);
    } else if (rule.kind === "colorScale" || rule.kind === "dataBar" || rule.kind === "iconSet") {
      const all = [];
      for (const range of ranges) {
        for (let r = range.r1;r <= range.r2; r++) {
          for (let c = range.c1;c <= range.c2; c++)
            all.push(`${r}:${c}`);
        }
      }
      matched = all;
    } else {
      continue;
    }
    for (const k of matched) {
      const cur = locks.get(k);
      if (cur === undefined || rule.priority < cur)
        locks.set(k, rule.priority);
    }
  }
  return locks;
}
function isCfLocked(locks, cellKey, rulePriority) {
  if (!locks)
    return false;
  const at = locks.get(cellKey);
  return at !== undefined && at < rulePriority;
}
function computeCfDxfMap(sheet, layout, locks) {
  const out = new Map;
  const dxfs = layout.dxfs ?? [];
  if (dxfs.length === 0)
    return out;
  const cfs = sheet.conditionalFormats;
  if (!cfs || cfs.length === 0)
    return out;
  const cellByKey = new Map;
  iterAllCells(sheet, (cell) => {
    cellByKey.set(`${cell.r}:${cell.c}`, cell);
  });
  const locallyLocked = new Set;
  for (const cf of cfs) {
    const sortedRules = [...cf.rules].sort((a, b) => a.priority - b.priority);
    for (const rule of sortedRules) {
      if (!PREDICATE_KINDS.has(rule.kind))
        continue;
      if (rule.dxfId === undefined)
        continue;
      const dxf = dxfs[rule.dxfId];
      if (!dxf)
        continue;
      const matched = computeRuleMatchSet(rule, cf.ranges, cellByKey, layout);
      if (matched.size === 0)
        continue;
      for (const k of matched) {
        if (locallyLocked.has(k))
          continue;
        if (isCfLocked(locks, k, rule.priority))
          continue;
        const prev = out.get(k);
        out.set(k, prev ? mergeDxf(prev, dxf) : dxf);
        if (rule.stopIfTrue)
          locallyLocked.add(k);
      }
    }
  }
  return out;
}
function computeRuleMatchSet(rule, ranges, cellByKey, layout) {
  const out = new Set;
  const covered = [];
  for (const range of ranges) {
    for (let r = range.r1;r <= range.r2; r++) {
      for (let c = range.c1;c <= range.c2; c++) {
        const k = `${r}:${c}`;
        covered.push({ k, cell: cellByKey.get(k) });
      }
    }
  }
  switch (rule.kind) {
    case "cellIs": {
      for (const { k, cell } of covered) {
        if (evaluateCellIs(cell, rule.operator, rule.operands, layout))
          out.add(k);
      }
      break;
    }
    case "top10": {
      const nums = [];
      for (const { k, cell } of covered) {
        if (!cell)
          continue;
        const v = cellNumericValue(cell);
        if (v !== null)
          nums.push({ k, v });
      }
      if (nums.length === 0)
        break;
      const rankRaw = rule.rank ?? 10;
      let n;
      if (rule.percent) {
        const pct = Math.max(0, Math.min(100, rankRaw));
        n = Math.max(1, Math.min(nums.length, Math.ceil(nums.length * pct / 100)));
      } else {
        n = Math.max(1, Math.min(nums.length, rankRaw));
      }
      nums.sort((a, b) => rule.bottom ? a.v - b.v : b.v - a.v);
      const cutoff = nums[n - 1].v;
      for (const { k, v } of nums) {
        if (rule.bottom ? v <= cutoff : v >= cutoff)
          out.add(k);
      }
      break;
    }
    case "aboveAverage": {
      const nums = [];
      for (const { k, cell } of covered) {
        if (!cell)
          continue;
        const v = cellNumericValue(cell);
        if (v !== null)
          nums.push({ k, v });
      }
      if (nums.length === 0)
        break;
      const above = rule.aboveAverage ?? true;
      const mean = nums.reduce((s, x) => s + x.v, 0) / nums.length;
      let threshold = mean;
      if (rule.stdDev !== undefined && rule.stdDev !== null) {
        const variance = nums.reduce((s, x) => s + (x.v - mean) ** 2, 0) / nums.length;
        const sd = Math.sqrt(variance);
        const k = Math.abs(rule.stdDev);
        threshold = above ? mean + k * sd : mean - k * sd;
      }
      for (const { k, v } of nums) {
        let hit;
        if (above) {
          hit = rule.equalAverage ? v >= threshold : v > threshold;
        } else {
          hit = rule.equalAverage ? v <= threshold : v < threshold;
        }
        if (hit)
          out.add(k);
      }
      break;
    }
    case "duplicateValues":
    case "uniqueValues": {
      const counts = new Map;
      const keyOf = [];
      for (const { k, cell } of covered) {
        if (!cell || cell.value === undefined || cell.value === "") {
          keyOf.push({ k, bucket: null });
          continue;
        }
        const num = cellNumericValue(cell);
        const bucket = num !== null ? `n:${num}` : `s:${cellTextValue(cell, layout)}`;
        keyOf.push({ k, bucket });
        counts.set(bucket, (counts.get(bucket) ?? 0) + 1);
      }
      const wantDup = rule.kind === "duplicateValues";
      for (const { k, bucket } of keyOf) {
        if (bucket === null)
          continue;
        const c = counts.get(bucket) ?? 0;
        if (wantDup ? c > 1 : c === 1)
          out.add(k);
      }
      break;
    }
    case "containsText":
    case "notContainsText":
    case "beginsWith":
    case "endsWith": {
      const needle = (rule.text ?? "").toLowerCase();
      if (needle.length === 0)
        break;
      for (const { k, cell } of covered) {
        if (!cell) {
          if (rule.kind === "notContainsText")
            out.add(k);
          continue;
        }
        const hay = cellTextValue(cell, layout).toLowerCase();
        let hit = false;
        switch (rule.kind) {
          case "containsText":
            hit = hay.includes(needle);
            break;
          case "notContainsText":
            hit = !hay.includes(needle);
            break;
          case "beginsWith":
            hit = hay.startsWith(needle);
            break;
          case "endsWith":
            hit = hay.endsWith(needle);
            break;
        }
        if (hit)
          out.add(k);
      }
      break;
    }
    case "timePeriod": {
      const today = new Date;
      today.setHours(0, 0, 0, 0);
      const period = rule.timePeriod ?? "today";
      for (const { k, cell } of covered) {
        if (!cell)
          continue;
        const v = cellNumericValue(cell);
        if (v === null)
          continue;
        const cellDate = excelSerialToDate(v);
        if (!cellDate)
          continue;
        cellDate.setHours(0, 0, 0, 0);
        if (matchesTimePeriod(cellDate, today, period))
          out.add(k);
      }
      break;
    }
  }
  return out;
}
function excelSerialToDate(serial) {
  if (!isFinite(serial) || serial < 0)
    return null;
  const ms = (serial - 25569) * 86400 * 1000;
  const d = new Date(ms);
  return Number.isNaN(d.getTime()) ? null : d;
}
function matchesTimePeriod(cellDay, today, period) {
  const dayMs = 86400 * 1000;
  const diffDays = Math.round((cellDay.getTime() - today.getTime()) / dayMs);
  switch (period) {
    case "today":
      return diffDays === 0;
    case "yesterday":
      return diffDays === -1;
    case "tomorrow":
      return diffDays === 1;
    case "last7Days":
      return diffDays <= 0 && diffDays >= -6;
    case "thisWeek": {
      const tow = today.getDay();
      const start = new Date(today);
      start.setDate(today.getDate() - tow);
      const end = new Date(start);
      end.setDate(start.getDate() + 6);
      return cellDay >= start && cellDay <= end;
    }
    case "lastWeek": {
      const tow = today.getDay();
      const start = new Date(today);
      start.setDate(today.getDate() - tow - 7);
      const end = new Date(start);
      end.setDate(start.getDate() + 6);
      return cellDay >= start && cellDay <= end;
    }
    case "nextWeek": {
      const tow = today.getDay();
      const start = new Date(today);
      start.setDate(today.getDate() - tow + 7);
      const end = new Date(start);
      end.setDate(start.getDate() + 6);
      return cellDay >= start && cellDay <= end;
    }
    case "thisMonth":
      return cellDay.getFullYear() === today.getFullYear() && cellDay.getMonth() === today.getMonth();
    case "lastMonth": {
      const m = new Date(today.getFullYear(), today.getMonth() - 1, 1);
      return cellDay.getFullYear() === m.getFullYear() && cellDay.getMonth() === m.getMonth();
    }
    case "nextMonth": {
      const m = new Date(today.getFullYear(), today.getMonth() + 1, 1);
      return cellDay.getFullYear() === m.getFullYear() && cellDay.getMonth() === m.getMonth();
    }
  }
  return false;
}
function mergeDxf(base, overlay) {
  return {
    fontColor: base.fontColor ?? overlay.fontColor,
    bold: base.bold ?? overlay.bold,
    italic: base.italic ?? overlay.italic,
    strike: base.strike ?? overlay.strike,
    underline: base.underline ?? overlay.underline,
    underlineStyle: base.underlineStyle ?? overlay.underlineStyle,
    fillColor: base.fillColor ?? overlay.fillColor,
    numFmt: base.numFmt ?? overlay.numFmt
  };
}
function evaluateCellIs(cell, operator, operands, _layout) {
  if (!cell || !operator || operands.length === 0)
    return false;
  const cellNum = cellNumericValue(cell);
  const cellStr = cellTextValue(cell, _layout);
  const a = parseCfOperand(operands[0]);
  const b = operands.length > 1 ? parseCfOperand(operands[1]) : undefined;
  if (a === null)
    return false;
  const cellIsText = cellNum === null && cellStr.length > 0;
  const aNum = typeof a === "number" ? a : NaN;
  const bNum = b !== undefined && typeof b === "number" ? b : NaN;
  const cmp = (lhsNum, lhsStr, op, rhsNum, rhsIsStr, rhsStr) => {
    if (rhsIsStr) {
      if (lhsNum !== null)
        return op === "notEqual";
      switch (op) {
        case "equal":
          return lhsStr === rhsStr;
        case "notEqual":
          return lhsStr !== rhsStr;
        case "greaterThan":
          return lhsStr > rhsStr;
        case "greaterThanOrEqual":
          return lhsStr >= rhsStr;
        case "lessThan":
          return lhsStr < rhsStr;
        case "lessThanOrEqual":
          return lhsStr <= rhsStr;
      }
      return false;
    }
    const lhs = lhsNum !== null ? lhsNum : lhsStr.length > 0 ? Infinity : null;
    if (lhs === null)
      return null;
    switch (op) {
      case "equal":
        return lhs === rhsNum;
      case "notEqual":
        return lhs !== rhsNum;
      case "greaterThan":
        return lhs > rhsNum;
      case "greaterThanOrEqual":
        return lhs >= rhsNum;
      case "lessThan":
        return lhs < rhsNum;
      case "lessThanOrEqual":
        return lhs <= rhsNum;
    }
    return false;
  };
  switch (operator) {
    case "equal":
    case "notEqual":
    case "greaterThan":
    case "greaterThanOrEqual":
    case "lessThan":
    case "lessThanOrEqual":
      return cmp(cellNum, cellStr, operator, aNum, typeof a === "string", typeof a === "string" ? a : "") === true;
    case "between":
    case "notBetween": {
      if (b === undefined)
        return false;
      if (typeof a !== "number" || typeof b !== "number")
        return false;
      const lo = Math.min(aNum, bNum), hi = Math.max(aNum, bNum);
      if (cellIsText)
        return operator === "notBetween";
      if (cellNum === null)
        return false;
      const inside = cellNum >= lo && cellNum <= hi;
      return operator === "between" ? inside : !inside;
    }
  }
  return false;
}
function parseCfOperand(s) {
  const t = s.trim();
  if (t.length === 0)
    return null;
  if (t.startsWith('"') && t.endsWith('"')) {
    return t.slice(1, -1).replace(/""/g, '"');
  }
  const body = t.startsWith("=") ? t.slice(1).trim() : t;
  if (/^-?\d+(\.\d+)?([eE][-+]?\d+)?$/.test(body)) {
    const n = parseFloat(body);
    return Number.isNaN(n) ? null : n;
  }
  return null;
}
function drawConditionalFormats(ctx, sheet, layout, g, vis, cfDxfs, locks) {
  if (cfDxfs.size > 0) {
    const { covered: covered2, topLeftOf: topLeftOf2 } = buildMergeMaps(sheet);
    for (const [k, dxf] of cfDxfs) {
      if (!dxf.fillColor)
        continue;
      if (covered2.has(k))
        continue;
      const [rs, cs] = k.split(":");
      const r = parseInt(rs, 10), c = parseInt(cs, 10);
      if (r < vis.firstRow || r > vis.lastRow)
        continue;
      if (c < vis.firstCol || c > vis.lastCol)
        continue;
      const rect = topLeftOf2.has(k) ? mergedRect(g, topLeftOf2.get(k)) : cellRect(g, r, c);
      ctx.fillStyle = colorToCss(dxf.fillColor, "#ffffff");
      ctx.fillRect(rect.x, rect.y, rect.w, rect.h);
    }
  }
  const cfs = sheet.conditionalFormats;
  if (!cfs || cfs.length === 0)
    return;
  const { covered, topLeftOf } = buildMergeMaps(sheet);
  const cellNumeric = new Map;
  iterAllCells(sheet, (cell) => {
    if (cell.value === undefined)
      return;
    if (cell.type === "n" || cell.type === "f") {
      const n = parseFloat(cell.value);
      if (!Number.isNaN(n))
        cellNumeric.set(`${cell.r}:${cell.c}`, n);
    }
  });
  for (const cf of cfs) {
    const rule = cf.rules.filter((r) => r.kind === "colorScale" && r.colorScale).sort((a, b) => a.priority - b.priority)[0];
    if (!rule || !rule.colorScale)
      continue;
    const values = [];
    for (const range of cf.ranges) {
      for (let r = range.r1;r <= range.r2; r++) {
        for (let c = range.c1;c <= range.c2; c++) {
          const v = cellNumeric.get(`${r}:${c}`);
          if (v !== undefined)
            values.push(v);
        }
      }
    }
    if (values.length === 0)
      continue;
    const stops = resolveColorScaleStops(rule.colorScale, values);
    if (stops.length < 2)
      continue;
    for (const range of cf.ranges) {
      const r1 = Math.max(range.r1, vis.firstRow);
      const r2 = Math.min(range.r2, vis.lastRow);
      const c1 = Math.max(range.c1, vis.firstCol);
      const c2 = Math.min(range.c2, vis.lastCol);
      for (let r = r1;r <= r2; r++) {
        for (let c = c1;c <= c2; c++) {
          const k = `${r}:${c}`;
          if (covered.has(k))
            continue;
          if (isCfLocked(locks, k, rule.priority))
            continue;
          const v = cellNumeric.get(k);
          if (v === undefined)
            continue;
          const css = interpolateStops(stops, v);
          if (!css)
            continue;
          const rect = topLeftOf.has(k) ? mergedRect(g, topLeftOf.get(k)) : cellRect(g, r, c);
          ctx.fillStyle = css;
          ctx.fillRect(rect.x, rect.y, rect.w, rect.h);
        }
      }
    }
  }
  for (const cf of cfs) {
    const rule = cf.rules.filter((r) => r.kind === "dataBar" && r.dataBar).sort((a, b) => a.priority - b.priority)[0];
    if (!rule || !rule.dataBar)
      continue;
    const db = rule.dataBar;
    const values = [];
    for (const range of cf.ranges) {
      for (let r = range.r1;r <= range.r2; r++) {
        for (let c = range.c1;c <= range.c2; c++) {
          const v = cellNumeric.get(`${r}:${c}`);
          if (v !== undefined)
            values.push(v);
        }
      }
    }
    if (values.length === 0)
      continue;
    const dataMin = Math.min(...values);
    const dataMax = Math.max(...values);
    const sorted = [...values].sort((a, b) => a - b);
    const minVal = resolveCfvoValue(db.min, dataMin, dataMax, sorted, true);
    const maxVal = resolveCfvoValue(db.max, dataMin, dataMax, sorted, false);
    if (!isFinite(minVal) || !isFinite(maxVal) || maxVal <= minVal)
      continue;
    const minPct = (db.minLengthPct ?? 10) / 100;
    const maxPct = (db.maxLengthPct ?? 90) / 100;
    const posCss = colorToCss(db.color, "#638EC6");
    const negCss = db.negativeColor ? colorToCss(db.negativeColor, "#FF0000") : "#FF0000";
    const straddles = minVal < 0 && maxVal > 0;
    const axisFrac = straddles ? -minVal / (maxVal - minVal) : 0;
    for (const range of cf.ranges) {
      const r1 = Math.max(range.r1, vis.firstRow);
      const r2 = Math.min(range.r2, vis.lastRow);
      const c1 = Math.max(range.c1, vis.firstCol);
      const c2 = Math.min(range.c2, vis.lastCol);
      for (let r = r1;r <= r2; r++) {
        for (let c = c1;c <= c2; c++) {
          const k = `${r}:${c}`;
          if (covered.has(k))
            continue;
          if (isCfLocked(locks, k, rule.priority))
            continue;
          const v = cellNumeric.get(k);
          if (v === undefined)
            continue;
          const rect = topLeftOf.has(k) ? mergedRect(g, topLeftOf.get(k)) : cellRect(g, r, c);
          const inset = 1;
          const bx = rect.x + inset;
          const by = rect.y + inset;
          const bw = Math.max(0, rect.w - inset * 2);
          const bh = Math.max(0, rect.h - inset * 2);
          if (bw <= 0 || bh <= 0)
            continue;
          const fillBar = (x, y, w, h, css, anchor) => {
            if (w <= 0 || h <= 0)
              return;
            if (db.gradient !== false) {
              const x0 = anchor === "left" ? x : x + w;
              const x1 = anchor === "left" ? x + w : x;
              const grad = ctx.createLinearGradient(x0, y, x1, y);
              grad.addColorStop(0, withAlpha(css, 1));
              grad.addColorStop(0.7, withAlpha(css, 0.8));
              grad.addColorStop(1, withAlpha(css, 0.05));
              ctx.fillStyle = grad;
            } else {
              ctx.fillStyle = css;
            }
            ctx.fillRect(x, y, w, h);
          };
          if (straddles) {
            const axisX = bx + bw * axisFrac;
            if (v >= 0) {
              const t = Math.min(1, v / maxVal);
              const len = bw * (1 - axisFrac) * (minPct + t * (maxPct - minPct));
              fillBar(axisX, by, len, bh, posCss, "left");
            } else {
              const t = Math.min(1, -v / -minVal);
              const len = bw * axisFrac * (minPct + t * (maxPct - minPct));
              fillBar(axisX - len, by, len, bh, negCss, "right");
            }
            ctx.fillStyle = "#000000";
            ctx.fillRect(Math.round(axisX) - 0.5, by, 1, bh);
          } else {
            const t = Math.max(0, Math.min(1, (v - minVal) / (maxVal - minVal)));
            const len = bw * (minPct + t * (maxPct - minPct));
            fillBar(bx, by, len, bh, posCss, "left");
          }
        }
      }
    }
  }
}
function resolveCfvoValue(s, dataMin, dataMax, sorted, isMin) {
  switch (s.type) {
    case "min":
    case "automin":
      return Math.min(0, dataMin);
    case "max":
    case "automax":
      return Math.max(0, dataMax);
    case "num":
    case "formula":
      return parseFloat(s.val ?? (isMin ? `${dataMin}` : `${dataMax}`));
    case "percent": {
      const p = parseFloat(s.val ?? (isMin ? "0" : "100")) / 100;
      return dataMin + (dataMax - dataMin) * p;
    }
    case "percentile": {
      const p = parseFloat(s.val ?? (isMin ? "0" : "100")) / 100;
      if (sorted.length === 0)
        return isMin ? dataMin : dataMax;
      const idx = Math.min(sorted.length - 1, Math.max(0, Math.round(p * (sorted.length - 1))));
      return sorted[idx] ?? (isMin ? dataMin : dataMax);
    }
    default:
      return isMin ? dataMin : dataMax;
  }
}
function computeCfTextSuppress(sheet, locks) {
  const out = new Set;
  const cfs = sheet.conditionalFormats;
  if (!cfs || cfs.length === 0)
    return out;
  for (const cf of cfs) {
    const rule = cf.rules.filter((r) => r.kind === "dataBar" && r.dataBar && r.dataBar.showValue === false).sort((a, b) => a.priority - b.priority)[0];
    if (!rule)
      continue;
    for (const range of cf.ranges) {
      for (let r = range.r1;r <= range.r2; r++) {
        for (let c = range.c1;c <= range.c2; c++) {
          const k = `${r}:${c}`;
          if (isCfLocked(locks, k, rule.priority))
            continue;
          out.add(k);
        }
      }
    }
  }
  return out;
}
function resolveColorScaleStops(cs, values) {
  const min = Math.min(...values);
  const max = Math.max(...values);
  const sorted = [...values].sort((a, b) => a - b);
  return cs.stops.map((s) => {
    let v;
    switch (s.type) {
      case "min":
        v = min;
        break;
      case "max":
        v = max;
        break;
      case "num":
        v = parseFloat(s.val ?? "0");
        break;
      case "percent": {
        const p = parseFloat(s.val ?? "0") / 100;
        v = min + (max - min) * p;
        break;
      }
      case "percentile": {
        const p = parseFloat(s.val ?? "0") / 100;
        const idx = Math.min(sorted.length - 1, Math.max(0, Math.round(p * (sorted.length - 1))));
        v = sorted[idx] ?? min;
        break;
      }
      default:
        v = min;
    }
    return { value: v, rgb: rgbTriple(s.color) };
  }).sort((a, b) => a.value - b.value);
}
function rgbTriple(c) {
  const css = colorToCss(c, "#ffffff");
  return [
    parseInt(css.slice(1, 3), 16),
    parseInt(css.slice(3, 5), 16),
    parseInt(css.slice(5, 7), 16)
  ];
}
function interpolateStops(stops, value) {
  if (stops.length === 0)
    return null;
  const first = stops[0];
  const last = stops[stops.length - 1];
  if (value <= first.value)
    return rgbToCss(first.rgb);
  if (value >= last.value)
    return rgbToCss(last.rgb);
  for (let i = 0;i < stops.length - 1; i++) {
    const a = stops[i];
    const b = stops[i + 1];
    if (value >= a.value && value <= b.value) {
      const span = b.value - a.value;
      const t = span === 0 ? 0 : (value - a.value) / span;
      const r = Math.round(a.rgb[0] + (b.rgb[0] - a.rgb[0]) * t);
      const gg = Math.round(a.rgb[1] + (b.rgb[1] - a.rgb[1]) * t);
      const bb = Math.round(a.rgb[2] + (b.rgb[2] - a.rgb[2]) * t);
      return `rgb(${r}, ${gg}, ${bb})`;
    }
  }
  return null;
}
function rgbToCss(rgb) {
  return `rgb(${rgb[0]}, ${rgb[1]}, ${rgb[2]})`;
}

// src/cfIcons.ts
function drawCfIcons(ctx, sheet, g, vis, cfIconDraw) {
  if (cfIconDraw.size === 0)
    return;
  const { covered, topLeftOf } = buildMergeMaps(sheet);
  const ICON_PX = 12;
  const INSET_X = 3;
  for (const [k, info] of cfIconDraw) {
    if (covered.has(k))
      continue;
    const [rs, cs] = k.split(":");
    const r = parseInt(rs, 10), c = parseInt(cs, 10);
    if (r < vis.firstRow || r > vis.lastRow)
      continue;
    if (c < vis.firstCol || c > vis.lastCol)
      continue;
    const rect = topLeftOf.has(k) ? mergedRect(g, topLeftOf.get(k)) : cellRect(g, r, c);
    const x = rect.x + INSET_X;
    const y = rect.y + (rect.h - ICON_PX) / 2;
    drawIconGlyph(ctx, info.iconSet, info.idx, info.n, x, y, ICON_PX);
  }
}
function drawIconGlyph(ctx, iconSet, idx, n, x, y, s) {
  const redYelGreen = (i, total) => {
    if (total <= 1)
      return "#63BE7B";
    const stops = ["#F8696B", "#FCB14E", "#FFEB84", "#B1D580", "#63BE7B"];
    if (total === 3)
      return [stops[0], stops[2], stops[4]][i] ?? "#888";
    if (total === 4)
      return [stops[0], stops[1], stops[3], stops[4]][i] ?? "#888";
    return stops[i] ?? "#888";
  };
  const grayScale = (i, total) => {
    const t = total <= 1 ? 0 : i / (total - 1);
    const v = Math.round(60 + t * (210 - 60));
    return `rgb(${v},${v},${v})`;
  };
  const setLower = iconSet.toLowerCase();
  const isGray = setLower.includes("gray");
  const colorAt = (i) => isGray ? grayScale(i, n) : redYelGreen(i, n);
  ctx.save();
  ctx.translate(x, y);
  const cx = s / 2, cy = s / 2;
  if (setLower.includes("arrow")) {
    const angleFor = (i, total) => {
      if (total === 3)
        return [180, 90, 0][i] ?? 90;
      if (total === 4)
        return [180, 135, 45, 0][i] ?? 90;
      return [180, 135, 90, 45, 0][i] ?? 90;
    };
    const ang = angleFor(idx, n) * Math.PI / 180;
    ctx.translate(cx, cy);
    ctx.rotate(ang);
    ctx.fillStyle = colorAt(idx);
    const h = s * 0.45, w = s * 0.35, stem = s * 0.18;
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
  if (setLower.includes("trafficlight") || setLower.includes("signs") || setLower.includes("flag")) {
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
    for (let i = 0;i < n; i++) {
      const bx = bx0 + i * (bw + gap);
      const filledHere = i < filled;
      ctx.fillStyle = filledHere ? "#444" : "#cccccc";
      const bh = s * 0.55 * (0.4 + 0.6 * (i + 1) / n);
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
    const wedges = [
      [-Math.PI / 2, 0],
      [0, Math.PI / 2],
      [Math.PI / 2, Math.PI],
      [Math.PI, Math.PI * 3 / 2]
    ];
    const fill = Math.min(idx, 4);
    for (let i = 0;i < fill; i++) {
      ctx.beginPath();
      ctx.moveTo(cx, cy);
      ctx.arc(cx, cy, s * 0.42, wedges[i][0], wedges[i][1]);
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
    for (let i = 0;i < n; i++) {
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
function drawStarFill(ctx, cx, cy, r, fillFrac) {
  const pts = [];
  for (let i = 0;i < 10; i++) {
    const ang = -Math.PI / 2 + i * Math.PI / 5;
    const rr = i % 2 === 0 ? r : r * 0.45;
    pts.push([cx + rr * Math.cos(ang), cy + rr * Math.sin(ang)]);
  }
  ctx.beginPath();
  pts.forEach(([x, y], i) => {
    if (i === 0)
      ctx.moveTo(x, y);
    else
      ctx.lineTo(x, y);
  });
  ctx.closePath();
  ctx.strokeStyle = "#aaa";
  ctx.lineWidth = 1;
  ctx.stroke();
  if (fillFrac <= 0)
    return;
  if (fillFrac >= 1) {
    ctx.fillStyle = "#f5b400";
    ctx.fill();
    return;
  }
  ctx.save();
  ctx.beginPath();
  pts.forEach(([x, y], i) => {
    if (i === 0)
      ctx.moveTo(x, y);
    else
      ctx.lineTo(x, y);
  });
  ctx.closePath();
  ctx.clip();
  ctx.fillStyle = "#f5b400";
  ctx.fillRect(cx - r, cy - r, 2 * r * fillFrac, 2 * r);
  ctx.restore();
}

// src/sparklines.ts
var PAD = 2;
var MIN_CELL_W = 14;
var MIN_CELL_H = 10;
var DEFAULT_SERIES = "#376092";
var DEFAULT_NEGATIVE = "#FF0000";
var DEFAULT_AXIS = "#000000";
var DEFAULT_MARKERS = "#D00000";
var DEFAULT_HIGH = "#00B050";
var DEFAULT_LOW = "#FF0000";
var DEFAULT_FIRST = "#92D050";
var DEFAULT_LAST = "#92D050";
function drawSparklines(ctx, sheet, g, vis) {
  const groups = sheet.sparklineGroups;
  if (!groups || groups.length === 0)
    return;
  for (const group of groups) {
    for (const sp of group.sparklines) {
      if (sp.r < vis.firstRow || sp.r > vis.lastRow)
        continue;
      if (sp.c < vis.firstCol || sp.c > vis.lastCol)
        continue;
      const rect = cellRect(g, sp.r, sp.c);
      if (rect.w < MIN_CELL_W || rect.h < MIN_CELL_H)
        continue;
      if (rect.y < g.originY || rect.x < g.originX)
        continue;
      const inner = {
        x: rect.x + PAD,
        y: rect.y + PAD,
        w: Math.max(1, rect.w - 2 * PAD),
        h: Math.max(1, rect.h - 2 * PAD)
      };
      ctx.save();
      ctx.beginPath();
      ctx.rect(rect.x, rect.y, rect.w, rect.h);
      ctx.clip();
      switch (group.sparkType) {
        case "column":
          drawColumnSparkline(ctx, group, sp, inner);
          break;
        case "stacked":
          drawWinLossSparkline(ctx, group, sp, inner);
          break;
        case "line":
        default:
          drawLineSparkline(ctx, group, sp, inner);
          break;
      }
      ctx.restore();
    }
  }
}
function resolveRange(group, values) {
  const present = values.filter((v) => v != null);
  let lo = present.length ? Math.min(...present) : 0;
  let hi = present.length ? Math.max(...present) : 1;
  switch (group.minAxisType) {
    case "group":
      if (group.groupMin != null)
        lo = group.groupMin;
      break;
    case "custom":
      if (group.manualMin != null)
        lo = group.manualMin;
      break;
  }
  switch (group.maxAxisType) {
    case "group":
      if (group.groupMax != null)
        hi = group.groupMax;
      break;
    case "custom":
      if (group.manualMax != null)
        hi = group.manualMax;
      break;
  }
  if (lo > 0 && group.minAxisType === "individual")
    lo = 0;
  if (hi < 0 && group.maxAxisType === "individual")
    hi = 0;
  if (hi - lo < 0.000000000001) {
    hi = lo + 0.5;
    lo = lo - 0.5;
  }
  return { min: lo, max: hi };
}
function drawLineSparkline(ctx, group, sp, rect) {
  const values = sp.values ?? [];
  if (values.length === 0)
    return;
  const range = resolveRange(group, values);
  const yOf = (v) => {
    const t = (v - range.min) / (range.max - range.min);
    return rect.y + (1 - t) * rect.h;
  };
  const xOf = (i) => {
    if (values.length === 1)
      return rect.x + rect.w / 2;
    return rect.x + i / (values.length - 1) * rect.w;
  };
  const seriesColor = group.colorSeries ? `#${group.colorSeries}` : DEFAULT_SERIES;
  ctx.lineWidth = Math.max(0.5, group.lineWeight ?? 0.75);
  ctx.strokeStyle = seriesColor;
  const empty = group.displayEmptyCellsAs || "gap";
  ctx.beginPath();
  let prevDrawn = false;
  for (let i = 0;i < values.length; i++) {
    const v = values[i];
    if (v == null) {
      if (empty === "zero") {
        const x2 = xOf(i);
        const y2 = yOf(0);
        if (prevDrawn)
          ctx.lineTo(x2, y2);
        else
          ctx.moveTo(x2, y2);
        prevDrawn = true;
      } else if (empty === "gap") {
        prevDrawn = false;
      }
      continue;
    }
    const x = xOf(i);
    const y = yOf(v);
    if (prevDrawn)
      ctx.lineTo(x, y);
    else
      ctx.moveTo(x, y);
    prevDrawn = true;
  }
  ctx.stroke();
  if (group.displayXAxis && range.min < 0 && range.max > 0) {
    const y = yOf(0);
    ctx.strokeStyle = group.colorAxis ? `#${group.colorAxis}` : DEFAULT_AXIS;
    ctx.lineWidth = 0.5;
    ctx.beginPath();
    ctx.moveTo(rect.x, y);
    ctx.lineTo(rect.x + rect.w, y);
    ctx.stroke();
  }
  const markerR = Math.max(1.25, Math.min(rect.w, rect.h) * 0.08);
  if (group.markers) {
    paintLineMarkers(ctx, values, xOf, yOf, markerR, group.colorMarkers ? `#${group.colorMarkers}` : DEFAULT_MARKERS);
  }
  paintExtremaMarkers(ctx, group, values, xOf, yOf, markerR);
}
function paintLineMarkers(ctx, values, xOf, yOf, r, color) {
  ctx.fillStyle = color;
  for (let i = 0;i < values.length; i++) {
    const v = values[i];
    if (v == null)
      continue;
    ctx.beginPath();
    ctx.arc(xOf(i), yOf(v), r, 0, Math.PI * 2);
    ctx.fill();
  }
}
function paintExtremaMarkers(ctx, group, values, xOf, yOf, r) {
  const present = [];
  for (let i = 0;i < values.length; i++) {
    const v = values[i];
    if (v != null)
      present.push({ i, v });
  }
  if (present.length === 0)
    return;
  const drawDot = (i, v, color) => {
    ctx.fillStyle = color;
    ctx.beginPath();
    ctx.arc(xOf(i), yOf(v), r, 0, Math.PI * 2);
    ctx.fill();
  };
  if (group.high) {
    let hi = present[0];
    for (const p of present)
      if (p.v > hi.v)
        hi = p;
    drawDot(hi.i, hi.v, group.colorHigh ? `#${group.colorHigh}` : DEFAULT_HIGH);
  }
  if (group.low) {
    let lo = present[0];
    for (const p of present)
      if (p.v < lo.v)
        lo = p;
    drawDot(lo.i, lo.v, group.colorLow ? `#${group.colorLow}` : DEFAULT_LOW);
  }
  if (group.negative) {
    const color = group.colorNegative ? `#${group.colorNegative}` : DEFAULT_NEGATIVE;
    for (const p of present)
      if (p.v < 0)
        drawDot(p.i, p.v, color);
  }
  if (group.first) {
    const f = present[0];
    drawDot(f.i, f.v, group.colorFirst ? `#${group.colorFirst}` : DEFAULT_FIRST);
  }
  if (group.last) {
    const l = present[present.length - 1];
    drawDot(l.i, l.v, group.colorLast ? `#${group.colorLast}` : DEFAULT_LAST);
  }
}
function drawColumnSparkline(ctx, group, sp, rect) {
  const values = sp.values ?? [];
  if (values.length === 0)
    return;
  const range = resolveRange(group, values);
  const baseline = Math.max(range.min, Math.min(0, range.max));
  const yOf = (v) => {
    const t = (v - range.min) / (range.max - range.min);
    return rect.y + (1 - t) * rect.h;
  };
  const yBase = yOf(baseline);
  const total = values.length;
  const slotW = rect.w / total;
  const barW = Math.max(1, Math.floor(slotW) - 1);
  const seriesColor = group.colorSeries ? `#${group.colorSeries}` : DEFAULT_SERIES;
  const negColor = group.colorNegative ? `#${group.colorNegative}` : DEFAULT_NEGATIVE;
  let hiIdx = -1;
  let loIdx = -1;
  let firstIdx = -1;
  let lastIdx = -1;
  for (let i = 0;i < values.length; i++) {
    const v = values[i];
    if (v == null)
      continue;
    if (firstIdx === -1)
      firstIdx = i;
    lastIdx = i;
    if (hiIdx === -1 || values[hiIdx] < v)
      hiIdx = i;
    if (loIdx === -1 || values[loIdx] > v)
      loIdx = i;
  }
  for (let i = 0;i < values.length; i++) {
    const v = values[i];
    if (v == null)
      continue;
    const x = rect.x + i * slotW + Math.floor((slotW - barW) / 2);
    const yv = yOf(v);
    const top = Math.min(yBase, yv);
    const h = Math.max(1, Math.abs(yBase - yv));
    let color = seriesColor;
    if (group.negative && v < 0)
      color = negColor;
    if (group.high && i === hiIdx)
      color = group.colorHigh ? `#${group.colorHigh}` : DEFAULT_HIGH;
    if (group.low && i === loIdx)
      color = group.colorLow ? `#${group.colorLow}` : DEFAULT_LOW;
    if (group.first && i === firstIdx)
      color = group.colorFirst ? `#${group.colorFirst}` : DEFAULT_FIRST;
    if (group.last && i === lastIdx)
      color = group.colorLast ? `#${group.colorLast}` : DEFAULT_LAST;
    ctx.fillStyle = color;
    ctx.fillRect(Math.round(x), Math.round(top), barW, Math.round(h));
  }
  if (group.displayXAxis && range.min < 0 && range.max > 0) {
    ctx.strokeStyle = group.colorAxis ? `#${group.colorAxis}` : DEFAULT_AXIS;
    ctx.lineWidth = 0.5;
    ctx.beginPath();
    const y = yBase + 0.5;
    ctx.moveTo(rect.x, y);
    ctx.lineTo(rect.x + rect.w, y);
    ctx.stroke();
  }
}
function drawWinLossSparkline(ctx, group, sp, rect) {
  const values = sp.values ?? [];
  if (values.length === 0)
    return;
  const slotW = rect.w / values.length;
  const barW = Math.max(1, Math.floor(slotW) - 1);
  const halfH = Math.max(1, Math.floor(rect.h * 0.45));
  const midY = rect.y + rect.h / 2;
  const seriesColor = group.colorSeries ? `#${group.colorSeries}` : DEFAULT_SERIES;
  const negColor = group.colorNegative ? `#${group.colorNegative}` : DEFAULT_NEGATIVE;
  let firstIdx = -1;
  let lastIdx = -1;
  for (let i = 0;i < values.length; i++) {
    if (values[i] != null && values[i] !== 0) {
      if (firstIdx === -1)
        firstIdx = i;
      lastIdx = i;
    }
  }
  for (let i = 0;i < values.length; i++) {
    const v = values[i];
    if (v == null || v === 0)
      continue;
    const x = Math.round(rect.x + i * slotW + (slotW - barW) / 2);
    let color = v > 0 ? seriesColor : negColor;
    if (group.first && i === firstIdx)
      color = group.colorFirst ? `#${group.colorFirst}` : DEFAULT_FIRST;
    if (group.last && i === lastIdx)
      color = group.colorLast ? `#${group.colorLast}` : DEFAULT_LAST;
    ctx.fillStyle = color;
    if (v > 0) {
      ctx.fillRect(x, Math.round(midY - halfH), barW, halfH);
    } else {
      ctx.fillRect(x, Math.round(midY), barW, halfH);
    }
  }
  if (group.displayXAxis) {
    ctx.strokeStyle = group.colorAxis ? `#${group.colorAxis}` : DEFAULT_AXIS;
    ctx.lineWidth = 0.5;
    ctx.beginPath();
    const y = Math.round(midY) + 0.5;
    ctx.moveTo(rect.x, y);
    ctx.lineTo(rect.x + rect.w, y);
    ctx.stroke();
  }
}

// src/outlineGutter.ts
function computeOutlineRuns(sheet, g) {
  const runs = [];
  if (g.rowOutlineDepth > 0) {
    const meta = sheet.decodedRowMeta;
    const lvlByRow = new Map;
    if (meta && meta.outlineLevel.length > 0) {
      for (let i = 0;i < meta.count; i++) {
        const v = meta.outlineLevel[i] ?? 0;
        if (v > 0)
          lvlByRow.set(meta.index[i] ?? 0, v);
      }
    }
    const summaryBelow = sheet.outlinePr?.summaryBelow ?? true;
    for (let lvl = 1;lvl <= g.rowOutlineDepth; lvl++) {
      let runStart = -1;
      for (let r = 1;r <= g.maxRow + 1; r++) {
        const inRun = r <= g.maxRow && (lvlByRow.get(r) ?? 0) >= lvl;
        if (inRun && runStart < 0)
          runStart = r;
        if (!inRun && runStart >= 0) {
          const runEnd = r - 1;
          const summary = summaryBelow ? runEnd + 1 : runStart - 1;
          runs.push({ axis: "row", level: lvl, start: runStart, end: runEnd, summary });
          runStart = -1;
        }
      }
    }
  }
  if (g.colOutlineDepth > 0) {
    const lvlByCol = new Map;
    for (const c of sheet.cols) {
      const lvl = c.outlineLevel ?? 0;
      if (lvl === 0)
        continue;
      for (let i = c.min;i <= c.max; i++)
        lvlByCol.set(i, lvl);
    }
    const summaryRight = sheet.outlinePr?.summaryRight ?? true;
    for (let lvl = 1;lvl <= g.colOutlineDepth; lvl++) {
      let runStart = -1;
      for (let c = 1;c <= g.maxCol + 1; c++) {
        const inRun = c <= g.maxCol && (lvlByCol.get(c) ?? 0) >= lvl;
        if (inRun && runStart < 0)
          runStart = c;
        if (!inRun && runStart >= 0) {
          const runEnd = c - 1;
          const summary = summaryRight ? runEnd + 1 : runStart - 1;
          runs.push({ axis: "col", level: lvl, start: runStart, end: runEnd, summary });
          runStart = -1;
        }
      }
    }
  }
  return runs;
}
function isOutlineRunCollapsed(run, g) {
  if (run.axis === "row") {
    for (let r = run.start;r <= run.end; r++) {
      if ((g.rowH[r] ?? 0) > 0)
        return false;
    }
  } else {
    for (let c = run.start;c <= run.end; c++) {
      if ((g.colW[c] ?? 0) > 0)
        return false;
    }
  }
  return true;
}
function outlineButtonHits(sheet, g, view) {
  const out = [];
  if (g.rowGutterW === 0 && g.colGutterH === 0)
    return out;
  const runs = computeOutlineRuns(sheet, g);
  for (const run of runs) {
    if (run.axis === "row") {
      const sumY = g.rowY[run.summary] ?? -1;
      const sumH = g.rowH[run.summary] ?? 0;
      if (sumH <= 0)
        continue;
      const isPinned = run.summary < view.splitY;
      const cy = isPinned ? sumY + sumH / 2 : sumY + sumH / 2 - view.sy;
      if (isPinned) {
        if (cy < g.originY || cy > g.originY + view.prh)
          continue;
      } else {
        if (cy < g.originY + view.prh || cy > view.canvasH)
          continue;
      }
      const cx = rowGutterTrackX(g, run.level);
      out.push({ run, cx, cy, collapsed: isOutlineRunCollapsed(run, g) });
    } else {
      const sumX = g.colX[run.summary] ?? -1;
      const sumW = g.colW[run.summary] ?? 0;
      if (sumW <= 0)
        continue;
      const isPinned = run.summary < view.splitX;
      const cx = isPinned ? sumX + sumW / 2 : sumX + sumW / 2 - view.sx;
      if (isPinned) {
        if (cx < g.originX || cx > g.originX + view.pcw)
          continue;
      } else {
        if (cx < g.originX + view.pcw || cx > view.canvasW)
          continue;
      }
      const cy = colGutterTrackY(g, run.level);
      out.push({ run, cx, cy, collapsed: isOutlineRunCollapsed(run, g) });
    }
  }
  return out;
}
function outlineCornerHits(g) {
  const out = [];
  if (g.colOutlineDepth > 0) {
    const cx = g.rowGutterW > 0 ? (g.rowGutterW + g.originX) / 2 : g.originX - HEADER_W / 2;
    for (let lvl = 1;lvl <= g.colOutlineDepth + 1; lvl++) {
      const cy = OUTLINE_GUTTER_PAD + (lvl - 1) * OUTLINE_GUTTER_STEP + OUTLINE_GUTTER_STEP / 2;
      out.push({ axis: "col", level: lvl, cx, cy });
    }
  }
  if (g.rowOutlineDepth > 0) {
    const cy = g.colGutterH > 0 ? (g.colGutterH + g.originY) / 2 : g.originY - HEADER_H / 2;
    for (let lvl = 1;lvl <= g.rowOutlineDepth + 1; lvl++) {
      const cx = OUTLINE_GUTTER_PAD + (lvl - 1) * OUTLINE_GUTTER_STEP + OUTLINE_GUTTER_STEP / 2;
      out.push({ axis: "row", level: lvl, cx, cy });
    }
  }
  return out;
}
var OUTLINE_BUTTON_HIT_RADIUS = 7;
function drawOutlineButtons(ctx, sheet, g, view) {
  const hits = outlineButtonHits(sheet, g, view);
  if (hits.length === 0)
    return;
  ctx.save();
  for (const h of hits) {
    drawOutlineButton(ctx, h.run.axis === "row" ? h.cx - 0.5 : h.cx, h.run.axis === "row" ? h.cy : h.cy - 0.5, h.collapsed ? "+" : "-");
  }
  ctx.restore();
}
var OUTLINE_STROKE = "#9aa0a6";
var OUTLINE_BUTTON_SIZE = 10;
var OUTLINE_BUTTON_BG = "#ffffff";
var OUTLINE_BUTTON_BORDER = "#6b7280";
var OUTLINE_BUTTON_GLYPH = "#374151";
var COLLAPSED_TICK_STROKE = "#137333";
var COLLAPSED_TICK_WIDTH = 2;
function drawCollapsedRowTicks(ctx, g, sy, splitY, prh, canvasH, rowScrollVis) {
  const xLeft = g.rowGutterW;
  const xRight = g.originX;
  ctx.save();
  ctx.strokeStyle = COLLAPSED_TICK_STROKE;
  ctx.lineWidth = COLLAPSED_TICK_WIDTH;
  const paintTick = (yTop, clipY1, clipY2) => {
    if (yTop < clipY1 || yTop > clipY2)
      return;
    const y = yTop + COLLAPSED_TICK_WIDTH / 2;
    ctx.beginPath();
    ctx.moveTo(xLeft, y);
    ctx.lineTo(xRight, y);
    ctx.stroke();
  };
  if (splitY > 1) {
    ctx.save();
    ctx.beginPath();
    ctx.rect(xLeft, g.originY, HEADER_W, prh);
    ctx.clip();
    for (let r = 2;r < splitY; r++) {
      if ((g.rowH[r] ?? 0) <= 0)
        continue;
      if ((g.rowH[r - 1] ?? 0) > 0)
        continue;
      paintTick(g.rowY[r] ?? 0, g.originY, g.originY + prh);
    }
    ctx.restore();
  }
  ctx.save();
  ctx.beginPath();
  ctx.rect(xLeft, g.originY + prh, HEADER_W, canvasH - g.originY - prh);
  ctx.clip();
  const first = Math.max(splitY, rowScrollVis.firstRow);
  for (let r = Math.max(2, first);r <= rowScrollVis.lastRow; r++) {
    if ((g.rowH[r] ?? 0) <= 0)
      continue;
    if ((g.rowH[r - 1] ?? 0) > 0)
      continue;
    paintTick((g.rowY[r] ?? 0) - sy, g.originY + prh, canvasH);
  }
  ctx.restore();
  ctx.restore();
}
function drawCollapsedColTicks(ctx, g, sx, splitX, pcw, canvasW, colScrollVis) {
  const yTop = g.colGutterH;
  const yBot = g.originY;
  ctx.save();
  ctx.strokeStyle = COLLAPSED_TICK_STROKE;
  ctx.lineWidth = COLLAPSED_TICK_WIDTH;
  const paintTick = (xLeft, clipX1, clipX2) => {
    if (xLeft < clipX1 || xLeft > clipX2)
      return;
    const x = xLeft + COLLAPSED_TICK_WIDTH / 2;
    ctx.beginPath();
    ctx.moveTo(x, yTop);
    ctx.lineTo(x, yBot);
    ctx.stroke();
  };
  if (splitX > 1) {
    ctx.save();
    ctx.beginPath();
    ctx.rect(g.originX, yTop, pcw, HEADER_H);
    ctx.clip();
    for (let c = 2;c < splitX; c++) {
      if ((g.colW[c] ?? 0) <= 0)
        continue;
      if ((g.colW[c - 1] ?? 0) > 0)
        continue;
      paintTick(g.colX[c] ?? 0, g.originX, g.originX + pcw);
    }
    ctx.restore();
  }
  ctx.save();
  ctx.beginPath();
  ctx.rect(g.originX + pcw, yTop, canvasW - g.originX - pcw, HEADER_H);
  ctx.clip();
  const first = Math.max(splitX, colScrollVis.firstCol);
  for (let c = Math.max(2, first);c <= colScrollVis.lastCol; c++) {
    if ((g.colW[c] ?? 0) <= 0)
      continue;
    if ((g.colW[c - 1] ?? 0) > 0)
      continue;
    paintTick((g.colX[c] ?? 0) - sx, g.originX + pcw, canvasW);
  }
  ctx.restore();
  ctx.restore();
}
function rowGutterTrackX(g, lvl) {
  return OUTLINE_GUTTER_PAD + (lvl - 1) * OUTLINE_GUTTER_STEP + OUTLINE_GUTTER_STEP / 2;
}
function colGutterTrackY(g, lvl) {
  return OUTLINE_GUTTER_PAD + (lvl - 1) * OUTLINE_GUTTER_STEP + OUTLINE_GUTTER_STEP / 2;
}
function drawOutlineButton(ctx, cx, cy, glyph) {
  const s = OUTLINE_BUTTON_SIZE;
  const x = Math.round(cx - s / 2) + 0.5;
  const y = Math.round(cy - s / 2) + 0.5;
  ctx.fillStyle = OUTLINE_BUTTON_BG;
  ctx.fillRect(x, y, s - 1, s - 1);
  ctx.strokeStyle = OUTLINE_BUTTON_BORDER;
  ctx.lineWidth = 1;
  ctx.strokeRect(x, y, s - 1, s - 1);
  ctx.strokeStyle = OUTLINE_BUTTON_GLYPH;
  ctx.beginPath();
  const mx1 = x + 2;
  const mx2 = x + s - 3;
  const my = y + (s - 1) / 2;
  ctx.moveTo(mx1, my);
  ctx.lineTo(mx2, my);
  if (glyph === "+") {
    const mvy1 = y + 2;
    const mvy2 = y + s - 3;
    const mvx = x + (s - 1) / 2;
    ctx.moveTo(mvx, mvy1);
    ctx.lineTo(mvx, mvy2);
  }
  ctx.stroke();
}
function drawOutlineCornerButtons(ctx, g) {
  ctx.save();
  ctx.font = '10px -apple-system, BlinkMacSystemFont, "Segoe UI", Arial, sans-serif';
  ctx.textBaseline = "middle";
  ctx.textAlign = "center";
  const paintNumeral = (cx, cy, n) => {
    const s = OUTLINE_BUTTON_SIZE;
    const x = Math.round(cx - s / 2) + 0.5;
    const y = Math.round(cy - s / 2) + 0.5;
    ctx.fillStyle = OUTLINE_BUTTON_BG;
    ctx.fillRect(x, y, s - 1, s - 1);
    ctx.strokeStyle = OUTLINE_BUTTON_BORDER;
    ctx.lineWidth = 1;
    ctx.strokeRect(x, y, s - 1, s - 1);
    ctx.fillStyle = OUTLINE_BUTTON_GLYPH;
    ctx.fillText(String(n), cx, cy + 0.5);
  };
  for (const h of outlineCornerHits(g))
    paintNumeral(h.cx, h.cy, h.level);
  ctx.restore();
}
function drawRowOutlineGutter(ctx, sheet, g, sy, splitY, prh, canvasH) {
  const meta = sheet.decodedRowMeta;
  if (meta.outlineLevel.length === 0)
    return;
  const lvlByRow = new Map;
  for (let i = 0;i < meta.count; i++) {
    const v = meta.outlineLevel[i] ?? 0;
    if (v > 0)
      lvlByRow.set(meta.index[i] ?? 0, v);
  }
  const summaryBelow = sheet.outlinePr?.summaryBelow ?? true;
  ctx.save();
  ctx.strokeStyle = OUTLINE_STROKE;
  ctx.lineWidth = 1;
  for (let lvl = 1;lvl <= g.rowOutlineDepth; lvl++) {
    const x = rowGutterTrackX(g, lvl) + 0.5;
    paintRowRunsForLevel(ctx, lvlByRow, lvl, x, g, summaryBelow, 1, Math.max(0, splitY - 1), 0, g.originY, g.originY + prh);
    paintRowRunsForLevel(ctx, lvlByRow, lvl, x, g, summaryBelow, Math.max(1, splitY), g.maxRow, -sy, g.originY + prh, canvasH);
  }
  ctx.restore();
}
function paintRowRunsForLevel(ctx, lvlByRow, lvl, xLine, g, summaryBelow, rowFrom, rowTo, offsetY, clipY1, clipY2) {
  if (rowTo < rowFrom)
    return;
  ctx.save();
  ctx.beginPath();
  ctx.rect(0, clipY1, g.rowGutterW, clipY2 - clipY1);
  ctx.clip();
  let runStart = -1;
  for (let r = rowFrom;r <= rowTo + 1; r++) {
    const inRun = r <= rowTo && (lvlByRow.get(r) ?? 0) >= lvl;
    if (inRun && runStart < 0)
      runStart = r;
    if (!inRun && runStart >= 0) {
      const runEnd = r - 1;
      const y1 = (g.rowY[runStart] ?? g.originY) + offsetY;
      const y2 = (g.rowY[runEnd + 1] ?? g.originY) + offsetY;
      if (y2 - y1 < 3) {
        runStart = -1;
        continue;
      }
      if (y2 > clipY1 && y1 < clipY2) {
        const hookY = summaryBelow ? y1 : y2;
        ctx.beginPath();
        ctx.moveTo(xLine, y1);
        ctx.lineTo(xLine, y2);
        ctx.moveTo(xLine, hookY);
        ctx.lineTo(xLine + 3, hookY);
        ctx.stroke();
      }
      runStart = -1;
    }
  }
  ctx.restore();
}
function drawColOutlineGutter(ctx, sheet, g, sx, splitX, pcw, canvasW) {
  if (g.colOutlineDepth === 0)
    return;
  const lvlByCol = new Map;
  for (const c of sheet.cols) {
    const lvl = c.outlineLevel ?? 0;
    if (lvl === 0)
      continue;
    for (let i = c.min;i <= c.max; i++)
      lvlByCol.set(i, lvl);
  }
  const summaryRight = sheet.outlinePr?.summaryRight ?? true;
  ctx.save();
  ctx.strokeStyle = OUTLINE_STROKE;
  ctx.lineWidth = 1;
  for (let lvl = 1;lvl <= g.colOutlineDepth; lvl++) {
    const y = colGutterTrackY(g, lvl) + 0.5;
    paintColRunsForLevel(ctx, lvlByCol, lvl, y, g, summaryRight, 1, Math.max(0, splitX - 1), 0, g.originX, g.originX + pcw);
    paintColRunsForLevel(ctx, lvlByCol, lvl, y, g, summaryRight, Math.max(1, splitX), g.maxCol, -sx, g.originX + pcw, canvasW);
  }
  ctx.restore();
}
function paintColRunsForLevel(ctx, lvlByCol, lvl, yLine, g, summaryRight, colFrom, colTo, offsetX, clipX1, clipX2) {
  if (colTo < colFrom)
    return;
  ctx.save();
  ctx.beginPath();
  ctx.rect(clipX1, 0, clipX2 - clipX1, g.colGutterH);
  ctx.clip();
  let runStart = -1;
  for (let c = colFrom;c <= colTo + 1; c++) {
    const inRun = c <= colTo && (lvlByCol.get(c) ?? 0) >= lvl;
    if (inRun && runStart < 0)
      runStart = c;
    if (!inRun && runStart >= 0) {
      const runEnd = c - 1;
      const x1 = (g.colX[runStart] ?? g.originX) + offsetX;
      const x2 = (g.colX[runEnd + 1] ?? g.originX) + offsetX;
      if (x2 - x1 < 3) {
        runStart = -1;
        continue;
      }
      if (x2 > clipX1 && x1 < clipX2) {
        const hookX = summaryRight ? x1 : x2;
        ctx.beginPath();
        ctx.moveTo(x1, yLine);
        ctx.lineTo(x2, yLine);
        ctx.moveTo(hookX, yLine);
        ctx.lineTo(hookX, yLine + 3);
        ctx.stroke();
      }
      runStart = -1;
    }
  }
  ctx.restore();
}

// src/sheetChrome.ts
function tableAccentHex(styleName) {
  let n = 2;
  if (styleName) {
    const m = styleName.match(/(\d+)$/);
    if (m)
      n = parseInt(m[1], 10);
  }
  const idx = ((n - 2) % 6 + 6) % 6;
  return activeThemeColor(4 + idx, "#4472c4");
}
function mixHex(hex, other, t) {
  const h = hex.startsWith("#") ? hex.slice(1) : hex;
  const o = other.startsWith("#") ? other.slice(1) : other;
  const r1 = parseInt(h.slice(0, 2), 16), g1 = parseInt(h.slice(2, 4), 16), b1 = parseInt(h.slice(4, 6), 16);
  const r2 = parseInt(o.slice(0, 2), 16), g2 = parseInt(o.slice(2, 4), 16), b2 = parseInt(o.slice(4, 6), 16);
  const r = Math.round(r1 + (r2 - r1) * t);
  const g = Math.round(g1 + (g2 - g1) * t);
  const b = Math.round(b1 + (b2 - b1) * t);
  const toHex = (v) => v.toString(16).padStart(2, "0");
  return "#" + toHex(r) + toHex(g) + toHex(b);
}
function computeTableState(sheet, vis) {
  const tableDxfs = new Map;
  const filterArrows = new Set;
  const tables = sheet.tables ?? [];
  const pivots = sheet.pivots ?? [];
  if (tables.length === 0 && pivots.length === 0) {
    return { tableDxfs, filterArrows };
  }
  for (const t of tables) {
    const accent = tableAccentHex(t.style?.name);
    const bandHex = mixHex("#ffffff", accent, 0.12);
    const accentColor = { rgb: accent.slice(1).toUpperCase() };
    const bandColor = { rgb: bandHex.slice(1).toUpperCase() };
    const whiteColor = { rgb: "FFFFFF" };
    const headerRows = t.headerRowCount;
    const totalsRows = t.totalsRowCount;
    const r1 = t.range.r1, r2 = t.range.r2;
    const c1 = t.range.c1, c2 = t.range.c2;
    const headerR = headerRows > 0 ? r1 : -1;
    const dataStart = r1 + headerRows;
    const dataEnd = r2 - totalsRows;
    if (headerR >= 0) {
      const hc1 = Math.max(c1, vis?.firstCol ?? c1);
      const hc2 = Math.min(c2, vis?.lastCol ?? c2);
      for (let c = hc1;c <= hc2; c++) {
        const k = `${headerR}:${c}`;
        if (!vis || headerR >= vis.firstRow && headerR <= vis.lastRow) {
          tableDxfs.set(k, {
            fillColor: accentColor,
            fontColor: whiteColor,
            bold: true
          });
        }
        if (t.hasAutoFilter)
          filterArrows.add(k);
      }
    }
    if (t.style?.showRowStripes !== false) {
      const rr1 = Math.max(dataStart, vis?.firstRow ?? dataStart);
      const rr2 = Math.min(dataEnd, vis?.lastRow ?? dataEnd);
      const cc1 = Math.max(c1, vis?.firstCol ?? c1);
      const cc2 = Math.min(c2, vis?.lastCol ?? c2);
      for (let r = rr1;r <= rr2; r++) {
        const isOdd = (r - dataStart & 1) === 1;
        if (!isOdd)
          continue;
        for (let c = cc1;c <= cc2; c++) {
          const k = `${r}:${c}`;
          if (tableDxfs.has(k))
            continue;
          tableDxfs.set(k, { fillColor: bandColor });
        }
      }
    }
    if (totalsRows > 0) {
      const totalsR = r2;
      if (vis && (totalsR < vis.firstRow || totalsR > vis.lastRow))
        continue;
      const tc1 = Math.max(c1, vis?.firstCol ?? c1);
      const tc2 = Math.min(c2, vis?.lastCol ?? c2);
      for (let c = tc1;c <= tc2; c++) {
        const k = `${totalsR}:${c}`;
        if (tableDxfs.has(k))
          continue;
        tableDxfs.set(k, { fillColor: bandColor, bold: true });
      }
    }
  }
  for (const p of pivots) {
    for (const cell of p.filterArrowCells) {
      filterArrows.add(`${cell.r}:${cell.c}`);
    }
  }
  return { tableDxfs, filterArrows };
}
function drawFilterArrows(ctx, sheet, g, vis, filterArrows) {
  if (filterArrows.size === 0)
    return;
  const BOX_W = 14, BOX_H = 14, INSET_X = 4;
  for (const k of filterArrows) {
    const [rs, cs] = k.split(":");
    const r = parseInt(rs, 10), c = parseInt(cs, 10);
    if (r < vis.firstRow || r > vis.lastRow)
      continue;
    if (c < vis.firstCol || c > vis.lastCol)
      continue;
    const rect = cellRect(g, r, c);
    const x = rect.x + rect.w - BOX_W - INSET_X;
    const y = rect.y + (rect.h - BOX_H) / 2;
    ctx.fillStyle = "rgba(255, 255, 255, 0.85)";
    ctx.fillRect(x, y, BOX_W, BOX_H);
    ctx.strokeStyle = "rgba(0, 0, 0, 0.25)";
    ctx.lineWidth = 1;
    ctx.strokeRect(x + 0.5, y + 0.5, BOX_W - 1, BOX_H - 1);
    ctx.fillStyle = "#374151";
    ctx.beginPath();
    const ax = x + BOX_W / 2;
    const ay = y + BOX_H / 2 + 2;
    ctx.moveTo(ax - 4, ay - 2);
    ctx.lineTo(ax + 4, ay - 2);
    ctx.lineTo(ax, ay + 3);
    ctx.closePath();
    ctx.fill();
  }
}
function drawHeaders(ctx, sheet, g, sel, vp, canvasW, canvasH, panes) {
  const sx = vp ? vp.x : 0;
  const sy = vp ? vp.y : 0;
  const { splitX, splitY, pcw, prh } = frozenDims(sheet, g);
  const scrollPane = panes.find((p) => p.kind === "br");
  const topPinPane = panes.find((p) => p.kind === "tr");
  const leftPinPane = panes.find((p) => p.kind === "bl");
  const colScrollVis = (topPinPane ?? scrollPane).vis;
  const rowScrollVis = (leftPinPane ?? scrollPane).vis;
  const headerLeft = g.rowGutterW;
  const headerTop = g.colGutterH;
  const originX = g.originX;
  const originY = g.originY;
  ctx.save();
  ctx.fillStyle = HEADER_BG;
  ctx.fillRect(0, 0, canvasW, originY);
  ctx.fillRect(0, 0, originX, canvasH);
  ctx.strokeStyle = HEADER_BORDER;
  ctx.lineWidth = 1;
  ctx.save();
  ctx.beginPath();
  ctx.rect(originX, headerTop, canvasW - originX, HEADER_H);
  ctx.clip();
  ctx.beginPath();
  for (let c = 2;c < splitX; c++) {
    const x = Math.round(g.colX[c] ?? 0) + 0.5;
    ctx.moveTo(x, headerTop);
    ctx.lineTo(x, originY);
  }
  const firstScrollCol = Math.max(splitX, colScrollVis.firstCol);
  for (let c = Math.max(2, firstScrollCol);c <= colScrollVis.lastCol + 1; c++) {
    const x = Math.round((g.colX[c] ?? 0) - sx) + 0.5;
    if (x < originX + pcw)
      continue;
    ctx.moveTo(x, headerTop);
    ctx.lineTo(x, originY);
  }
  ctx.stroke();
  ctx.restore();
  ctx.save();
  ctx.beginPath();
  ctx.rect(headerLeft, originY, HEADER_W, canvasH - originY);
  ctx.clip();
  ctx.beginPath();
  for (let r = 2;r < splitY; r++) {
    const y = Math.round(g.rowY[r] ?? 0) + 0.5;
    ctx.moveTo(headerLeft, y);
    ctx.lineTo(originX, y);
  }
  const firstScrollRow = Math.max(splitY, rowScrollVis.firstRow);
  for (let r = Math.max(2, firstScrollRow);r <= rowScrollVis.lastRow + 1; r++) {
    const y = Math.round((g.rowY[r] ?? 0) - sy) + 0.5;
    if (y < originY + prh)
      continue;
    ctx.moveTo(headerLeft, y);
    ctx.lineTo(originX, y);
  }
  ctx.stroke();
  ctx.restore();
  if (sel) {
    ctx.fillStyle = HEADER_HIGHLIGHT;
    const cAbsX1 = g.colX[sel.c1] ?? 0;
    const cAbsX2 = g.colX[sel.c2 + 1] ?? cAbsX1;
    if (cAbsX2 > cAbsX1) {
      if (sel.c1 < splitX) {
        const x1 = cAbsX1;
        const x2 = Math.min(cAbsX2, g.colX[splitX] ?? cAbsX2);
        const cx1 = Math.max(originX, x1);
        const cx2 = Math.min(originX + pcw, x2);
        if (cx2 > cx1)
          ctx.fillRect(cx1, headerTop, cx2 - cx1, HEADER_H);
      }
      if (sel.c2 >= splitX) {
        const x1 = Math.max(cAbsX1, g.colX[splitX] ?? cAbsX1) - sx;
        const x2 = cAbsX2 - sx;
        const cx1 = Math.max(originX + pcw, x1);
        const cx2 = Math.min(canvasW, x2);
        if (cx2 > cx1)
          ctx.fillRect(cx1, headerTop, cx2 - cx1, HEADER_H);
      }
    }
    const rAbsY1 = g.rowY[sel.r1] ?? 0;
    const rAbsY2 = g.rowY[sel.r2 + 1] ?? rAbsY1;
    if (rAbsY2 > rAbsY1) {
      if (sel.r1 < splitY) {
        const y1 = rAbsY1;
        const y2 = Math.min(rAbsY2, g.rowY[splitY] ?? rAbsY2);
        const cy1 = Math.max(originY, y1);
        const cy2 = Math.min(originY + prh, y2);
        if (cy2 > cy1)
          ctx.fillRect(headerLeft, cy1, HEADER_W, cy2 - cy1);
      }
      if (sel.r2 >= splitY) {
        const y1 = Math.max(rAbsY1, g.rowY[splitY] ?? rAbsY1) - sy;
        const y2 = rAbsY2 - sy;
        const cy1 = Math.max(originY + prh, y1);
        const cy2 = Math.min(canvasH, y2);
        if (cy2 > cy1)
          ctx.fillRect(headerLeft, cy1, HEADER_W, cy2 - cy1);
      }
    }
  }
  ctx.strokeStyle = GUTTER_LINE;
  ctx.lineWidth = 2;
  ctx.beginPath();
  ctx.moveTo(0, originY);
  ctx.lineTo(canvasW, originY);
  ctx.moveTo(originX, 0);
  ctx.lineTo(originX, canvasH);
  ctx.stroke();
  if (g.rowGutterW > 0 || g.colGutterH > 0) {
    ctx.strokeStyle = HEADER_BORDER;
    ctx.lineWidth = 1;
    ctx.beginPath();
    if (g.rowGutterW > 0) {
      const x = headerLeft + 0.5;
      ctx.moveTo(x, originY);
      ctx.lineTo(x, canvasH);
    }
    if (g.colGutterH > 0) {
      const y = headerTop + 0.5;
      ctx.moveTo(originX, y);
      ctx.lineTo(canvasW, y);
    }
    ctx.stroke();
  }
  ctx.fillStyle = HEADER_FG;
  ctx.font = '11px -apple-system, BlinkMacSystemFont, "Segoe UI", Arial, sans-serif';
  ctx.textBaseline = "middle";
  ctx.textAlign = "center";
  const colLabelMidY = headerTop + HEADER_H / 2;
  if (splitX > 1) {
    ctx.save();
    ctx.beginPath();
    ctx.rect(originX, headerTop, pcw, HEADER_H);
    ctx.clip();
    for (let c = 1;c < splitX; c++) {
      const w = g.colW[c] ?? 0;
      if (w <= 0)
        continue;
      const x = (g.colX[c] ?? 0) + w / 2;
      ctx.fillText(colLabel(c), x, colLabelMidY);
    }
    ctx.restore();
  }
  ctx.save();
  ctx.beginPath();
  ctx.rect(originX + pcw, headerTop, canvasW - originX - pcw, HEADER_H);
  ctx.clip();
  for (let c = Math.max(splitX, colScrollVis.firstCol);c <= colScrollVis.lastCol; c++) {
    const w = g.colW[c] ?? 0;
    if (w <= 0)
      continue;
    const x = (g.colX[c] ?? 0) + w / 2 - sx;
    ctx.fillText(colLabel(c), x, colLabelMidY);
  }
  ctx.restore();
  const rowLabelMidX = headerLeft + HEADER_W / 2;
  if (splitY > 1) {
    ctx.save();
    ctx.beginPath();
    ctx.rect(headerLeft, originY, HEADER_W, prh);
    ctx.clip();
    for (let r = 1;r < splitY; r++) {
      const h = g.rowH[r] ?? 0;
      if (h <= 0)
        continue;
      const y = (g.rowY[r] ?? 0) + h / 2;
      ctx.fillText(String(r), rowLabelMidX, y);
    }
    ctx.restore();
  }
  ctx.save();
  ctx.beginPath();
  ctx.rect(headerLeft, originY + prh, HEADER_W, canvasH - originY - prh);
  ctx.clip();
  for (let r = Math.max(splitY, rowScrollVis.firstRow);r <= rowScrollVis.lastRow; r++) {
    const h = g.rowH[r] ?? 0;
    if (h <= 0)
      continue;
    const y = (g.rowY[r] ?? 0) + h / 2 - sy;
    ctx.fillText(String(r), rowLabelMidX, y);
  }
  ctx.restore();
  drawCollapsedRowTicks(ctx, g, sy, splitY, prh, canvasH, rowScrollVis);
  drawCollapsedColTicks(ctx, g, sx, splitX, pcw, canvasW, colScrollVis);
  if (g.rowGutterW > 0 || g.colGutterH > 0) {
    drawOutlineCornerButtons(ctx, g);
  }
  if (g.rowGutterW > 0) {
    drawRowOutlineGutter(ctx, sheet, g, sy, splitY, prh, canvasH);
  }
  if (g.colGutterH > 0) {
    drawColOutlineGutter(ctx, sheet, g, sx, splitX, pcw, canvasW);
  }
  if (g.rowGutterW > 0 || g.colGutterH > 0) {
    drawOutlineButtons(ctx, sheet, g, {
      sx,
      sy,
      splitX,
      splitY,
      pcw,
      prh,
      canvasW,
      canvasH
    });
  }
  ctx.textAlign = "start";
  ctx.textBaseline = "alphabetic";
  ctx.restore();
}
function computeHyperlinkDxfs(sheet, layout) {
  const out = new Map;
  const hyperlinks = sheet.hyperlinks ?? [];
  if (hyperlinks.length === 0)
    return out;
  const hlinkColor = { theme: 10 };
  for (const h of hyperlinks) {
    const { r1, c1, r2, c2 } = h.range;
    for (let r = r1;r <= r2; r++) {
      for (let c = c1;c <= c2; c++) {
        const k = `${r}:${c}`;
        if (out.has(k))
          continue;
        const cell = findCell(sheet, r, c);
        if (cell && cell.styleIndex !== undefined) {
          const xf = layout.styles.cellXfs[cell.styleIndex];
          if (xf && xf.fontId !== undefined && xf.fontId !== 0)
            continue;
        }
        out.set(k, { fontColor: hlinkColor, underline: true });
      }
    }
  }
  return out;
}
function drawCommentMarkers(ctx, sheet, g, vis) {
  const comments = sheet.comments ?? [];
  if (comments.length === 0)
    return;
  const { topLeftOf } = buildMergeMaps(sheet);
  const SIZE = 6;
  ctx.save();
  ctx.fillStyle = "#C81E1E";
  for (const cmt of comments) {
    if (cmt.r < vis.firstRow || cmt.r > vis.lastRow)
      continue;
    if (cmt.c < vis.firstCol || cmt.c > vis.lastCol)
      continue;
    const k = `${cmt.r}:${cmt.c}`;
    const rect = topLeftOf.has(k) ? mergedRect(g, topLeftOf.get(k)) : cellRect(g, cmt.r, cmt.c);
    const x2 = rect.x + rect.w;
    const y1 = rect.y;
    ctx.beginPath();
    ctx.moveTo(x2 - SIZE, y1);
    ctx.lineTo(x2, y1);
    ctx.lineTo(x2, y1 + SIZE);
    ctx.closePath();
    ctx.fill();
  }
  ctx.restore();
}

// src/render.ts
function render(canvas, sheet, layout, opts = {}) {
  const ctx = canvas.getContext("2d");
  if (!ctx)
    throw new Error("no 2d context");
  setActiveTheme(layout.theme);
  const renderHeaders = opts.renderHeaders ?? true;
  const dpr = opts.scale ?? (typeof window !== "undefined" ? window.devicePixelRatio || 1 : 1);
  const zoom = opts.zoom ?? 1;
  const vp = opts.viewport;
  const requiredFarX = vp ? vp.x + vp.w : undefined;
  const requiredFarY = vp ? vp.y + vp.h : undefined;
  const grid = buildGrid(sheet, opts.colOverrides, opts.rowOverrides, requiredFarX, requiredFarY);
  const W = vp ? vp.w : grid.totalW;
  const H = vp ? vp.h : grid.totalH;
  const total = zoom * dpr;
  const pixelW = Math.ceil(W * total);
  const pixelH = Math.ceil(H * total);
  if (canvas.width !== pixelW)
    canvas.width = pixelW;
  if (canvas.height !== pixelH)
    canvas.height = pixelH;
  if ("style" in canvas && canvas.style) {
    canvas.style.width = `${W * zoom}px`;
    canvas.style.height = `${H * zoom}px`;
  }
  ctx.setTransform(total, 0, 0, total, 0, 0);
  ctx.fillStyle = "#ffffff";
  ctx.fillRect(0, 0, W, H);
  const sel = resolveSelection(opts, grid);
  const panes = splitPanes(sheet, grid, vp ?? null, W, H);
  const cfLocks = computeCfStopLocks(sheet, layout);
  const cfDxfs = computeCfDxfMap(sheet, layout, cfLocks);
  const cfTextSuppress = computeCfTextSuppress(sheet, cfLocks);
  const { cfIconReserve, cfIconDraw, cfIconSuppress } = computeCfIconState(sheet, cfLocks);
  for (const k of cfIconSuppress)
    cfTextSuppress.add(k);
  const { tableDxfs, filterArrows } = computeTableState(sheet, visibleEnvelope(panes));
  for (const [k, dxf] of tableDxfs) {
    if (!cfDxfs.has(k))
      cfDxfs.set(k, dxf);
  }
  const hyperlinkDxfs = computeHyperlinkDxfs(sheet, layout);
  for (const [k, dxf] of hyperlinkDxfs) {
    if (!cfDxfs.has(k))
      cfDxfs.set(k, dxf);
  }
  for (const pane of panes) {
    ctx.save();
    ctx.beginPath();
    ctx.rect(pane.cx, pane.cy, pane.cw, pane.ch);
    ctx.clip();
    ctx.translate(pane.tx, pane.ty);
    drawGridLines(ctx, sheet, grid, pane.vis);
    drawDefaultFills(ctx, sheet, layout, grid, pane.vis);
    drawCellBackgrounds(ctx, sheet, layout, grid, pane.vis);
    drawConditionalFormats(ctx, sheet, layout, grid, pane.vis, cfDxfs, cfLocks);
    drawCellBorders(ctx, sheet, layout, grid, pane.vis);
    drawCellText(ctx, sheet, layout, grid, pane.vis, cfDxfs, cfTextSuppress, cfIconReserve);
    drawCfIcons(ctx, sheet, grid, pane.vis, cfIconDraw);
    drawSparklines(ctx, sheet, grid, pane.vis);
    drawFilterArrows(ctx, sheet, grid, pane.vis, filterArrows);
    drawDrawings(ctx, sheet, grid);
    drawCommentMarkers(ctx, sheet, grid, pane.vis);
    if (sel)
      drawSelection(ctx, sheet, grid, sel, opts.activeCell ?? null);
    ctx.restore();
  }
  drawFreezeIndicators(ctx, sheet, grid, W, H);
  if (renderHeaders)
    drawHeaders(ctx, sheet, grid, sel, vp ?? null, W, H, panes);
}
function visibleEnvelope(panes) {
  let firstRow = Infinity;
  let lastRow = 0;
  let firstCol = Infinity;
  let lastCol = 0;
  for (const pane of panes) {
    firstRow = Math.min(firstRow, pane.vis.firstRow);
    lastRow = Math.max(lastRow, pane.vis.lastRow);
    firstCol = Math.min(firstCol, pane.vis.firstCol);
    lastCol = Math.max(lastCol, pane.vis.lastCol);
  }
  return {
    firstRow: Number.isFinite(firstRow) ? firstRow : 1,
    lastRow: Math.max(lastRow, 1),
    firstCol: Number.isFinite(firstCol) ? firstCol : 1,
    lastCol: Math.max(lastCol, 1)
  };
}

// src/interact.ts
var RESIZE_TOL = 4;
var MIN_COL_W = 8;
var MIN_ROW_H = 4;
var ZOOM_MIN = 0.25;
var ZOOM_MAX = 4;
function attachInteractivity(canvas, opts) {
  let drag = null;
  const savedCursor = canvas.style.cursor;
  let cachedGrid = null;
  function invalidateGrid() {
    cachedGrid = null;
  }
  function getGrid() {
    const sheet = opts.getSheet();
    if (cachedGrid && cachedGrid.sheet === sheet && cachedGrid.colOverrides === opts.colOverrides && cachedGrid.rowOverrides === opts.rowOverrides) {
      return cachedGrid.grid;
    }
    const grid = buildGrid(sheet, opts.colOverrides, opts.rowOverrides);
    cachedGrid = { sheet, colOverrides: opts.colOverrides, rowOverrides: opts.rowOverrides, grid };
    return grid;
  }
  let mapsForSheet = null;
  let hyperlinkMap = new Map;
  let commentMap = new Map;
  function ensureMaps() {
    const sheet = opts.getSheet();
    if (mapsForSheet === sheet)
      return;
    mapsForSheet = sheet;
    hyperlinkMap = new Map;
    commentMap = new Map;
    for (const h of sheet.hyperlinks ?? []) {
      for (let r = h.range.r1;r <= h.range.r2; r++) {
        for (let c = h.range.c1;c <= h.range.c2; c++) {
          hyperlinkMap.set(`${r}:${c}`, h);
        }
      }
    }
    for (const cmt of sheet.comments ?? []) {
      commentMap.set(`${cmt.r}:${cmt.c}`, cmt);
    }
  }
  function resolveAnchor(r, c) {
    const sheet = opts.getSheet();
    for (const m of sheet.merges) {
      if (r >= m.r1 && r <= m.r2 && c >= m.c1 && c <= m.c2)
        return { r: m.r1, c: m.c1 };
    }
    return { r, c };
  }
  let popoverEl = null;
  function ensurePopover() {
    if (popoverEl)
      return popoverEl;
    const el = document.createElement("div");
    el.setAttribute("data-xlcore", "comment-popover");
    el.style.cssText = [
      "position: fixed",
      "z-index: 10000",
      "max-width: 280px",
      "padding: 6px 10px",
      "background: #fffbcb",
      "border: 1px solid #c0a060",
      "box-shadow: 2px 2px 6px rgba(0,0,0,0.18)",
      "font: 12px -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif",
      "color: #111",
      "white-space: pre-wrap",
      "pointer-events: none",
      "display: none"
    ].join("; ");
    document.body.appendChild(el);
    popoverEl = el;
    return el;
  }
  function hidePopover() {
    if (popoverEl)
      popoverEl.style.display = "none";
  }
  function showPopover(cmt, anchorClient) {
    const el = ensurePopover();
    el.textContent = "";
    if (cmt.author) {
      const a = document.createElement("div");
      a.style.cssText = "font-weight: 600; margin-bottom: 2px;";
      a.textContent = cmt.author;
      el.appendChild(a);
    }
    const body = document.createElement("div");
    body.textContent = cmt.text;
    el.appendChild(body);
    el.style.display = "block";
    const popW = el.offsetWidth;
    const popH = el.offsetHeight;
    let x = anchorClient.right + 6;
    let y = anchorClient.top;
    if (x + popW > window.innerWidth - 4)
      x = anchorClient.left - popW - 6;
    if (y + popH > window.innerHeight - 4)
      y = window.innerHeight - popH - 4;
    if (y < 4)
      y = 4;
    el.style.left = x + "px";
    el.style.top = y + "px";
  }
  function cellAtLogical(p) {
    const grid = getGrid();
    if (p.x < grid.originX || p.y < grid.originY)
      return null;
    return cellAt(grid, p.x, p.y);
  }
  function toCanvasLocal(ev) {
    const r = canvas.getBoundingClientRect();
    const z = opts.zoom.get();
    return {
      x: (ev.clientX - r.left) / z,
      y: (ev.clientY - r.top) / z
    };
  }
  function toLogical(ev) {
    const p = toCanvasLocal(ev);
    const vp = opts.getViewport?.() ?? null;
    const sheet = opts.getSheet();
    const grid = getGrid();
    const { pcw, prh } = frozenDims(sheet, grid);
    const sx = vp && p.x > grid.originX + pcw ? vp.x : 0;
    const sy = vp && p.y > grid.originY + prh ? vp.y : 0;
    return { x: p.x + sx, y: p.y + sy };
  }
  function hitTest(cx, cy) {
    const vp = opts.getViewport?.() ?? null;
    const sx = vp?.x ?? 0;
    const sy = vp?.y ?? 0;
    const sheet = opts.getSheet();
    const grid = getGrid();
    const { splitX, splitY, pcw, prh } = frozenDims(sheet, grid);
    if (cy >= grid.colGutterH && cy <= grid.originY && cx > grid.originX) {
      if (cx <= grid.originX + pcw) {
        const edgeIndex = nearestEdgeIndex(grid.colX, cx, 2, splitX);
        if (edgeIndex !== null) {
          return { kind: "col", index: edgeIndex - 1, edgeX: grid.colX[edgeIndex] ?? 0 };
        }
      } else {
        const x = cx + sx;
        const edgeIndex = nearestEdgeIndex(grid.colX, x, Math.max(splitX + 1, 2), grid.maxCol + 1);
        if (edgeIndex !== null) {
          return { kind: "col", index: edgeIndex - 1, edgeX: grid.colX[edgeIndex] ?? 0 };
        }
      }
    }
    if (cx >= grid.rowGutterW && cx <= grid.originX && cy > grid.originY) {
      if (cy <= grid.originY + prh) {
        const edgeIndex = nearestEdgeIndex(grid.rowY, cy, 2, splitY);
        if (edgeIndex !== null) {
          return { kind: "row", index: edgeIndex - 1, edgeY: grid.rowY[edgeIndex] ?? 0 };
        }
      } else {
        const y = cy + sy;
        const edgeIndex = nearestEdgeIndex(grid.rowY, y, Math.max(splitY + 1, 2), grid.maxRow + 1);
        if (edgeIndex !== null) {
          return { kind: "row", index: edgeIndex - 1, edgeY: grid.rowY[edgeIndex] ?? 0 };
        }
      }
    }
    return null;
  }
  function maybeOutlineCursor(cp) {
    if (outlineButtonAt(cp) || outlineCornerAt(cp)) {
      canvas.style.cursor = "pointer";
      hidePopover();
      return true;
    }
    return false;
  }
  function onPointerMove(ev) {
    if (drag) {
      const p = toLogical(ev);
      if (drag.hit.kind === "col") {
        const delta = p.x - drag.startPx;
        const next = Math.max(MIN_COL_W, drag.original + delta);
        opts.colOverrides.set(drag.hit.index, next);
      } else {
        const delta = p.y - drag.startPx;
        const next = Math.max(MIN_ROW_H, drag.original + delta);
        opts.rowOverrides.set(drag.hit.index, next);
      }
      invalidateGrid();
      opts.redraw();
      return;
    }
    const cp = toCanvasLocal(ev);
    if (maybeOutlineCursor(cp))
      return;
    const hit = hitTest(cp.x, cp.y);
    if (hit) {
      canvas.style.cursor = hit.kind === "col" ? "col-resize" : "row-resize";
      hidePopover();
      return;
    }
    ensureMaps();
    const lp = toLogical(ev);
    const cell = cellAtLogical(lp);
    if (!cell) {
      canvas.style.cursor = savedCursor;
      hidePopover();
      return;
    }
    const anchor = resolveAnchor(cell.r, cell.c);
    const k = `${anchor.r}:${anchor.c}`;
    const link = hyperlinkMap.get(k);
    const cmt = commentMap.get(k);
    canvas.style.cursor = link ? "pointer" : savedCursor;
    if (cmt) {
      const grid = getGrid();
      const z = opts.zoom.get();
      const r = canvas.getBoundingClientRect();
      const vp = opts.getViewport?.() ?? null;
      const { splitX, splitY, pcw, prh } = frozenDims(opts.getSheet(), grid);
      const cx = grid.colX[anchor.c] ?? 0;
      const cy = grid.rowY[anchor.r] ?? 0;
      const cw = grid.colW[anchor.c] ?? 0;
      const sx = vp && anchor.c >= splitX ? vp.x : 0;
      const sy = vp && anchor.r >= splitY ? vp.y : 0;
      const left = r.left + (cx - sx) * z;
      const top = r.top + (cy - sy) * z;
      const right = left + cw * z;
      showPopover(cmt, { left, top, right });
    } else {
      hidePopover();
    }
  }
  function setSelection(active, range) {
    opts.activeCell.set(active);
    opts.selection?.set(range);
  }
  function outlineButtonAt(cp) {
    const sheet = opts.getSheet();
    const grid = getGrid();
    if (grid.rowGutterW === 0 && grid.colGutterH === 0)
      return null;
    const vp = opts.getViewport?.() ?? null;
    const { splitX, splitY, pcw, prh } = frozenDims(sheet, grid);
    const view = {
      sx: vp?.x ?? 0,
      sy: vp?.y ?? 0,
      splitX,
      splitY,
      pcw,
      prh,
      canvasW: canvas.clientWidth || canvas.width,
      canvasH: canvas.clientHeight || canvas.height
    };
    const hits = outlineButtonHits(sheet, grid, view);
    let best = null;
    let bestD = Infinity;
    for (const h of hits) {
      const d = Math.max(Math.abs(cp.x - h.cx), Math.abs(cp.y - h.cy));
      if (d <= OUTLINE_BUTTON_HIT_RADIUS && d < bestD) {
        best = h;
        bestD = d;
      }
    }
    return best ? { run: best.run, collapsed: best.collapsed } : null;
  }
  function outlineCornerAt(cp) {
    const grid = getGrid();
    if (grid.rowGutterW === 0 && grid.colGutterH === 0)
      return null;
    if (cp.x > Math.max(grid.rowGutterW, grid.originX))
      return null;
    if (cp.y > Math.max(grid.colGutterH, grid.originY))
      return null;
    const hits = outlineCornerHits(grid);
    let best = null;
    let bestD = Infinity;
    for (const h of hits) {
      const d = Math.max(Math.abs(cp.x - h.cx), Math.abs(cp.y - h.cy));
      if (d <= OUTLINE_BUTTON_HIT_RADIUS && d < bestD) {
        best = h;
        bestD = d;
      }
    }
    return best ? { axis: best.axis, level: best.level } : null;
  }
  function naturalRowHeight(sheet, r) {
    const meta = sheet.decodedRowMeta;
    if (meta) {
      for (let i = 0;i < meta.count; i++) {
        if (meta.index[i] === r) {
          const h = meta.heightPx[i];
          if (h !== undefined && !Number.isNaN(h))
            return h;
          break;
        }
      }
    }
    return sheet.defaultRowHeightPx;
  }
  function naturalColWidth(sheet, c) {
    for (const col of sheet.cols) {
      if (c >= col.min && c <= col.max)
        return col.widthPx;
    }
    return sheet.defaultColWidthPx;
  }
  function setRunCollapsed(run, collapsed) {
    const sheet = opts.getSheet();
    if (run.axis === "row") {
      for (let r = run.start;r <= run.end; r++) {
        if (collapsed)
          opts.rowOverrides.set(r, 0);
        else
          opts.rowOverrides.set(r, Math.max(1, naturalRowHeight(sheet, r)));
      }
    } else {
      for (let c = run.start;c <= run.end; c++) {
        if (collapsed)
          opts.colOverrides.set(c, 0);
        else
          opts.colOverrides.set(c, Math.max(1, naturalColWidth(sheet, c)));
      }
    }
  }
  function applyCornerCollapse(target) {
    const sheet = opts.getSheet();
    const grid = getGrid();
    const runs = computeOutlineRuns(sheet, grid);
    for (const run of runs) {
      if (run.axis !== target.axis)
        continue;
      const shouldCollapse = run.level >= target.level;
      setRunCollapsed(run, shouldCollapse);
    }
    invalidateGrid();
    opts.redraw();
  }
  function onPointerDown(ev) {
    if (ev.button !== 0)
      return;
    const cp = toCanvasLocal(ev);
    const p = toLogical(ev);
    const ob = outlineButtonAt(cp);
    if (ob) {
      ev.preventDefault();
      setRunCollapsed(ob.run, !ob.collapsed);
      invalidateGrid();
      opts.redraw();
      canvas.focus({ preventScroll: true });
      return;
    }
    const oc = outlineCornerAt(cp);
    if (oc) {
      ev.preventDefault();
      applyCornerCollapse(oc);
      canvas.focus({ preventScroll: true });
      return;
    }
    const hit = hitTest(cp.x, cp.y);
    if (hit) {
      ev.preventDefault();
      canvas.setPointerCapture(ev.pointerId);
      const grid2 = getGrid();
      if (hit.kind === "col") {
        drag = { hit, startPx: p.x, original: grid2.colW[hit.index] ?? 0 };
      } else {
        drag = { hit, startPx: p.y, original: grid2.rowH[hit.index] ?? 0 };
      }
      return;
    }
    const grid = getGrid();
    const inColHeader = cp.y >= grid.colGutterH && cp.y < grid.originY;
    const inRowHeader = cp.x >= grid.rowGutterW && cp.x < grid.originX;
    if (inColHeader && inRowHeader) {
      ev.preventDefault();
      setSelection({ r: 1, c: 1 }, { r1: 1, c1: 1, r2: grid.maxRow, c2: grid.maxCol });
      opts.redraw();
      canvas.focus({ preventScroll: true });
      return;
    }
    if (inColHeader && cp.x >= grid.originX) {
      const cell = cellAt(grid, p.x, grid.originY + 1);
      if (cell) {
        ev.preventDefault();
        setSelection({ r: 1, c: cell.c }, { r1: 1, c1: cell.c, r2: grid.maxRow, c2: cell.c });
        opts.redraw();
        canvas.focus({ preventScroll: true });
      }
      return;
    }
    if (inRowHeader && cp.y >= grid.originY) {
      const cell = cellAt(grid, grid.originX + 1, p.y);
      if (cell) {
        ev.preventDefault();
        setSelection({ r: cell.r, c: 1 }, { r1: cell.r, c1: 1, r2: cell.r, c2: grid.maxCol });
        opts.redraw();
        canvas.focus({ preventScroll: true });
      }
      return;
    }
    if (cp.x >= grid.originX && cp.y >= grid.originY) {
      const cell = cellAt(grid, p.x, p.y);
      if (cell) {
        const sheet = opts.getSheet();
        let anchor = cell;
        for (const m of sheet.merges) {
          if (cell.r >= m.r1 && cell.r <= m.r2 && cell.c >= m.c1 && cell.c <= m.c2) {
            anchor = { r: m.r1, c: m.c1 };
            setSelection({ r: m.r1, c: m.c1 }, { r1: m.r1, c1: m.c1, r2: m.r2, c2: m.c2 });
            break;
          }
        }
        if (anchor === cell) {
          setSelection(cell, { r1: cell.r, c1: cell.c, r2: cell.r, c2: cell.c });
        }
        opts.redraw();
        canvas.focus({ preventScroll: true });
        ensureMaps();
        const link = hyperlinkMap.get(`${anchor.r}:${anchor.c}`);
        if (link)
          openHyperlink(link);
      }
    }
  }
  function openHyperlink(link) {
    const t = link.target ?? "";
    const isInWorkbook = t.startsWith("#") || !link.target && !!link.location;
    if (isInWorkbook) {
      const dest = link.target?.startsWith("#") ? link.target.slice(1) : link.location ?? "";
      canvas.dispatchEvent(new CustomEvent("xlcore-hyperlink-jump", {
        detail: { location: dest },
        bubbles: true
      }));
      return;
    }
    if (link.target) {
      window.open(link.target, "_blank", "noopener");
    }
  }
  function cellAt(grid, x, y) {
    const c = edgeOwnerIndex(grid.colX, x, 1, grid.maxCol);
    const r = edgeOwnerIndex(grid.rowY, y, 1, grid.maxRow);
    if (r === null || c === null)
      return null;
    return { r, c };
  }
  function edgeOwnerIndex(edges, px, minIndex, maxIndex) {
    if (maxIndex < minIndex)
      return null;
    if (px < (edges[minIndex] ?? 0) || px >= (edges[maxIndex + 1] ?? 0))
      return null;
    let lo = minIndex + 1;
    let hi = maxIndex + 1;
    while (lo < hi) {
      const mid = lo + hi >> 1;
      if ((edges[mid] ?? 0) <= px)
        lo = mid + 1;
      else
        hi = mid;
    }
    return lo - 1;
  }
  function nearestEdgeIndex(edges, px, minEdgeIndex, maxEdgeIndex) {
    if (maxEdgeIndex < minEdgeIndex)
      return null;
    let lo = minEdgeIndex;
    let hi = maxEdgeIndex + 1;
    while (lo < hi) {
      const mid = lo + hi >> 1;
      if ((edges[mid] ?? 0) < px)
        lo = mid + 1;
      else
        hi = mid;
    }
    let best = null;
    let bestDist = Infinity;
    for (const i of [lo - 1, lo]) {
      if (i < minEdgeIndex || i > maxEdgeIndex)
        continue;
      const dist = Math.abs(px - (edges[i] ?? 0));
      if (dist < bestDist) {
        best = i;
        bestDist = dist;
      }
    }
    return bestDist <= RESIZE_TOL ? best : null;
  }
  function ensureVisible(cell) {
    const sc = opts.scrollContainer;
    if (!sc)
      return;
    const sheet = opts.getSheet();
    const grid = getGrid();
    const z = opts.zoom.get();
    const { splitX, splitY, pcw, prh } = frozenDims(sheet, grid);
    const x = (grid.colX[cell.c] ?? 0) * z;
    const y = (grid.rowY[cell.r] ?? 0) * z;
    const w = (grid.colW[cell.c] ?? 0) * z;
    const h = (grid.rowH[cell.r] ?? 0) * z;
    const padX = (grid.originX + pcw) * z;
    const padY = (grid.originY + prh) * z;
    if (cell.c >= splitX) {
      if (x < sc.scrollLeft + padX)
        sc.scrollLeft = x - padX;
      else if (x + w > sc.scrollLeft + sc.clientWidth)
        sc.scrollLeft = x + w - sc.clientWidth;
    }
    if (cell.r >= splitY) {
      if (y < sc.scrollTop + padY)
        sc.scrollTop = y - padY;
      else if (y + h > sc.scrollTop + sc.clientHeight)
        sc.scrollTop = y + h - sc.clientHeight;
    }
  }
  function onKeyDown(ev) {
    const cur = opts.activeCell.get();
    if (!cur)
      return;
    let dr = 0, dc = 0;
    switch (ev.key) {
      case "ArrowUp":
        dr = -1;
        break;
      case "ArrowDown":
        dr = 1;
        break;
      case "ArrowLeft":
        dc = -1;
        break;
      case "ArrowRight":
        dc = 1;
        break;
      case "Tab":
        dc = ev.shiftKey ? -1 : 1;
        break;
      case "Enter":
        dr = ev.shiftKey ? -1 : 1;
        break;
      default:
        return;
    }
    ev.preventDefault();
    const grid = getGrid();
    const next = {
      r: clamp2(cur.r + dr, 1, grid.maxRow),
      c: clamp2(cur.c + dc, 1, grid.maxCol)
    };
    setSelection(next, { r1: next.r, c1: next.c, r2: next.r, c2: next.c });
    ensureVisible(next);
    opts.redraw();
  }
  function onPointerUp(ev) {
    if (drag) {
      try {
        canvas.releasePointerCapture(ev.pointerId);
      } catch {}
      drag = null;
    }
  }
  function onPointerLeave() {
    if (!drag)
      canvas.style.cursor = savedCursor;
    hidePopover();
  }
  function onWheel(ev) {
    if (!ev.ctrlKey && !ev.metaKey)
      return;
    ev.preventDefault();
    const cur = opts.zoom.get();
    const next = clamp2(cur * Math.exp(-ev.deltaY * 0.01), ZOOM_MIN, ZOOM_MAX);
    if (next === cur)
      return;
    const sc = opts.scrollContainer;
    const vp = opts.getViewport?.();
    if (sc && vp) {
      const r = canvas.getBoundingClientRect();
      const cssX = ev.clientX - r.left;
      const cssY = ev.clientY - r.top;
      const newVpX = vp.x + cssX * (1 / cur - 1 / next);
      const newVpY = vp.y + cssY * (1 / cur - 1 / next);
      opts.zoom.set(next);
      sc.scrollLeft = Math.max(0, newVpX * next);
      sc.scrollTop = Math.max(0, newVpY * next);
      opts.redraw();
    } else if (sc) {
      const r = canvas.getBoundingClientRect();
      const px = ev.clientX - r.left;
      const py = ev.clientY - r.top;
      const lx = px / cur;
      const ly = py / cur;
      opts.zoom.set(next);
      opts.redraw();
      const newPx = lx * next;
      const newPy = ly * next;
      sc.scrollLeft += newPx - px;
      sc.scrollTop += newPy - py;
    } else {
      opts.zoom.set(next);
      opts.redraw();
    }
  }
  if (!canvas.hasAttribute("tabindex"))
    canvas.tabIndex = 0;
  const savedOutline = canvas.style.outline;
  canvas.style.outline = "none";
  canvas.addEventListener("pointermove", onPointerMove);
  canvas.addEventListener("pointerdown", onPointerDown);
  canvas.addEventListener("pointerup", onPointerUp);
  canvas.addEventListener("pointercancel", onPointerUp);
  canvas.addEventListener("pointerleave", onPointerLeave);
  canvas.addEventListener("keydown", onKeyDown);
  canvas.addEventListener("wheel", onWheel, { passive: false });
  return {
    destroy() {
      canvas.removeEventListener("pointermove", onPointerMove);
      canvas.removeEventListener("pointerdown", onPointerDown);
      canvas.removeEventListener("pointerup", onPointerUp);
      canvas.removeEventListener("pointercancel", onPointerUp);
      canvas.removeEventListener("pointerleave", onPointerLeave);
      canvas.removeEventListener("keydown", onKeyDown);
      canvas.removeEventListener("wheel", onWheel);
      canvas.style.cursor = savedCursor;
      canvas.style.outline = savedOutline;
      if (popoverEl && popoverEl.parentNode)
        popoverEl.parentNode.removeChild(popoverEl);
      popoverEl = null;
    }
  };
}
function clamp2(v, lo, hi) {
  return v < lo ? lo : v > hi ? hi : v;
}

// src/previewer.ts
var VIRTUAL_EXTRA_COLS = 50;
var VIRTUAL_EXTRA_ROWS = 1000;
function createWorkbookPreviewer(container, layout, options = {}) {
  return new WorkbookPreviewerImpl(container, layout, options);
}

class WorkbookPreviewerImpl extends EventTarget {
  root;
  canvas;
  layout;
  tabs;
  sheetTabs;
  formulaBar;
  zoomBox;
  nameBox;
  formulaBox;
  zoomLabel;
  zoomOut;
  zoomIn;
  stage;
  spacer;
  sheetStates;
  tabButtons = [];
  resizeObserver;
  interactHandle = null;
  activeSheetIndex = 0;
  zoom = 1;
  viewport = { x: 0, y: 0, w: 0, h: 0 };
  rafPending = false;
  constructor(container, rawLayout, options) {
    super();
    this.layout = decodeWorkbookLayout(rawLayout);
    this.zoom = clamp3(options.initialZoom ?? 1, 0.25, 4);
    this.sheetStates = this.layout.sheets.map(() => ({
      colOverrides: new Map,
      rowOverrides: new Map,
      activeCell: { r: 1, c: 1 },
      selection: { r1: 1, c1: 1, r2: 1, c2: 1 }
    }));
    this.root = document.createElement("div");
    this.root.className = options.className ?? "xlcore-previewer";
    this.root.style.cssText = "display:grid;grid-template-rows:auto auto minmax(0,1fr);min-width:0;min-height:0;width:100%;height:100%;overflow:hidden;background:#f4f4f5;font:13px -apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;";
    this.formulaBar = document.createElement("div");
    this.formulaBar.className = "xlcore-formula-bar";
    this.formulaBar.style.cssText = "display:flex;gap:6px;align-items:center;padding:6px 8px;background:#f8fafc;border-bottom:1px solid #d1d5db;min-width:0;";
    this.nameBox = document.createElement("div");
    this.nameBox.style.cssText = "font:12px ui-monospace,SFMono-Regular,Menlo,monospace;padding:4px 10px;background:#fff;border:1px solid #d1d5db;border-radius:4px;min-width:86px;color:#111827;text-align:center;white-space:nowrap;";
    const fxLabel = document.createElement("div");
    fxLabel.textContent = "fx";
    fxLabel.style.cssText = "font:600 12px ui-monospace,SFMono-Regular,Menlo,monospace;color:#4b5563;padding:0 2px;";
    this.formulaBox = document.createElement("input");
    this.formulaBox.readOnly = true;
    this.formulaBox.setAttribute("aria-label", "Formula or value");
    this.formulaBox.style.cssText = "min-width:0;flex:1;height:28px;padding:0 9px;border:1px solid #d1d5db;border-radius:4px;background:#fff;color:#111827;font:12px ui-monospace,SFMono-Regular,Menlo,monospace;";
    this.formulaBar.append(this.nameBox, fxLabel, this.formulaBox);
    this.tabs = document.createElement("div");
    this.tabs.className = "xlcore-tabs";
    this.tabs.style.cssText = "display:flex;align-items:stretch;gap:6px;padding:0 8px;background:#e5e7eb;min-width:0;min-height:31px;overflow:hidden;";
    this.sheetTabs = document.createElement("div");
    this.sheetTabs.className = "xlcore-sheet-tabs";
    this.sheetTabs.style.cssText = "display:flex;gap:2px;flex:1 1 auto;min-width:0;overflow-x:auto;overflow-y:hidden;scrollbar-width:thin;";
    this.zoomBox = document.createElement("div");
    this.zoomBox.className = "xlcore-zoom";
    this.zoomBox.style.cssText = "margin-left:auto;display:flex;gap:4px;align-items:center;padding-right:8px;flex:none;";
    this.zoomOut = makeButton("-");
    this.zoomLabel = document.createElement("span");
    this.zoomLabel.style.cssText = "font-size:12px;min-width:42px;text-align:center;color:#374151;";
    this.zoomIn = makeButton("+");
    this.zoomBox.append(this.zoomOut, this.zoomLabel, this.zoomIn);
    this.tabs.append(this.sheetTabs, this.zoomBox);
    this.stage = document.createElement("div");
    this.stage.className = "xlcore-stage";
    this.stage.style.cssText = "overflow:auto;position:relative;background:#f4f4f5;min-width:0;min-height:0;width:100%;";
    this.spacer = document.createElement("div");
    this.spacer.style.position = "relative";
    this.canvas = document.createElement("canvas");
    this.canvas.style.cssText = "position:sticky;top:0;left:0;background:#fff;display:block;box-shadow:0 1px 3px rgba(0,0,0,0.1);";
    this.spacer.append(this.canvas);
    this.stage.append(this.spacer);
    this.root.append(this.formulaBar, this.tabs, this.stage);
    container.append(this.root);
    this.activeSheetIndex = this.resolveInitialSheet(options.initialSheet);
    this.zoomOut.onclick = () => this.setZoom(this.zoom - 0.25);
    this.zoomIn.onclick = () => this.setZoom(this.zoom + 0.25);
    this.stage.addEventListener("scroll", this.scheduleDraw, { passive: true });
    window.addEventListener("xlcore-image-ready", this.scheduleDraw);
    this.resizeObserver = new ResizeObserver(() => {
      this.updateSpacerSize();
      this.scheduleDraw();
    });
    this.resizeObserver.observe(this.stage);
    this.renderTabs();
    this.attachInteractivity();
    this.updateZoomLabel();
    this.updateSpacerSize();
    this.draw();
  }
  destroy() {
    this.interactHandle?.destroy();
    this.interactHandle = null;
    this.resizeObserver.disconnect();
    this.stage.removeEventListener("scroll", this.scheduleDraw);
    window.removeEventListener("xlcore-image-ready", this.scheduleDraw);
    this.root.remove();
  }
  redraw() {
    this.draw();
  }
  getState() {
    return {
      activeSheetIndex: this.activeSheetIndex,
      activeCell: { ...this.currentState().activeCell },
      selection: { ...this.currentState().selection },
      zoom: this.zoom
    };
  }
  getActiveSheet() {
    return this.layout.sheets[this.activeSheetIndex] ?? this.layout.sheets[0];
  }
  getActiveSheetIndex() {
    return this.activeSheetIndex;
  }
  setActiveSheet(sheet) {
    const next = this.resolveSheet(sheet);
    if (next === this.activeSheetIndex)
      return;
    this.activeSheetIndex = next;
    this.stage.scrollTop = 0;
    this.stage.scrollLeft = 0;
    this.attachInteractivity();
    this.updateActiveTab();
    this.updateSpacerSize();
    this.draw();
    this.scrollActiveTabIntoView();
    this.emit("sheetchange");
  }
  getActiveCell() {
    return { ...this.currentState().activeCell };
  }
  getSelection() {
    return { ...this.currentState().selection };
  }
  selectCell(r, c, options = {}) {
    this.selectRange({ r1: r, c1: c, r2: r, c2: c }, { scroll: options.scroll, activeCell: { r, c } });
  }
  selectRange(selection, options = {}) {
    const grid = buildGrid(this.getActiveSheet(), this.currentState().colOverrides, this.currentState().rowOverrides);
    const range = normalizeSelection(selection, grid.maxRow, grid.maxCol);
    const active = options.activeCell ? {
      r: clamp3(Math.floor(options.activeCell.r), range.r1, range.r2),
      c: clamp3(Math.floor(options.activeCell.c), range.c1, range.c2)
    } : { r: range.r1, c: range.c1 };
    const state = this.currentState();
    state.activeCell = active;
    state.selection = range;
    if (options.scroll)
      this.scrollToCell(active.r, active.c);
    this.draw();
    this.emit("selectionchange");
  }
  scrollToCell(r, c) {
    const sheet = this.getActiveSheet();
    const state = this.currentState();
    const grid = buildGrid(sheet, state.colOverrides, state.rowOverrides);
    const rr = clamp3(Math.floor(r), 1, grid.maxRow);
    const cc = clamp3(Math.floor(c), 1, grid.maxCol);
    const z = this.zoom;
    const x = (grid.colX[cc] ?? 0) * z;
    const y = (grid.rowY[rr] ?? 0) * z;
    const w = (grid.colW[cc] ?? 0) * z;
    const h = (grid.rowH[rr] ?? 0) * z;
    const padX = grid.originX * z;
    const padY = grid.originY * z;
    if (x < this.stage.scrollLeft + padX)
      this.stage.scrollLeft = Math.max(0, x - padX);
    else if (x + w > this.stage.scrollLeft + this.stage.clientWidth) {
      this.stage.scrollLeft = x + w - this.stage.clientWidth;
    }
    if (y < this.stage.scrollTop + padY)
      this.stage.scrollTop = Math.max(0, y - padY);
    else if (y + h > this.stage.scrollTop + this.stage.clientHeight) {
      this.stage.scrollTop = y + h - this.stage.clientHeight;
    }
  }
  getZoom() {
    return this.zoom;
  }
  setZoom(zoom) {
    const next = clamp3(Math.round(zoom * 100) / 100, 0.25, 4);
    if (next === this.zoom)
      return;
    this.zoom = next;
    this.updateZoomLabel();
    this.updateSpacerSize();
    this.draw();
    this.emit("zoomchange");
  }
  on(name, listener) {
    this.addEventListener(name, listener);
  }
  off(name, listener) {
    this.removeEventListener(name, listener);
  }
  scheduleDraw = () => {
    if (this.rafPending)
      return;
    this.rafPending = true;
    requestAnimationFrame(() => {
      this.rafPending = false;
      this.draw();
    });
  };
  currentState() {
    return this.sheetStates[this.activeSheetIndex] ?? this.sheetStates[0];
  }
  draw() {
    const state = this.currentState();
    this.recomputeViewport();
    render(this.canvas, this.getActiveSheet(), this.layout, {
      scale: window.devicePixelRatio || 1,
      zoom: this.zoom,
      colOverrides: state.colOverrides,
      rowOverrides: state.rowOverrides,
      activeCell: state.activeCell,
      selection: state.selection,
      viewport: this.viewport
    });
    this.nameBox.textContent = formatNameBox(state.activeCell, state.selection);
    this.formulaBox.value = formatFormulaBar(this.getActiveSheet(), state.activeCell);
  }
  attachInteractivity() {
    this.interactHandle?.destroy();
    const state = this.currentState();
    this.interactHandle = attachInteractivity(this.canvas, {
      getSheet: () => this.getActiveSheet(),
      getLayout: () => this.layout,
      zoom: {
        get: () => this.zoom,
        set: (value) => {
          this.zoom = value;
          this.updateZoomLabel();
          this.updateSpacerSize();
          this.emit("zoomchange");
        }
      },
      colOverrides: state.colOverrides,
      rowOverrides: state.rowOverrides,
      activeCell: {
        get: () => state.activeCell,
        set: (value) => {
          if (value)
            state.activeCell = value;
        }
      },
      selection: {
        get: () => state.selection,
        set: (value) => {
          if (value) {
            state.selection = value;
            this.emit("selectionchange");
          }
        }
      },
      scrollContainer: this.stage,
      getViewport: () => this.viewport,
      redraw: this.scheduleDraw
    });
  }
  renderTabs() {
    this.sheetTabs.replaceChildren();
    this.tabButtons.length = 0;
    this.layout.sheets.forEach((sheet, i) => {
      const button = makeTab(sheet.name);
      button.onclick = () => this.setActiveSheet(i);
      this.sheetTabs.append(button);
      this.tabButtons.push(button);
    });
    this.updateActiveTab();
  }
  updateActiveTab() {
    this.tabButtons.forEach((button, i) => {
      button.classList.toggle("active", i === this.activeSheetIndex);
      button.style.fontWeight = i === this.activeSheetIndex ? "600" : "400";
    });
  }
  scrollActiveTabIntoView() {
    const activeButton = this.tabButtons[this.activeSheetIndex];
    if (!activeButton)
      return;
    activeButton.scrollIntoView({ block: "nearest", inline: "nearest" });
  }
  recomputeViewport() {
    this.viewport = {
      x: this.stage.scrollLeft / this.zoom,
      y: this.stage.scrollTop / this.zoom,
      w: this.stage.clientWidth / this.zoom,
      h: this.stage.clientHeight / this.zoom
    };
  }
  updateSpacerSize() {
    const vs = virtualSize(this.getActiveSheet(), this.currentState());
    this.spacer.style.width = `${vs.w * this.zoom}px`;
    this.spacer.style.height = `${vs.h * this.zoom}px`;
  }
  updateZoomLabel() {
    this.zoomLabel.textContent = `${Math.round(this.zoom * 100)}%`;
  }
  resolveInitialSheet(sheet) {
    if (sheet !== undefined)
      return this.resolveSheet(sheet);
    const active = this.layout.activeSheetIndex;
    return typeof active === "number" && active >= 0 && active < this.layout.sheets.length ? active : 0;
  }
  resolveSheet(sheet) {
    if (typeof sheet === "number") {
      const i2 = Math.floor(sheet);
      if (i2 < 0 || i2 >= this.layout.sheets.length)
        throw new RangeError(`sheet index out of range: ${sheet}`);
      return i2;
    }
    const i = this.layout.sheets.findIndex((s) => s.name === sheet);
    if (i < 0)
      throw new Error(`sheet not found: ${sheet}`);
    return i;
  }
  emit(name) {
    this.dispatchEvent(new CustomEvent(name, { detail: this.getState() }));
  }
}
function virtualSize(sheet, state) {
  const dw = sheet.defaultColWidthPx || 64;
  const dh = sheet.defaultRowHeightPx || 18;
  const maxCol = Math.min(16384, Math.max(sheet.maxCol + 2, sheet.maxCol + VIRTUAL_EXTRA_COLS));
  const maxRow = Math.min(1048576, Math.max(sheet.maxRow + 5, sheet.maxRow + VIRTUAL_EXTRA_ROWS));
  let w = HEADER_W + maxCol * dw;
  let h = HEADER_H + maxRow * dh;
  const colWidths = new Map;
  for (const c of sheet.cols) {
    for (let i = c.min;i <= c.max; i++)
      colWidths.set(i, c.hidden ? 0 : c.widthPx);
  }
  for (const [c, v] of state.colOverrides)
    colWidths.set(c, Math.max(0, v));
  for (const [c, v] of colWidths)
    if (c >= 1 && c <= maxCol)
      w += v - dw;
  const rowHeights = new Map;
  iterRows(sheet, (row) => {
    if (row.hidden)
      rowHeights.set(row.index, 0);
    else if (row.heightPx !== undefined)
      rowHeights.set(row.index, row.heightPx);
  });
  for (const [r, v] of state.rowOverrides)
    rowHeights.set(r, Math.max(0, v));
  for (const [r, v] of rowHeights)
    if (r >= 1 && r <= maxRow)
      h += v - dh;
  return { w, h };
}
function normalizeSelection(selection, maxRow, maxCol) {
  const r1 = clamp3(Math.floor(Math.min(selection.r1, selection.r2)), 1, maxRow);
  const r2 = clamp3(Math.floor(Math.max(selection.r1, selection.r2)), 1, maxRow);
  const c1 = clamp3(Math.floor(Math.min(selection.c1, selection.c2)), 1, maxCol);
  const c2 = clamp3(Math.floor(Math.max(selection.c1, selection.c2)), 1, maxCol);
  return { r1, c1, r2, c2 };
}
function formatNameBox(active, selection) {
  if (selection.r1 !== selection.r2 || selection.c1 !== selection.c2) {
    return `${colLabel2(active.c)}${active.r}  (${selection.r2 - selection.r1 + 1}R×${selection.c2 - selection.c1 + 1}C)`;
  }
  return colLabel2(active.c) + active.r;
}
function formatFormulaBar(sheet, active) {
  const cell = findCell(sheet, active.r, active.c);
  if (!cell)
    return "";
  if (cell.formula)
    return cell.formula.startsWith("=") ? cell.formula : `=${cell.formula}`;
  if (cell.value !== undefined)
    return String(cell.value);
  if (cell.runs && cell.runs.length > 0)
    return cell.runs.map((run) => run.text).join("");
  return "";
}
function colLabel2(n) {
  let s = "";
  let cur = Math.max(1, Math.floor(n));
  while (cur > 0) {
    const r = (cur - 1) % 26;
    s = String.fromCharCode(65 + r) + s;
    cur = Math.floor((cur - 1) / 26);
  }
  return s;
}
function makeButton(label) {
  const button = document.createElement("button");
  button.textContent = label;
  button.style.cssText = "background:#fff;border:1px solid #d1d5db;padding:4px 10px;cursor:pointer;font:inherit;font-size:12px;border-radius:4px;";
  return button;
}
function makeTab(label) {
  const button = document.createElement("button");
  button.textContent = label;
  button.style.cssText = "flex:none;background:#fff;border:1px solid #d1d5db;border-bottom:none;padding:6px 14px;cursor:pointer;font:inherit;font-size:12px;white-space:nowrap;";
  return button;
}
function clamp3(v, lo, hi) {
  return v < lo ? lo : v > hi ? hi : v;
}

// src/browserLoader.ts
var DEFAULT_WASM_URL = new URL("./xlcore_wasm.js", import.meta.url).toString();
var DEFAULT_WORKER_URL = new URL("./xlsxWorker.js", import.meta.url).toString();
async function loadWorkbookFromFile(file, options = {}) {
  progress(options, "Reading file");
  const bytes = await file.arrayBuffer();
  return loadWorkbookFromArrayBuffer(bytes, options);
}
async function loadWorkbookFromArrayBuffer(bytes, options = {}) {
  const worker = createExtractionWorker(options);
  return await new Promise((resolve, reject) => {
    worker.onmessage = (event) => {
      const message = event.data;
      if (message.type === "stage") {
        progress(options, message.label);
      } else if (message.type === "layout") {
        worker.terminate();
        resolve(message.layout);
      } else if (message.type === "error") {
        worker.terminate();
        reject(new Error(message.message));
      }
    };
    worker.onerror = (event) => {
      worker.terminate();
      reject(new Error(event.message || "Workbook worker failed"));
    };
    worker.postMessage({ bytes, wasmUrl: options.wasmUrl ?? DEFAULT_WASM_URL }, [bytes]);
  });
}
async function createWorkbookPreviewerFromFile(container, file, options = {}) {
  const layout = await loadWorkbookFromFile(file, options);
  progress(options, "Preparing preview");
  const previewer = createWorkbookPreviewer(container, layout, options);
  progress(options, "Rendering canvas");
  return previewer;
}
function createExtractionWorker(options) {
  const workerUrl = options.workerUrl ?? DEFAULT_WORKER_URL;
  try {
    return new Worker(workerUrl, { type: "module" });
  } catch {
    return createBlobWorker();
  }
}
function createBlobWorker() {
  const source = `
let wasmModulePromise = null;
const stage = (label) => self.postMessage({ type: "stage", label });
self.onmessage = async (event) => {
  try {
    const { bytes, wasmUrl } = event.data;
    stage("Loading WASM");
    wasmModulePromise ??= import(wasmUrl).then(async (mod) => {
      await mod.default();
      return mod;
    });
    const mod = await wasmModulePromise;
    stage("Extracting OOXML");
    const layout = mod.extract_xlsx(new Uint8Array(bytes), undefined);
    self.postMessage({ type: "layout", layout });
  } catch (error) {
    self.postMessage({ type: "error", message: error && error.stack ? error.stack : String(error) });
  }
};
`;
  return new Worker(URL.createObjectURL(new Blob([source], { type: "text/javascript" })));
}
function progress(options, label) {
  options.onProgress?.({ label });
}
export {
  loadWorkbookFromFile,
  loadWorkbookFromArrayBuffer,
  createWorkbookPreviewerFromFile
};
