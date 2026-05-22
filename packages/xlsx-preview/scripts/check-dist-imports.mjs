#!/usr/bin/env node

const entries = [
  "../dist/index.js",
  "../dist/node.js",
  "../dist/browserLoader.js",
  "../dist/previewer.js",
  "../dist/react.js",
  "../dist/cdn.js",
];

for (const entry of entries) {
  await import(new URL(entry, import.meta.url));
}

const rootSource = await import("node:fs/promises").then((fs) =>
  fs.readFile(new URL("../dist/index.js", import.meta.url), "utf8"),
);
if (rootSource.includes("skia-canvas") || rootSource.includes("node:fs")) {
  throw new Error("dist/index.js must stay browser-safe; import Node helpers from ./node");
}
