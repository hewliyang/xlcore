import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";

const MAX_LINES = 800;
const ROOT = join(import.meta.dir, "..");
const CHECK_DIRS = ["src"];
const EXTENSIONS = new Set([".ts", ".tsx", ".js", ".jsx", ".json"]);

function extension(path: string): string {
  const idx = path.lastIndexOf(".");
  return idx === -1 ? "" : path.slice(idx);
}

function walk(dir: string): string[] {
  const out: string[] = [];
  for (const entry of readdirSync(dir)) {
    const path = join(dir, entry);
    const stat = statSync(path);
    if (stat.isDirectory()) {
      if (entry === "dist" || entry === "node_modules") continue;
      out.push(...walk(path));
    } else if (EXTENSIONS.has(extension(path))) {
      out.push(path);
    }
  }
  return out;
}

const tooLarge = CHECK_DIRS.flatMap((dir) => walk(join(ROOT, dir)))
  .map((path) => {
    const text = readFileSync(path, "utf8");
    const lines = text.length === 0 ? 0 : text.split(/\r\n|\r|\n/).length;
    return { path, lines };
  })
  .filter(({ lines }) => lines > MAX_LINES)
  .sort((a, b) => b.lines - a.lines);

if (tooLarge.length > 0) {
  console.error(`Files over ${MAX_LINES} LoC:`);
  for (const { path, lines } of tooLarge) {
    console.error(`${String(lines).padStart(5)}  ${relative(ROOT, path)}`);
  }
  process.exit(1);
}

console.log(`All checked files are <= ${MAX_LINES} LoC.`);
