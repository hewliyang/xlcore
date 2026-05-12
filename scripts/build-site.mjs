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
// The examples try `../dist/browser-loader.js` first (for local dev under
// `pnpm preview`); that 404s on Pages and the page falls through to the
// jsdelivr-hosted npm bundle pinned to the published version. We don't
// ship dist/ here — jsdelivr is globally edge-cached and pinned by
// `@version`, so re-hosting the 17 MB wasm would just duplicate it.

import { cp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, "..");
const out = resolve(repo, "site");
const examples = resolve(repo, "packages/xlsx-preview/examples");

const routes = [
  ["app", "xlsx-app.html"],
  ["tiles", "xlsx-tiles.html"],
  ["multi", "xlsx-multi.html"],
];

async function exists(p) {
  try {
    await readFile(p);
    return true;
  } catch {
    return false;
  }
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
  console.log(`  /${route}  ←  ${file}`);
}

const landing = `<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <title>xlcore</title>
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <link rel="preconnect" href="https://rsms.me/">
  <link rel="stylesheet" href="https://rsms.me/inter/inter.css">
  <style>
    :root { color-scheme: light dark; }
    html, body { margin: 0; height: 100%; }
    body {
      font-family: "Inter", -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
      display: grid; place-items: center;
      background: #f7f8fa; color: #0f172a;
    }
    @media (prefers-color-scheme: dark) {
      body { background: #0b0f17; color: #e6edf3; }
      a { color: #93c5fd; }
    }
    main { padding: 40px; max-width: 560px; }
    h1 { margin: 0 0 8px; font-weight: 600; letter-spacing: -0.02em; }
    p { color: #64748b; margin: 0 0 24px; }
    ul { padding: 0; list-style: none; display: grid; gap: 8px; }
    li a {
      display: block; padding: 12px 16px; border-radius: 8px;
      background: rgba(127,127,127,0.08); text-decoration: none;
      color: inherit; font-weight: 500;
    }
    li a:hover { background: rgba(127,127,127,0.16); }
    li a small { display: block; font-weight: 400; opacity: 0.7; margin-top: 2px; }
    code {
      background: rgba(127,127,127,0.12); padding: 2px 6px; border-radius: 4px;
      font-family: ui-monospace, "JetBrains Mono", monospace; font-size: 12px;
    }
  </style>
</head>
<body>
<main>
  <h1>xlcore</h1>
  <p>Canvas-based XLSX previewer. Powered by <code>@hewliyang/xlsx-preview</code>.</p>
  <ul>
    <li><a href="/app">/app<small>Full previewer (file picker, sheet tabs, zoom)</small></a></li>
    <li><a href="/tiles">/tiles<small>Thumbnail tile grid</small></a></li>
    <li><a href="/multi">/multi<small>Multiple workbooks side-by-side</small></a></li>
  </ul>
  <p style="margin-top: 32px;">
    <a href="https://github.com/hewliyang/xlcore">GitHub</a> ·
    <a href="https://www.npmjs.com/package/@hewliyang/xlsx-preview">npm</a>
  </p>
</main>
</body>
</html>
`;
await writeFile(resolve(out, "index.html"), landing);
console.log(`  /      ←  landing`);
console.log(`\nsite/ ready (${out})`);
