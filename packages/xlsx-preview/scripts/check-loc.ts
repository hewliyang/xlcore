import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, extname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const MAX_LINES = 900;
const PACKAGE_ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const WORKSPACE_ROOT = join(PACKAGE_ROOT, "..", "..");

const CHECK_TARGETS = [
  {
    root: PACKAGE_ROOT,
    dirs: ["src"],
    extensions: new Set([".ts", ".tsx", ".js", ".jsx", ".json"]),
  },
  {
    root: WORKSPACE_ROOT,
    dirs: ["crates"],
    extensions: new Set([".rs"]),
  },
];

const SKIP_DIRS = new Set(["dist", "node_modules", "pkg", "target"]);
const SKIP_FILES = new Set(["world110m.ts"]);

function walk(dir: string, extensions: Set<string>): string[] {
  const out: string[] = [];
  for (const entry of readdirSync(dir)) {
    if (SKIP_DIRS.has(entry) || SKIP_FILES.has(entry)) continue;

    const path = join(dir, entry);
    const stat = statSync(path);
    if (stat.isDirectory()) {
      out.push(...walk(path, extensions));
    } else if (extensions.has(extname(path))) {
      out.push(path);
    }
  }
  return out;
}

const tooLarge = CHECK_TARGETS.flatMap(({ root, dirs, extensions }) =>
  dirs.flatMap((dir) => walk(join(root, dir), extensions)),
)
  .map((path) => {
    const text = readFileSync(path, "utf8");
    const lines = text.length === 0 ? 0 : text.replace(/\r\n|\r|\n$/, "").split(/\r\n|\r|\n/).length;
    return { path, lines };
  })
  .filter(({ lines }) => lines > MAX_LINES)
  .sort((a, b) => b.lines - a.lines);

if (tooLarge.length > 0) {
  console.error(`Files over ${MAX_LINES} LoC:`);
  for (const { path, lines } of tooLarge) {
    console.error(`${String(lines).padStart(5)}  ${relative(WORKSPACE_ROOT, path)}`);
  }
  process.exit(1);
}

console.log(`All checked files are <= ${MAX_LINES} LoC.`);
