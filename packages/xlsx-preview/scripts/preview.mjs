#!/usr/bin/env node
import { createServer } from "node:http";
import { createReadStream } from "node:fs";
import { stat } from "node:fs/promises";
import { extname, join, normalize, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = resolve(fileURLToPath(import.meta.url), "../../../..");
const preferredPort = Number(process.env.PORT || 8765);
const host = process.env.HOST || "127.0.0.1";

const mimes = new Map([
  [".html", "text/html; charset=utf-8"],
  [".js", "text/javascript; charset=utf-8"],
  [".mjs", "text/javascript; charset=utf-8"],
  [".css", "text/css; charset=utf-8"],
  [".json", "application/json; charset=utf-8"],
  [".wasm", "application/wasm"],
  [".png", "image/png"],
  [".jpg", "image/jpeg"],
  [".jpeg", "image/jpeg"],
  [".gif", "image/gif"],
  [".svg", "image/svg+xml"],
  [".xlsx", "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"],
]);

function servePath(urlPath) {
  const pathname = decodeURIComponent(new URL(urlPath, "http://localhost").pathname);
  const rel = normalize(pathname).replace(/^\/+/, "");
  const resolved = resolve(repoRoot, rel || "packages/xlsx-preview/examples/xlsx-app.html");
  if (!resolved.startsWith(repoRoot)) return null;
  return resolved;
}

const server = createServer(async (req, res) => {
  const path = servePath(req.url || "/");
  if (!path) {
    res.writeHead(403).end("Forbidden");
    return;
  }
  try {
    const s = await stat(path);
    const file = s.isDirectory() ? join(path, "index.html") : path;
    res.writeHead(200, {
      "content-type": mimes.get(extname(file)) || "application/octet-stream",
      "cache-control": "no-store",
    });
    createReadStream(file).pipe(res);
  } catch {
    res.writeHead(404, { "content-type": "text/plain; charset=utf-8" }).end("Not found");
  }
});

let shuttingDown = false;
function shutdown(signal) {
  if (shuttingDown) return;
  shuttingDown = true;
  console.log(`\n${signal}: stopping xlsx-preview server`);

  // End keep-alive/browser connections so Node can actually exit after Ctrl+C.
  server.closeIdleConnections?.();
  server.closeAllConnections?.();

  server.close((error) => {
    if (error) {
      console.error(error);
      process.exit(1);
    }
    process.exit(0);
  });

  // Don't hang forever if a client keeps a socket open.
  setTimeout(() => process.exit(0), 1000).unref();
}

process.once("SIGINT", () => shutdown("SIGINT"));
process.once("SIGTERM", () => shutdown("SIGTERM"));

async function listen(port) {
  return await new Promise((resolveListen, rejectListen) => {
    server.once("error", rejectListen);
    server.listen(port, host, () => {
      server.off("error", rejectListen);
      resolveListen(port);
    });
  });
}

let port = preferredPort;
for (;;) {
  try {
    await listen(port);
    break;
  } catch (error) {
    if (error?.code !== "EADDRINUSE" || port >= preferredPort + 20) throw error;
    port++;
  }
}

const url = `http://${host}:${port}/packages/xlsx-preview/examples/xlsx-app.html`;
console.log(`xlsx-preview example: ${url}`);
if (port !== preferredPort) console.log(`port ${preferredPort} was busy; using ${port}`);
