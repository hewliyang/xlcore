#!/usr/bin/env node
import { readdir, stat } from "node:fs/promises";
import { dirname, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, "../../..");
const wasmPath = resolve(repoRoot, "crates/xlcore-wasm/pkg/xlcore_wasm_bg.wasm");
const watchedPaths = [
  "Cargo.lock",
  "Cargo.toml",
  "crates/xlcore-export/Cargo.toml",
  "crates/xlcore-export/src",
  "crates/xlcore-io/Cargo.toml",
  "crates/xlcore-io/src",
  "crates/xlcore-wasm/Cargo.toml",
  "crates/xlcore-wasm/src",
];

const wasmStat = await stat(wasmPath).catch(() => null);
if (!wasmStat) {
  fail(`missing ${relative(repoRoot, wasmPath)}`);
}

const newest = await newestInput();
if (newest && newest.mtimeMs > wasmStat.mtimeMs) {
  fail(`${newest.path} is newer than ${relative(repoRoot, wasmPath)}`);
}

async function newestInput() {
  let newest = null;
  for (const path of watchedPaths) {
    const abs = resolve(repoRoot, path);
    for await (const file of walk(abs)) {
      const s = await stat(file);
      if (!newest || s.mtimeMs > newest.mtimeMs) {
        newest = { path: relative(repoRoot, file), mtimeMs: s.mtimeMs };
      }
    }
  }
  return newest;
}

async function* walk(path) {
  const s = await stat(path);
  if (s.isDirectory()) {
    for (const entry of await readdir(path)) {
      yield* walk(resolve(path, entry));
    }
  } else if (isWatchedFile(path)) {
    yield path;
  }
}

function isWatchedFile(path) {
  return /\.(rs|toml|lock)$/.test(path);
}

function fail(reason) {
  console.error(
    [
      `WASM bundle is stale: ${reason}.`,
      "Run `pnpm --filter @hewliyang/xlsx-preview run build:wasm` before testing.",
    ].join("\n"),
  );
  process.exit(1);
}
