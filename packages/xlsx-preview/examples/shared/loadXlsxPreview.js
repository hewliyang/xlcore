

const CDN_BASE = "https://cdn.jsdelivr.net/npm/@hewliyang/xlsx-preview@0.0.9/dist/";

function isDevHost() {
  if (typeof location === "undefined") return false;
  const proto = location.protocol;
  if (proto === "file:") return true;
  const host = location.hostname;
  return (
    host === "localhost" ||
    host === "0.0.0.0" ||
    host === "::1" ||
    host.startsWith("127.") ||

    host.endsWith(".local")
  );
}

function urlParam(name) {
  if (typeof location === "undefined") return null;
  return new URLSearchParams(location.search).get(name);
}

export async function loadXlsxPreviewRuntime(options = {}) {
  const localBase = urlParam("dist") ?? options.localBase ?? "../dist/";
  const modules = options.modules ?? ["browserLoader.js", "color.js"];
  const forceCdn = urlParam("cdn") === "1";
  const dev = isDevHost();

  if (!forceCdn) {
    try {
      const mods = await importAll(localBase, modules);
      return {
        assetBase: localBase,
        source: "local",
        ...mods,
      };
    } catch (error) {
      if (dev) {

        const reason = error && error.message ? error.message : String(error);
        const msg =
          `Failed to load local xlsx-preview runtime from "${localBase}": ${reason}\n\n` +
          `Did you run \`pnpm run build\` (or \`build:release\` for a full wasm + ts build)?\n` +
          `\n` +
          `To bypass this check and load the published CDN copy instead, append ?cdn=1 to the URL.`;
        throw new Error(msg);
      }

      console.warn(
        `xlsx-preview: local runtime at ${localBase} failed to load, falling back to CDN`,
        error,
      );
    }
  }

  const mods = await importAll(CDN_BASE, modules);
  return {
    assetBase: CDN_BASE,
    source: "cdn",
    ...mods,
  };
}

async function importAll(base, modules) {

  const cacheBust = isDevHost() ? `?v=${Date.now()}` : "";

  const pageBase =
    typeof document !== "undefined" && document.baseURI ? document.baseURI : base;
  const out = {};
  for (const file of modules) {
    const url = new URL(`${base}${file}${cacheBust}`, pageBase).href;
    const mod = await import(url);
    Object.assign(out, mod);
  }
  return out;
}
