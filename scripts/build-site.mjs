#!/usr/bin/env node
// Assembles the Cloudflare Pages deploy directory.
//
//   site/
//     index.html          -- landing page with links to /app and /tiles
//     app/index.html      <- packages/xlsx-preview/examples/xlsx-app.html
//     tiles/index.html    <- packages/xlsx-preview/examples/xlsx-tiles.html
//     multi/index.html    <- packages/xlsx-preview/examples/xlsx-multi.html
//
// Cloudflare Pages maps `/app` to `app/index.html` automatically.
// The examples try `../dist/browserLoader.js` first (for local dev under
// `pnpm preview`); that 404s on Pages and the page falls through to the
// jsdelivr-hosted npm bundle pinned to the published version. We don't
// ship dist/ here — jsdelivr is globally edge-cached and pinned by
// `@version`, so re-hosting the 17 MB wasm would just duplicate it.

import { execFileSync } from "node:child_process";
import { cp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { homedir } from "node:os";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, "..");
const out = resolve(repo, "site");
const examples = resolve(repo, "packages/xlsx-preview/examples");
const pkg = JSON.parse(await readFile(resolve(repo, "packages/xlsx-preview/package.json"), "utf8"));
const packageVersion = pkg.version;
// Demo workbook for the landing terminal. Prefer the JPM GOOGL research model
// from the local fixture stash (not committed — third-party copyright); fall
// back to the in-repo kitchensink so the build still works on CI / fresh clones.
const demoCandidates = [
  resolve(homedir(), "Developer/excel-fixtures/e-007_input-4.xlsx"),
  resolve(repo, "tests/fixtures/kitchensink/kitchensink.xlsx"),
];
let fixture = demoCandidates[demoCandidates.length - 1];
for (const c of demoCandidates) {
  try { await readFile(c); fixture = c; break; } catch {}
}

const routes = [
  ["app", "xlsx-app.html"],
  ["tiles", "xlsx-tiles.html"],
];

async function exists(p) {
  try {
    await readFile(p);
    return true;
  } catch {
    return false;
  }
}

async function xlsxPreview(args, opts = {}) {
  const localCli = resolve(repo, "packages/xlsx-preview/dist/cli.js");
  if (await exists(localCli)) {
    return execFileSync(process.execPath, [localCli, ...args], opts);
  }
  return execFileSync("xlsx-preview", args, opts);
}

await rm(out, { recursive: true, force: true });
await mkdir(out, { recursive: true });

for (const [route, file] of routes) {
  const src = resolve(examples, file);
  if (!(await exists(src))) {
    console.error(`Missing example: ${src}`);
    process.exit(1);
  }
  await mkdir(resolve(out, route), { recursive: true });
  await cp(src, resolve(out, route, "index.html"));
  // The example HTML imports `./shared/loadXlsxPreview.js` at module top level.
  // Without this, Cloudflare Pages serves its HTML 404 page and the browser
  // refuses the module with a MIME-type error before our try/catch CDN
  // fallback can run.
  await cp(resolve(examples, "shared"), resolve(out, route, "shared"), { recursive: true });
  console.log(`  /${route}  ←  ${file}`);
}

// --- render demo PNG via the CLI so the landing terminal can show real output
const demoDir = resolve(out, "demo");
await mkdir(demoDir, { recursive: true });
const demoPng = resolve(demoDir, "cover.png");
// Sheet to feature in the landing terminal demo. The JPM model has a DCF sheet
// that's visually rich (multi-year projections, banded sections, yellow result
// highlight, collapsed column outline groups) and — unlike the cover — carries
// no analyst contact info. Falls back to the active sheet if not present.
const preferredSheet = "DCF";
let demoInfo = null;
try {
  // Probe sheets first so the fallback (kitchensink) still works.
  const probe = JSON.parse(await xlsxPreview([fixture, "--info"], { encoding: "utf8" }));
  const hasPreferred = probe.sheets?.some((s) => s.name === preferredSheet);
  // Crop to a focused range when rendering the DCF sheet so the image lands
  // wider-than-tall and balances the snippet stack on the left. Falls back to
  // the whole active sheet for the kitchensink path.
  const range = hasPreferred ? "A1:V50" : null;
  const renderArgs = hasPreferred
    ? [fixture, "-o", demoPng, "--sheet", preferredSheet, "--range", range, "--scale", "2"]
    : [fixture, "-o", demoPng, "--sheet-index", "0", "--scale", "2"];
  await xlsxPreview(renderArgs, { stdio: "inherit" });
  const infoOut = await xlsxPreview([fixture, "--info"], { encoding: "utf8" });
  demoInfo = JSON.parse(infoOut);
  console.log(`  /demo/cover.png        (rendered ← ${fixture.split("/").pop()})`);
} catch (err) {
  console.warn(`  ⚠ skipped CLI demo render: ${err.message}`);
}

// Build a compact, readable summary of what `--info` would print, plus the
// matching reveal command. Both get embedded into the landing page so the
// terminal animation reflects the workbook we actually rendered.
function summariseInfo(info) {
  if (!info) {
    return {
      filename: "workbook.xlsx",
      infoLines: ["(workbook info unavailable)"],
      sheetName: "Sheet1",
      pngBytes: 0,
    };
  }
  const filename = fixture.split("/").pop();
  const sheets = info.sheets ?? [];
  const totalCells = sheets.reduce((s, x) => s + (x.cells || 0), 0);
  const totalComments = sheets.reduce((s, x) => s + (x.comments || 0), 0);
  const totalDrawings = sheets.reduce((s, x) => s + (x.drawings || 0), 0);
  const totalTables = sheets.reduce((s, x) => s + (x.tables || 0), 0);
  // Pick first 3 sheets + a trailing summary line.
  const head = sheets.slice(0, 3).map((s) => {
    const extras = [];
    if (s.drawings) extras.push(`drawings: ${s.drawings}`);
    if (s.comments) extras.push(`comments: ${s.comments}`);
    if (s.tables)   extras.push(`tables: ${s.tables}`);
    const tail = extras.length ? `, ${extras.join(", ")}` : "";
    return `    { "name": "${s.name}", "usedRange": "${s.usedRange}", "cells": ${s.cells}${tail} },`;
  });
  const more = sheets.length > 3 ? `    … +${sheets.length - 3} more sheets` : null;
  const lines = [
    "{",
    `  "sheets": ${sheets.length},`,
    `  "totals": { "cells": ${totalCells.toLocaleString("en-US")}, "comments": ${totalComments}, "drawings": ${totalDrawings}, "tables": ${totalTables} },`,
    '  "detail": [',
    ...head,
    ...(more ? [more] : []),
    "  ]",
    "}",
  ];
  const renderedSheet = sheets.find((s) => s.name === preferredSheet)?.name ?? sheets[0]?.name ?? "Sheet1";
  return {
    filename,
    infoLines: lines,
    sheetName: renderedSheet,
    pngBytes: 0, // filled in below
  };
}

const demoMeta = summariseInfo(demoInfo);
try {
  const { size } = await import("node:fs").then((m) => m.promises.stat(demoPng));
  demoMeta.pngBytes = size;
} catch {}

// Tiny tokeniser for the JSON-ish info output: colours "keys", "strings",
// numbers, and leaves braces/punctuation neutral.
function esc(s) { return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;"); }
function highlight(line) {
  let out = "";
  let i = 0;
  const isKeyContext = (rest) => /^\s*:/.test(rest);
  while (i < line.length) {
    const ch = line[i];
    if (ch === '"') {
      const end = line.indexOf('"', i + 1);
      if (end === -1) { out += esc(line.slice(i)); break; }
      const str = line.slice(i, end + 1);
      const cls = isKeyContext(line.slice(end + 1)) ? "key" : "str";
      out += `<span class="${cls}">${esc(str)}</span>`;
      i = end + 1;
    } else if (/[0-9]/.test(ch) && (i === 0 || /[\s,:\[]/.test(line[i - 1]))) {
      let j = i;
      while (j < line.length && /[0-9,]/.test(line[j])) j++;
      out += `<span class="num">${esc(line.slice(i, j))}</span>`;
      i = j;
    } else {
      out += esc(ch);
      i++;
    }
  }
  return out;
}
const infoHtmlRaw = demoMeta.infoLines.map(highlight).join("\n");
// Escape so the resulting string is safe inside a JS template literal in the
// emitted browser script (backticks would close the literal, ${ would start an
// unwanted interpolation).
const BACKTICK = String.fromCharCode(96);
const infoHtml = infoHtmlRaw
  .split(BACKTICK).join("\\" + BACKTICK)
  .split("${").join("\\${");
const pngKB = Math.round(demoMeta.pngBytes / 1024);
const displayName = demoMeta.filename;


const landing = `<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <title>xlcore — render xlsx anywhere</title>
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <link rel="preconnect" href="https://fonts.googleapis.com">
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
  <link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Geist:wght@400;500;600;700&family=Geist+Mono:wght@400;500&display=swap">
  <style>
    :root {
      color-scheme: light;
      /* engineer's notebook — warm cream paper, ink text, blueprint accent */
      --paper:   oklch(98% 0.004 80);
      --paper-2: oklch(96% 0.006 80);
      --ink:     oklch(22% 0.012 250);
      --ink-2:   oklch(40% 0.012 250);
      --muted:   oklch(54% 0.008 250);
      --rule:    oklch(88% 0.012 250);
      --rule-2:  oklch(82% 0.014 250);
      --accent:  oklch(46% 0.16 245);
      --accent-soft: oklch(94% 0.04 245);

      /* terminal — light, cool-tinted ivory so it reads as a separate
         surface from the warm paper without going inky. Token colours are
         pushed to ~45% lightness with real chroma so they survive on pale. */
      --term-bg:     oklch(96% 0.006 245);
      --term-chrome: oklch(92% 0.010 245);
      --term-fg:     oklch(28% 0.014 250);
      --term-mute:   oklch(54% 0.010 250);
      --t-prompt:    oklch(48% 0.16 150);  /* green */
      --t-host:      oklch(46% 0.12 220);  /* cyan */
      --t-path:      oklch(46% 0.17 320);  /* magenta */
      --t-cmd:       oklch(50% 0.17 50);   /* amber */
      --t-flag:      oklch(46% 0.18 245);  /* blue */
      --t-str:       oklch(44% 0.11 220);  /* deep cyan */
      --t-key:       oklch(44% 0.19 300);  /* purple */
      --t-num:       oklch(50% 0.17 50);
      --t-ok:        oklch(48% 0.17 150);
    }

    *, *::before, *::after { box-sizing: border-box; }
    html, body { margin: 0; }
    body {
      font-family: "Geist", -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
      background: var(--paper); color: var(--ink);
      min-height: 100vh;
      -webkit-font-smoothing: antialiased;
      font-feature-settings: "ss01", "cv11";
    }
    a { color: var(--accent); text-decoration: none; }
    a:hover { text-decoration: underline; text-underline-offset: 3px; }
    main {
      max-width: 1200px; margin: 0 auto;
      padding: clamp(40px, 6vw, 72px) clamp(20px, 4vw, 32px) 56px;
    }

    header {
      display: flex; align-items: end; justify-content: space-between;
      gap: 32px; margin-bottom: 36px;
      padding-bottom: 24px; border-bottom: 1px solid var(--rule);
    }
    .brand h1 {
      margin: 0; font-weight: 700; letter-spacing: -0.035em;
      font-size: clamp(40px, 5.5vw, 60px); line-height: 1;
    }
    .brand .tag {
      color: var(--ink-2); margin: 14px 0 0;
      font-size: 17px; max-width: 56ch; line-height: 1.55;
    }
    .brand .tag code {
      font-family: "Geist Mono", ui-monospace, monospace;
      font-size: 0.92em; color: var(--accent);
      background: none; padding: 0;
    }
    .meta {
      font-family: "Geist Mono", ui-monospace, monospace;
      font-size: 12px; color: var(--muted);
      display: grid; gap: 4px; text-align: right;
      flex-shrink: 0;
    }
    .meta a { color: var(--ink-2); }
    .meta .ver { color: var(--accent); }

    /* split: demos + quickstart + coverage | terminal + output  (40 / 60) */
    .split {
      display: grid;
      grid-template-columns: minmax(0, 2fr) minmax(0, 3fr);
      gap: clamp(20px, 2.5vw, 32px);
      align-items: stretch;
    }
    @media (max-width: 820px) {
      .split { grid-template-columns: 1fr; align-items: start; }
    }
    .col { display: flex; flex-direction: column; gap: 14px; min-width: 0; }
    .col-right { gap: 14px; }

    /* demo links — hairline-separated rows, not nested cards */
    .demos {
      border: 1px solid var(--rule);
      border-radius: 4px;
      background: var(--paper-2);
      overflow: hidden;
    }
    .demos a {
      display: grid; grid-template-columns: 84px 1fr auto;
      align-items: baseline; gap: 14px;
      padding: 14px 16px; color: var(--ink);
      border-bottom: 1px solid var(--rule);
      transition: background-color 220ms cubic-bezier(0.22, 1, 0.36, 1);
    }
    .demos a:last-child { border-bottom: 0; }
    .demos a:hover { background: var(--accent-soft); text-decoration: none; }
    .demos .route {
      font-family: "Geist Mono", ui-monospace, monospace;
      font-size: 14px; color: var(--accent); font-weight: 500;
    }
    .demos .desc { font-size: 14px; color: var(--ink-2); line-height: 1.45; }
    .demos .arrow {
      font-family: "Geist Mono", ui-monospace, monospace;
      color: var(--muted);
      transition: transform 220ms cubic-bezier(0.22, 1, 0.36, 1), color 220ms;
    }
    .demos a:hover .arrow { transform: translateX(3px); color: var(--accent); }

    /* runtime quickstart — tabbed surface */
    .quick {
      border: 1px solid var(--rule);
      border-radius: 4px;
      background: var(--paper-2);
      display: flex; flex-direction: column;
    }
    .quick-tabs {
      display: flex; border-bottom: 1px solid var(--rule);
      background: var(--paper);
    }
    .quick-tab {
      flex: 1; padding: 10px 14px;
      font-family: "Geist Mono", ui-monospace, monospace;
      font-size: 12px; color: var(--muted);
      background: none; border: 0; cursor: pointer;
      border-right: 1px solid var(--rule);
      text-align: left;
      transition: color 200ms, background-color 200ms, box-shadow 200ms;
    }
    .quick-tab:last-child { border-right: 0; }
    .quick-tab:hover { color: var(--ink-2); }
    .quick-tab[aria-selected="true"] {
      color: var(--ink); background: var(--paper-2);
      box-shadow: inset 0 -2px 0 0 var(--accent);
    }
    .quick-pane {
      display: none; padding: 18px 20px;
      font-family: "Geist Mono", ui-monospace, monospace;
      font-size: 12.5px; line-height: 1.75;
      overflow: auto;
    }
    .quick-pane[data-active] { display: block; }
    .quick pre { margin: 0; white-space: pre-wrap; }
    .quick .k { color: oklch(40% 0.18 295); }
    .quick .s { color: oklch(38% 0.14 220); }
    .quick .c { color: var(--muted); font-style: italic; }
    .quick .f { color: oklch(48% 0.16 35); }
    .quick .p { color: var(--accent); user-select: none; }

    /* coverage grid — what the renderer actually handles */
    .coverage {
      border: 1px solid var(--rule);
      border-radius: 4px;
      background: var(--paper-2);
      padding: 4px 0;
    }
    .cov-head {
      display: flex; justify-content: space-between; align-items: baseline;
      padding: 12px 16px 8px;
      font-family: "Geist Mono", ui-monospace, monospace;
      font-size: 12px; color: var(--muted);
      border-bottom: 1px solid var(--rule);
    }
    .cov-head .h { color: var(--ink); font-weight: 500; letter-spacing: -0.005em; }
    .cov-grid {
      display: grid;
      grid-template-columns: 1fr 1fr;
      column-gap: 1px;
      background: var(--rule);
    }
    .cov-grid > div {
      background: var(--paper-2);
      padding: 10px 16px;
      display: flex; align-items: center; gap: 10px;
      font-size: 13px; color: var(--ink-2);
      border-bottom: 1px solid var(--rule);
    }
    .cov-grid > div:nth-last-child(-n+2) { border-bottom: 0; }
    .cov-grid .ok {
      width: 14px; height: 14px; flex-shrink: 0; border-radius: 50%;
      background: var(--accent-soft);
      display: grid; place-items: center;
      color: var(--accent); font-size: 10px; line-height: 1;
      font-weight: 700;
    }
    .cov-grid .partial .ok {
      background: oklch(94% 0.04 65); color: oklch(50% 0.18 65);
    }
    .cov-grid .partial { color: var(--muted); }

    /* terminal — domain truth, stays inky */
    .term {
      background: var(--term-bg); color: var(--term-fg);
      border-radius: 6px; overflow: hidden;
      border: 1px solid var(--rule);
      box-shadow: 0 1px 0 var(--rule), 0 12px 36px -22px oklch(40% 0.05 250 / 0.18);
      font-family: "Geist Mono", ui-monospace, "SF Mono", Menlo, monospace;
      font-size: 12.5px; line-height: 1.6;
      display: flex; flex-direction: column;
    }
    .term-chrome {
      background: var(--term-chrome); padding: 9px 14px;
      display: flex; align-items: center; gap: 8px;
      border-bottom: 1px solid var(--rule);
    }
    .dot { width: 11px; height: 11px; border-radius: 50%; }
    .dot.r { background: oklch(68% 0.18 28); }
    .dot.y { background: oklch(82% 0.16 88); }
    .dot.g { background: oklch(74% 0.18 145); }
    .term-title {
      margin-left: auto; margin-right: auto; transform: translateX(-22px);
      font-size: 11.5px; color: var(--term-mute);
    }
    .term-body {
      padding: 16px 20px 18px;
      white-space: pre-wrap; word-break: break-word;
    }
    .prompt-user { color: var(--t-prompt); }
    .prompt-at   { color: var(--term-mute); }
    .prompt-host { color: var(--t-host); }
    .prompt-path { color: var(--t-path); }
    .prompt-sym  { color: var(--term-mute); }
    .cmd { color: var(--t-cmd); }
    .flag { color: var(--t-flag); }
    .str { color: var(--t-str); }
    .out { color: var(--term-fg); }
    .ok  { color: var(--t-ok); }
    .key { color: var(--t-key); }
    .num { color: var(--t-num); }
    .cursor {
      display: inline-block; width: 7px; height: 1em;
      background: var(--term-fg); vertical-align: text-bottom;
      margin-left: 1px;
      animation: blink 1.05s steps(1) infinite;
    }
    @keyframes blink { 50% { opacity: 0; } }

    /* rendered output — sharp corners, it's a screenshot.
       The wrap absorbs the right column's remaining height so it matches
       the left column. The image stays its real aspect via object-fit:
       contain with object-position: top — letterbox lands at the bottom
       and disappears against --paper. */
    .out-wrap {
      display: flex; flex-direction: column; gap: 8px;
      flex: 1; min-height: 0;
    }
    .out-img {
      width: 100%; display: block;
      flex: 1; min-height: 0; height: 100%;
      object-fit: contain; object-position: top;
      border: 1px solid var(--rule);
      border-radius: 4px;
      background: var(--paper);
      opacity: 0; transform: translateY(4px);
      transition: opacity 500ms cubic-bezier(0.22, 1, 0.36, 1),
                  transform 500ms cubic-bezier(0.22, 1, 0.36, 1);
    }
    @media (max-width: 820px) {
      .out-wrap { flex: initial; min-height: initial; }
      .out-img { flex: initial; min-height: initial; height: auto; }
    }
    .out-img.show { opacity: 1; transform: none; }
    .out-cap {
      font-family: "Geist Mono", ui-monospace, monospace;
      font-size: 11.5px; color: var(--muted);
      display: flex; justify-content: space-between; gap: 16px;
      padding: 0 2px;
    }
    .out-cap .file { color: var(--ink-2); }

    footer {
      margin-top: 48px; padding-top: 20px;
      border-top: 1px solid var(--rule);
      font-family: "Geist Mono", ui-monospace, monospace;
      font-size: 12px; color: var(--muted);
      display: flex; gap: 20px; flex-wrap: wrap; align-items: baseline;
    }
    footer a { color: var(--ink-2); }
    footer .install { margin-left: auto; color: var(--ink-2); }
    footer .install code { color: var(--accent); }
  </style>
</head>
<body>
<main>
  <header>
    <div class="brand">
      <h1>xlcore</h1>
      <p class="tag">Render <code>.xlsx</code> workbooks to a <code>&lt;canvas&gt;</code> or PNG, in the browser, in Node, or straight from your terminal.</p>
    </div>
    <div class="meta">
      <span class="ver">@hewliyang/xlsx-preview@${packageVersion}</span>
      <span>MIT</span>
    </div>
  </header>

  <div class="split">
    <div class="col col-left">
      <div class="demos">
        <a href="/app">
          <span class="route">/app</span>
          <span class="desc">Full previewer: file picker, sheet tabs, zoom.</span>
          <span class="arrow">→</span>
        </a>
        <a href="/tiles">
          <span class="route">/tiles</span>
          <span class="desc">Thumbnail tile grid of every sheet.</span>
          <span class="arrow">→</span>
        </a>
      </div>

      <div class="quick" role="tablist" aria-label="Runtime quickstart">
        <div class="quick-tabs">
          <button class="quick-tab" role="tab" aria-selected="true"  data-pane="browser">browser</button>
          <button class="quick-tab" role="tab" aria-selected="false" data-pane="react">react</button>
          <button class="quick-tab" role="tab" aria-selected="false" data-pane="node">node</button>
          <button class="quick-tab" role="tab" aria-selected="false" data-pane="cli">cli</button>
        </div>
        <div class="quick-pane" data-pane="browser" data-active><pre><span class="k">import</span> { createWorkbookPreviewerFromFile }
  <span class="k">from</span> <span class="s">"@hewliyang/xlsx-preview/browser"</span>;

<span class="k">await</span> <span class="f">createWorkbookPreviewerFromFile</span>(el, file);
<span class="c">// wasm runs in a Worker; xlsx never leaves the page.</span></pre></div>
        <div class="quick-pane" data-pane="react"><pre><span class="k">import</span> { ExcelPreviewer }
  <span class="k">from</span> <span class="s">"@hewliyang/xlsx-preview/react"</span>;

&lt;<span class="f">ExcelPreviewer</span> file={file} /&gt;
<span class="c">// works in Vite, Next.js, anything that bundles ESM.</span></pre></div>
        <div class="quick-pane" data-pane="node"><pre><span class="k">import</span> { renderXlsxToPng } <span class="k">from</span> <span class="s">"@hewliyang/xlsx-preview"</span>;
<span class="k">import</span> { writeFile } <span class="k">from</span> <span class="s">"node:fs/promises"</span>;

<span class="k">const</span> png = <span class="k">await</span> <span class="f">renderXlsxToPng</span>(<span class="s">"model.xlsx"</span>, { scale: <span class="f">2</span> });
<span class="k">await</span> <span class="f">writeFile</span>(<span class="s">"out.png"</span>, png);</pre></div>
        <div class="quick-pane" data-pane="cli"><pre><span class="p">$</span> npx xlsx-preview model.xlsx <span class="k">--info</span>
<span class="p">$</span> npx xlsx-preview model.xlsx <span class="k">-o</span> cover.png <span class="k">--scale</span> <span class="f">2</span>
<span class="p">$</span> npx xlsx-preview model.xlsx <span class="k">-o</span> <span class="s">"previews/{index}-{sheet}.png"</span> <span class="k">--all</span></pre></div>
      </div>

      <div class="coverage">
        <div class="cov-head">
          <span class="h">What it renders</span>
          <span>parity →</span>
        </div>
        <div class="cov-grid">
          <div><span class="ok">✓</span>Charts · bar, line, pie</div>
          <div><span class="ok">✓</span>Conditional formatting</div>
          <div><span class="ok">✓</span>Dynamic arrays · spill</div>
          <div><span class="ok">✓</span>Tables · filter chrome</div>
          <div><span class="ok">✓</span>Embedded images</div>
          <div><span class="ok">✓</span>Comments · indicators</div>
          <div><span class="ok">✓</span>Outline groups</div>
          <div><span class="ok">✓</span>Number + date formats</div>
          <div><span class="ok">✓</span>Merged cells · wrap</div>
          <div><span class="ok">✓</span>Rich text · sup / sub</div>
          <div class="partial"><span class="ok">~</span>Pivot tables · static</div>
          <div class="partial"><span class="ok">~</span>Sparklines</div>
        </div>
      </div>
    </div>

    <div class="col col-right">
      <div class="term" aria-label="xlsx-preview CLI demo">
        <div class="term-chrome">
          <span class="dot r"></span><span class="dot y"></span><span class="dot g"></span>
          <span class="term-title">~ — xlsx-preview — zsh</span>
        </div>
        <div class="term-body" id="term"></div>
      </div>
      <div class="out-wrap">
        <img id="out-img" class="out-img" alt="rendered ${displayName}" src="/demo/cover.png" loading="lazy">
        <div class="out-cap">
          <span class="file">cover.png</span>
          <span>${demoMeta.sheetName} · ${pngKB} KB · scale 2</span>
        </div>
      </div>
    </div>
  </div>

  <footer>
    <a href="https://github.com/hewliyang/xlcore">GitHub ↗</a>
    <a href="https://www.npmjs.com/package/@hewliyang/xlsx-preview">npm ↗</a>
    <span class="install">npm i <code>@hewliyang/xlsx-preview</code></span>
  </footer>
</main>

<script>
(() => {
  const term = document.getElementById("term");
  const PROMPT = '<span class="prompt-user">you</span><span class="prompt-at">@</span><span class="prompt-host">mac</span> <span class="prompt-path">~/work</span> <span class="prompt-sym">❯</span> ';

  // each step: typed command (with html spans) + delay + output html
  const steps = [
    {
      cmd: '<span class="cmd">xlsx-preview</span> <span class="str">${displayName}</span> <span class="flag">-o</span> <span class="str">cover.png</span> <span class="flag">--sheet</span> <span class="str">${demoMeta.sheetName}</span> <span class="flag">--scale</span> <span class="num">2</span>',
      out: \`<span class="ok">✓</span> <span class="out">rendered <span class="str">${demoMeta.sheetName}</span> → <span class="str">cover.png</span> (${pngKB} KB)</span>\`,
      reveal: true,
    },
  ];

  const TYPE_MS = 28;
  const PAUSE_AFTER_CMD = 380;
  const PAUSE_AFTER_OUT = 900;

  const sleep = (ms) => new Promise(r => setTimeout(r, ms));

  // Type a string char-by-char while preserving inline html spans.
  // We tokenise into [text|tagOpen|tagClose] and stream characters,
  // re-rendering with a trailing cursor.
  function tokenise(html) {
    const tokens = [];
    let i = 0;
    while (i < html.length) {
      if (html[i] === '<') {
        const end = html.indexOf('>', i);
        tokens.push({ tag: html.slice(i, end + 1) });
        i = end + 1;
      } else {
        tokens.push({ ch: html[i] });
        i++;
      }
    }
    return tokens;
  }

  async function typeInto(lineEl, html) {
    const tokens = tokenise(html);
    let shown = "";
    for (const t of tokens) {
      if (t.tag) {
        shown += t.tag;
        lineEl.innerHTML = shown + '<span class="cursor"></span>';
      } else {
        shown += t.ch;
        lineEl.innerHTML = shown + '<span class="cursor"></span>';
        await sleep(TYPE_MS);
      }
    }
    // strip cursor at end of typing
    lineEl.innerHTML = shown;
  }

  function appendLine(html = "") {
    const div = document.createElement("div");
    div.innerHTML = html;
    term.appendChild(div);
    return div;
  }

  async function run() {
    for (const step of steps) {
      const promptLine = appendLine(PROMPT);
      const cmdSpan = document.createElement("span");
      promptLine.appendChild(cmdSpan);
      await typeInto(cmdSpan, step.cmd);
      await sleep(PAUSE_AFTER_CMD);
      appendLine(step.out);
      if (step.reveal) {
        const img = document.getElementById('out-img');
        if (img) requestAnimationFrame(() => img.classList.add('show'));
      }
      await sleep(PAUSE_AFTER_OUT);
      appendLine();
    }
    // final blinking prompt
    const final = appendLine(PROMPT);
    final.insertAdjacentHTML("beforeend", '<span class="cursor"></span>');
  }

  // Render the final state immediately when the tab can't play motion:
  // a hidden tab (setTimeout clamps to 1s+) or reduced-motion preference
  // would otherwise show a frozen prompt to anyone returning to the page.
  function renderFinal() {
    for (const step of steps) {
      const line = appendLine(PROMPT);
      const span = document.createElement('span');
      span.innerHTML = step.cmd;
      line.appendChild(span);
      appendLine(step.out);
      if (step.reveal) {
        const img = document.getElementById('out-img');
        if (img) img.classList.add('show');
      }
      appendLine();
    }
    const final = appendLine(PROMPT);
    final.insertAdjacentHTML('beforeend', '<span class="cursor"></span>');
  }

  const prefersStill = matchMedia('(prefers-reduced-motion: reduce)').matches;
  if (document.hidden || prefersStill) {
    renderFinal();
  } else {
    const termRoot = term.closest('.term') || term;
    const io = new IntersectionObserver((entries, obs) => {
      for (const e of entries) {
        if (e.isIntersecting) { obs.disconnect(); run(); }
      }
    }, { threshold: 0.1 });
    io.observe(termRoot);
  }

  // quickstart tab switching
  const tabs = document.querySelectorAll('.quick-tab');
  const panes = document.querySelectorAll('.quick-pane');
  tabs.forEach((tab) => {
    tab.addEventListener('click', () => {
      const target = tab.dataset.pane;
      tabs.forEach((t) => t.setAttribute('aria-selected', t === tab ? 'true' : 'false'));
      panes.forEach((p) => {
        if (p.dataset.pane === target) p.setAttribute('data-active', '');
        else p.removeAttribute('data-active');
      });
    });
  });
})();
</script>
</body>
</html>
`;
await writeFile(resolve(out, "index.html"), landing);
console.log(`  /      ←  landing`);
console.log(`\nsite/ ready (${out})`);
