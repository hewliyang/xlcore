#!/usr/bin/env python3
import argparse
import glob
import os
import re
import sys
import tomllib


def find_ooxmlsdk_root(override):
    if override:
        return override
    lock = os.path.join(os.path.dirname(__file__), "..", "Cargo.lock")
    version = None
    with open(lock, "rb") as fh:
        data = tomllib.load(fh)
    for pkg in data.get("package", []):
        if pkg.get("name") == "ooxmlsdk":
            version = pkg.get("version")
            break
    if version is None:
        raise SystemExit("ooxmlsdk not found in Cargo.lock")
    home = os.path.expanduser("~/.cargo/registry/src")
    matches = glob.glob(os.path.join(home, "*", f"ooxmlsdk-{version}"))
    if not matches:
        raise SystemExit(f"ooxmlsdk-{version} not found under {home}")
    return matches[0]


def leaf(qname):
    seg = qname.split("/")[-1]
    if ":" in seg:
        seg = seg.split(":", 1)[1]
    return seg


def norm(s):
    return re.sub(r"[^a-z0-9]", "", s.lower())


FIELD_RE = re.compile(r"\n[ \t]*pub (?:r#)?(\w+):\s*([^\n]+),")
EXTLST = "extlst"
# ooxmlsdk serialization plumbing carried on every part/struct, never OOXML
# schema content; ignored globally so per-pair manifests stay schema-only.
PLUMBING = {"xmlns", "xmlheader", "xmlotherattrs", "xmlotherchildren"}


def base_type(ftype):
    inner = re.sub(r"\bstd::boxed::Box\b|\bOption\b|\bVec\b", "", ftype)
    inner = inner.replace("<", " ").replace(">", " ").strip()
    if not inner:
        return ""
    inner = inner.split()[-1]
    return inner.split("::")[-1].strip()


def parse_fields(body):
    fields = []
    matches = list(FIELD_RE.finditer(body))
    for i, fm in enumerate(matches):
        fname, ftype = fm.group(1), fm.group(2).strip()
        prev_end = matches[i - 1].end() if i else 0
        attrs = body[prev_end : fm.start()]
        qnames = re.findall(r'qname = "([^"]+)"', attrs)
        tags = [leaf(q) for q in qnames]
        fields.append(
            {
                "field": fname,
                "tags": tags,
                "optional": ftype.startswith("Option"),
                "vec": ftype.startswith("Vec"),
                "choice": "choice(" in attrs,
                "base": base_type(ftype),
            }
        )
    return fields


_FILE_CACHE = {}


def schema_files(root):
    return sorted(glob.glob(os.path.join(root, "src", "schemas", "*.rs")))


def file_text(path):
    if path not in _FILE_CACHE:
        with open(path, encoding="utf-8") as fh:
            _FILE_CACHE[path] = fh.read()
    return _FILE_CACHE[path]


def find_struct(text, name):
    m = re.search(
        r'(#\[sdk\([^\]]*?qname = "([^"]+)"[^\]]*\)\]\s*)?pub struct '
        + re.escape(name)
        + r" \{(.*?)\n\}",
        text,
        re.DOTALL,
    )
    if not m:
        return None
    return m.group(2) or "", m.group(3)


def parse_sdk_struct(root, name, ns=None):
    candidates = []
    for path in schema_files(root):
        found = find_struct(file_text(path), name)
        if not found:
            continue
        qname, body = found
        if ns and not qname.startswith(f"{ns}:"):
            continue
        candidates.append((os.path.basename(path), qname, parse_fields(body), path))
    if not candidates:
        raise SystemExit(f"sdk struct '{name}' not found under {root}/src/schemas")
    canonical = [c for c in candidates if c[0].startswith("schemas_openxmlformats_org")]
    chosen = (canonical or candidates)[0]
    if len(candidates) > 1:
        others = ", ".join(c[0] for c in candidates if c is not chosen)
        print(f"# note: '{name}' also defined in: {others}", file=sys.stderr)
    return chosen


def resolve_child_struct(root, name, prefer_path):
    if prefer_path:
        found = find_struct(file_text(prefer_path), name)
        if found:
            return found[1]
    for path in schema_files(root):
        found = find_struct(file_text(path), name)
        if found:
            return found[1]
    return None


DTO_FIELD_RE = re.compile(r"\n    pub (\w+):")
ANNOT_RE = re.compile(r"schema-(excluded|derived):\s*([^\n*]+)")


def parse_dto_struct(name):
    src = os.path.join(os.path.dirname(__file__), "..", "crates", "xlcore-types", "src")
    for path in sorted(glob.glob(os.path.join(src, "*.rs"))):
        text = file_text(path)
        m = re.search(
            r"((?:^[ \t]*///[^\n]*\n)*)pub struct " + re.escape(name) + r" \{(.*?)\n\}",
            text,
            re.DOTALL | re.MULTILINE,
        )
        if not m:
            continue
        doc, body = m.group(1), m.group(2)
        fields = [fm.group(1) for fm in DTO_FIELD_RE.finditer("\n" + body)]
        excluded, derived = [], []
        for kind, items in ANNOT_RE.findall(doc):
            toks = [t.strip() for t in items.replace(",", " ").split() if t.strip()]
            (excluded if kind == "excluded" else derived).extend(toks)
        return fields, excluded, derived
    raise SystemExit(f"dto struct '{name}' not found in xlcore-types/src")


def keys_of(field):
    return [field["field"]] + field["tags"]


def in_set(field, normset):
    return any(norm(k) in normset for k in keys_of(field))


def covered_direct(field, dto_norms, alias_norms):
    return in_set(field, dto_norms) or in_set(field, alias_norms)


def flatten_children(field, root, prefer_path):
    if field["choice"] and field["vec"]:
        return [{"field": t, "tags": [t]} for t in field["tags"]]
    if not field["vec"] and field["base"]:
        body = resolve_child_struct(root, field["base"], prefer_path)
        if body:
            return parse_fields(body)
    return None


def child_keys(child):
    return [child["field"]] + child.get("tags", [])


def classify(field, root, prefer_path, dto_norms, alias_norms, excl_norms, deriv_norms):
    if in_set(field, deriv_norms):
        return "derived", False
    keynorms = [norm(k) for k in keys_of(field)]
    if any(k == EXTLST or k in PLUMBING for k in keynorms):
        return "excluded", False
    if in_set(field, excl_norms):
        return "excluded", False
    children = flatten_children(field, root, prefer_path)
    choice_vec = field["choice"] and field["vec"]
    if not choice_vec and covered_direct(field, dto_norms, alias_norms):
        return "covered", False
    if children:
        m = n = 0
        for c in children:
            cnorms = [norm(k) for k in child_keys(c)]
            if any(x == EXTLST for x in cnorms) or any(x in excl_norms for x in cnorms):
                continue
            m += 1
            if any(x in dto_norms or x in alias_norms for x in cnorms):
                n += 1
        if m and n:
            return f"flattened ({n}/{m})", False
    if covered_direct(field, dto_norms, alias_norms):
        return "covered", False
    return "MISSING", True


def analyze(root, sdk_struct, dto_struct, ns, derived, excluded, aliases):
    filename, qname, sdk_fields, prefer_path = parse_sdk_struct(root, sdk_struct, ns)
    dto_fields, doc_excl, doc_deriv = parse_dto_struct(dto_struct)
    dto_norms = {norm(f) for f in dto_fields}
    alias_norms = {norm(k) for k in aliases}
    excl_norms = {norm(x) for x in list(excluded) + doc_excl}
    deriv_norms = {norm(x) for x in list(derived) + doc_deriv}
    rows = []
    for f in sdk_fields:
        status, gap = classify(
            f, root, prefer_path, dto_norms, alias_norms, excl_norms, deriv_norms
        )
        rows.append((f, status, gap))
    return {
        "filename": filename,
        "qname": qname,
        "rows": rows,
        "dto_fields": dto_fields,
        "sdk_fields": sdk_fields,
    }


def load_manifest_full():
    path = os.path.join(os.path.dirname(__file__), "schema_coverage.toml")
    with open(path, "rb") as fh:
        return tomllib.load(fh)


def load_manifest():
    return load_manifest_full().get("pair", [])


CHART_AUDIT_ROOTS = ["Chart"]


def chart_schema_file(root):
    for p in schema_files(root):
        if "drawingml_2006_chart" in os.path.basename(p):
            return p
    raise SystemExit("chart schema file not found under ooxmlsdk")


def build_chart_graph(path):
    text = file_text(path)
    structs = {}
    for m in re.finditer(
        r'#\[sdk\(qname = "([^"]+)"\)\]\s*pub struct (\w+) \{(.*?)\n\}', text, re.DOTALL
    ):
        structs[m.group(2)] = (m.group(1), m.group(3))
    enums = {}
    for m in re.finditer(r"pub enum (\w+) \{(.*?)\n\}", text, re.DOTALL):
        variants = [b for _a, b in re.findall(r"(\w+)\((?:std::boxed::Box<)?(\w+)>?\)", m.group(2))]
        if variants:
            enums[m.group(1)] = variants
    return structs, enums


def field_basetypes(body):
    return [base_type(fm.group(1)) for fm in re.finditer(r"pub (?:r#)?\w+:\s*([^\n,]+),", body)]


def is_container(name, structs, enums):
    if name.endswith(("Extension", "ExtensionList")):
        return False
    for x in field_basetypes(structs[name][1]):
        if x in enums:
            return True
        if x in structs and not x.endswith(("Extension", "ExtensionList")):
            return True
    return False


def reachable_containers(structs, enums, roots):
    seen, out = set(), set()
    stack = list(roots)
    while stack:
        n = stack.pop()
        if n in seen or n not in structs:
            continue
        seen.add(n)
        if is_container(n, structs, enums):
            out.add(n)
        for x in field_basetypes(structs[n][1]):
            if x in enums:
                stack.extend(enums[x])
            elif x in structs:
                stack.append(x)
    return out


def run_audit(root):
    structs, enums = build_chart_graph(chart_schema_file(root))
    containers = reachable_containers(structs, enums, CHART_AUDIT_ROOTS)
    data = load_manifest_full()
    paired = {p["sdk"] for p in data.get("pair", [])}
    deferred = {d["sdk"]: d.get("note", "") for d in data.get("deferred", [])}
    unaccounted = []
    for n in sorted(containers):
        if n in paired:
            print(f"pair {n}")
        elif n in deferred:
            print(f"ack  {n} -- {deferred[n]}")
        else:
            unaccounted.append(n)
            print(f"GAP  {n} (reachable chart container with no pair or deferral)")
    stale = sorted(s for s in deferred if s not in containers and s not in paired)
    promoted = sorted(s for s in deferred if s in paired)
    for s in stale:
        print(f"stale-deferral {s} (no longer a reachable chart container)")
    for s in promoted:
        print(f"stale-deferral {s} (now has a pair; drop from [[deferred]])")
    n_ack = sum(1 for n in containers if n in deferred and n not in paired)
    n_pair = sum(1 for n in containers if n in paired)
    print(
        f"\naudit: {len(containers)} reachable chart containers "
        f"-- {n_pair} paired, {n_ack} deferred, {len(unaccounted)} unaccounted"
    )
    if unaccounted or stale or promoted:
        print(
            f"{len(unaccounted)} unaccounted, {len(stale) + len(promoted)} stale deferrals",
            file=sys.stderr,
        )
        return 1
    return 0


def pair_decls(p):
    return (
        p.get("ns"),
        p.get("derived", []),
        p.get("excluded", []),
        p.get("aliases", {}),
    )


def print_table(res, sdk_struct, dto_struct):
    qn = f" [{res['qname']}]" if res["qname"] else ""
    print(f"# Coverage: {sdk_struct}{qn} ({res['filename']}) vs {dto_struct}")
    print()
    print("| ooxmlsdk field | xml tag | opt | choice | status |")
    print("| --- | --- | --- | --- | --- |")
    for f, status, _ in res["rows"]:
        tag = ", ".join(f["tags"])
        print(
            f"| {f['field']} | {tag} | {'Y' if f['optional'] else ''} | "
            f"{'Y' if f['choice'] else ''} | {status} |"
        )
    print()
    total = len(res["rows"])
    counts = {}
    for _, status, _ in res["rows"]:
        key = status.split(" ")[0]
        counts[key] = counts.get(key, 0) + 1
    missing = [f["field"] for f, _, gap in res["rows"] if gap]
    summary = ", ".join(f"{k} {v}" for k, v in sorted(counts.items()))
    print(f"{total} sdk fields: {summary}")
    print(f"gaps (MISSING, undeclared): {', '.join(missing) or 'none'}")
    sdk_norms = set()
    for f in res["sdk_fields"]:
        for k in keys_of(f):
            sdk_norms.add(norm(k))
    extra = [f for f in res["dto_fields"] if norm(f) not in sdk_norms]
    if extra:
        print(f"dto-only fields (sugar/derived/scoping): {', '.join(extra)}")


def run_check(root):
    pairs = load_manifest()
    failures = []
    for p in pairs:
        ns, derived, excluded, aliases = pair_decls(p)
        label = f"{p['sdk']} vs {p['dto']}"
        try:
            res = analyze(root, p["sdk"], p["dto"], ns, derived, excluded, aliases)
        except SystemExit as e:
            failures.append((label, [str(e)]))
            print(f"FAIL {label}: {e}")
            continue
        missing = [f["field"] for f, _, gap in res["rows"] if gap]
        if missing:
            failures.append((label, missing))
            print(f"FAIL {label}: {', '.join(missing)}")
        else:
            covered = sum(1 for _, s, _ in res["rows"] if s != "excluded")
            print(f"ok   {label} ({covered}/{len(res['rows'])} modeled)")
    if failures:
        print(f"\n{len(failures)} pair(s) with undeclared MISSING fields", file=sys.stderr)
        return 1
    print(f"\nall {len(pairs)} pairs clean")
    return 0


def main():
    ap = argparse.ArgumentParser(
        description="Coverage diff: ooxmlsdk CT_* struct fields vs an xlcore-types DTO."
    )
    ap.add_argument("sdk_struct", nargs="?", help="ooxmlsdk struct name, e.g. ValueAxis")
    ap.add_argument("dto_struct", nargs="?", help="xlcore-types struct, e.g. ChartAxisPatch")
    ap.add_argument("--ooxmlsdk-root", default=None)
    ap.add_argument("--ns", default=None, help="namespace prefix to disambiguate (c/x/a)")
    ap.add_argument("--check", action="store_true", help="check every manifest pair + audit")
    ap.add_argument(
        "--audit",
        action="store_true",
        help="audit: every chart container reachable from c:chart must be paired or deferred",
    )
    args = ap.parse_args()

    root = find_ooxmlsdk_root(args.ooxmlsdk_root)
    if args.audit:
        raise SystemExit(run_audit(root))
    if args.check:
        rc = run_check(root)
        print()
        rc |= run_audit(root)
        raise SystemExit(rc)
    if not args.sdk_struct or not args.dto_struct:
        ap.error("sdk_struct and dto_struct are required unless --check")

    ns, derived, excluded, aliases = args.ns, [], [], {}
    for p in load_manifest():
        if p["sdk"] == args.sdk_struct and p["dto"] == args.dto_struct:
            ns, derived, excluded, aliases = pair_decls(p)
            if args.ns:
                ns = args.ns
            break
    res = analyze(root, args.sdk_struct, args.dto_struct, ns, derived, excluded, aliases)
    print_table(res, args.sdk_struct, args.dto_struct)


if __name__ == "__main__":
    main()
