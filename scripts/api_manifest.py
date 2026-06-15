#!/usr/bin/env python3
import argparse
import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
LIB_RS = os.path.join(HERE, "..", "crates", "xlcore-wasm", "src", "lib.rs")
MANIFEST_JSON = os.path.join(HERE, "api_methods.json")
TS_SRC = os.path.join(HERE, "..", "packages", "xlsx-preview", "src")

KIND_TYPES = {
    "s": "&str",
    "os": "Option<String>",
    "u32": "u32",
    "u8": "u8",
    "usize": "usize",
    "f64": "f64",
    "bool": "bool",
}


def read_lib():
    with open(LIB_RS, encoding="utf-8") as fh:
        return fh.read()


def split_top_level(text, sep):
    out, depth, buf = [], 0, ""
    for ch in text:
        if ch in "<([{":
            depth += 1
        elif ch in ">)]}":
            depth -= 1
        if ch == sep and depth == 0:
            out.append(buf)
            buf = ""
        else:
            buf += ch
    if buf.strip():
        out.append(buf)
    return out


def slice_block(text, open_token):
    start = text.index(open_token) + len(open_token) - 1
    depth = 0
    for i in range(start, len(text)):
        if text[i] == "{":
            depth += 1
        elif text[i] == "}":
            depth -= 1
            if depth == 0:
                return text[start + 1 : i]
    raise SystemExit(f"unbalanced block for {open_token!r}")


def parse_table_args(inner):
    inner = inner.strip()
    if not inner:
        return []
    args = []
    for part in split_top_level(inner, ","):
        part = part.strip()
        if not part:
            continue
        head = part.split(None, 1)
        kind = head[0]
        rest = head[1] if len(head) > 1 else ""
        if kind in ("de", "deopt"):
            name, ty = rest.split(":", 1)
            args.append({"name": name.strip(), "kind": kind, "type": ty.strip()})
        else:
            args.append({"name": rest.strip(), "kind": kind, "type": KIND_TYPES[kind]})
    return args


def parse_table(lib):
    block = slice_block(lib[lib.index("api_methods! {") :], "api_methods! {")
    rows = []
    for raw in block.split("}"):
        raw = raw.strip()
        if not raw.startswith("{"):
            continue
        body = raw[1:].strip()
        m = re.match(
            r'^(\w+)(?:\s+as\s+"([^"]+)")?\s*\((.*)\)\s*->\s*(\w+)$', body, re.DOTALL
        )
        if not m:
            raise SystemExit(f"unparsed table row: {body!r}")
        name, js, inner, ret = m.groups()
        rows.append(
            {
                "name": name,
                "jsName": js or name,
                "source": "table",
                "kind": "method",
                "args": parse_table_args(inner),
                "ret": ret,
            }
        )
    return rows


def parse_handwritten_args(params):
    args = []
    for part in split_top_level(params, ","):
        part = part.strip()
        if not part or part in ("self", "&self", "&mut self"):
            continue
        if ":" not in part:
            continue
        name, ty = part.split(":", 1)
        args.append({"name": name.strip(), "kind": "raw", "type": ty.strip()})
    return args


def parse_handwritten(lib):
    needle = "// Hand-written bindings"
    block = slice_block(lib[lib.index(needle) :], "impl WorkbookHandle {")
    method_re = re.compile(
        r"(?:#\[wasm_bindgen\(([^\]]*)\)\]\s*)?pub fn (\w+)\s*\(([^)]*)\)"
        r"(?:\s*->\s*([^{]+?))?\s*\{",
        re.DOTALL,
    )
    rows = []
    for m in method_re.finditer(block):
        attr, name, params, ret = m.groups()
        attr = attr or ""
        ret = (ret or "()").strip()
        if "constructor" in attr:
            kind, js = "constructor", None
        else:
            params_has_self = bool(
                re.match(r"^\s*&?\s*(mut\s+)?self\b", params.strip())
            )
            kind = "method" if params_has_self else "static"
            jm = re.search(r'js_name\s*=\s*"?(\w+)"?', attr)
            js = jm.group(1) if jm else name
        rows.append(
            {
                "name": name,
                "jsName": js,
                "source": "handwritten",
                "kind": kind,
                "args": parse_handwritten_args(params),
                "ret": ret,
            }
        )
    return rows


def build_manifest():
    lib = read_lib()
    return parse_handwritten(lib) + parse_table(lib)


def collect_ts_calls():
    calls = {}
    pat = re.compile(r"\bhandle\.(\w+)\s*\(")
    for root, _dirs, files in os.walk(TS_SRC):
        for fn in files:
            if not (fn.endswith(".ts") or fn.endswith(".tsx")):
                continue
            path = os.path.join(root, fn)
            with open(path, encoding="utf-8") as fh:
                for name in pat.findall(fh.read()):
                    calls.setdefault(name, []).append(
                        os.path.relpath(path, os.path.join(HERE, ".."))
                    )
    return calls


def write_manifest():
    manifest = build_manifest()
    with open(MANIFEST_JSON, "w", encoding="utf-8") as fh:
        json.dump(manifest, fh, indent=2)
        fh.write("\n")
    print(f"wrote {len(manifest)} methods to {os.path.relpath(MANIFEST_JSON)}")


def run_check():
    manifest = build_manifest()
    failures = []

    if os.path.exists(MANIFEST_JSON):
        with open(MANIFEST_JSON, encoding="utf-8") as fh:
            checked = json.load(fh)
    else:
        checked = None
    if checked != manifest:
        failures.append("manifest stale: regenerate with scripts/api_manifest.py")
        print("FAIL manifest out of date vs lib.rs; run scripts/api_manifest.py")
        cur = {m["name"]: m for m in manifest}
        old = {m["name"]: m for m in (checked or [])}
        for k in sorted(set(cur) | set(old)):
            if cur.get(k) != old.get(k):
                print(f"  changed: {k}")

    js_names = {m["jsName"] for m in manifest if m["kind"] == "method"}
    ts_calls = collect_ts_calls()

    uncalled = sorted(js_names - set(ts_calls))
    if uncalled:
        failures.append(f"manifest methods never forwarded in TS: {uncalled}")
        print(f"FAIL not forwarded in TS: {', '.join(uncalled)}")

    manifest_js = {m["jsName"] for m in manifest if m["jsName"]}
    phantom = sorted(set(ts_calls) - manifest_js)
    if phantom:
        failures.append(f"TS handle.<name>() calls not in manifest: {phantom}")
        for name in phantom:
            print(f"FAIL phantom handle.{name}(): {', '.join(ts_calls[name])}")

    if failures:
        print(f"\n{len(failures)} failure(s)", file=sys.stderr)
        return 1
    print(
        f"ok {len(manifest)} methods; "
        f"{len(js_names)} forwarded jsNames matched in TS"
    )
    return 0


def main():
    ap = argparse.ArgumentParser(
        description="Emit/check the xlcore-wasm WorkbookHandle method manifest."
    )
    ap.add_argument("--check", action="store_true", help="diff manifest + cross-check TS")
    args = ap.parse_args()
    if args.check:
        raise SystemExit(run_check())
    write_manifest()


if __name__ == "__main__":
    main()
