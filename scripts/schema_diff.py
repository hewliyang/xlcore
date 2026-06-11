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


SDK_STRUCT_RE = re.compile(r"pub struct (\w+) \{(.*?)\n\}", re.DOTALL)
SDK_FIELD_RE = re.compile(
    r"((?:#\[sdk\([^\n]*\)\]\s*)*)pub (\w+):\s*([^,]+),", re.DOTALL
)


def parse_sdk_struct(root, name):
    candidates = []
    for path in sorted(glob.glob(os.path.join(root, "src", "schemas", "*.rs"))):
        with open(path, encoding="utf-8") as fh:
            text = fh.read()
        m = re.search(
            r'(#\[sdk\(qname = "([^"]+)"\)\]\s*)?pub struct '
            + re.escape(name)
            + r" \{(.*?)\n\}",
            text,
            re.DOTALL,
        )
        if not m:
            continue
        body = m.group(3)
        fields = []
        for fm in SDK_FIELD_RE.finditer(body):
            attrs, fname, ftype = fm.group(1), fm.group(2), fm.group(3)
            optional = ftype.strip().startswith("Option")
            qnames = re.findall(r'qname = "([^"]+)"', attrs)
            is_choice = "choice(" in attrs
            tags = [leaf(q) for q in qnames] if qnames else []
            fields.append(
                {
                    "field": fname,
                    "tags": tags,
                    "optional": optional,
                    "choice": is_choice,
                }
            )
        candidates.append((os.path.basename(path), m.group(2), fields))
    if not candidates:
        raise SystemExit(f"sdk struct '{name}' not found under {root}/src/schemas")
    canonical = [c for c in candidates if c[0].startswith("schemas_openxmlformats_org")]
    chosen = (canonical or candidates)[0]
    if len(candidates) > 1:
        others = ", ".join(c[0] for c in candidates if c is not chosen)
        print(f"# note: '{name}' also defined in: {others}", file=sys.stderr)
    return chosen[0], chosen[1], chosen[2]


DTO_FIELD_RE = re.compile(r"\n    pub (\w+):", re.DOTALL)


def parse_dto_struct(name):
    path = os.path.join(
        os.path.dirname(__file__), "..", "crates", "xlcore-types", "src", "lib.rs"
    )
    with open(path, encoding="utf-8") as fh:
        text = fh.read()
    m = re.search(r"pub struct " + re.escape(name) + r" \{(.*?)\n\}", text, re.DOTALL)
    if not m:
        raise SystemExit(f"dto struct '{name}' not found in xlcore-types/src/lib.rs")
    return [fm.group(1) for fm in DTO_FIELD_RE.finditer("\n" + m.group(1))]


def norm(s):
    return re.sub(r"[^a-z0-9]", "", s.lower())


def covered(sdk_field, dto_norms):
    candidates = [sdk_field["field"]] + sdk_field["tags"]
    for c in candidates:
        if norm(c) in dto_norms:
            return True
    return False


EXCLUDE_DEFAULT = {"extlst", "sppr", "txpr"}


def main():
    ap = argparse.ArgumentParser(
        description="Coverage diff: ooxmlsdk CT_* struct fields vs an xlcore-types DTO."
    )
    ap.add_argument("sdk_struct", help="ooxmlsdk struct name, e.g. ValueAxis")
    ap.add_argument("dto_struct", nargs="?", help="xlcore-types struct name, e.g. ChartAxisPatch")
    ap.add_argument("--ooxmlsdk-root", default=None)
    args = ap.parse_args()

    root = find_ooxmlsdk_root(args.ooxmlsdk_root)
    src_file, sdk_qname, sdk_fields = parse_sdk_struct(root, args.sdk_struct)
    dto_fields = parse_dto_struct(args.dto_struct) if args.dto_struct else []
    dto_norms = {norm(f) for f in dto_fields}

    qn = f" [{sdk_qname}]" if sdk_qname else ""
    print(f"# Coverage: {args.sdk_struct}{qn} ({src_file}) vs "
          f"{args.dto_struct or '(none)'}")
    print()
    print("| ooxmlsdk field | xml tag | opt | choice | covered |")
    print("| --- | --- | --- | --- | --- |")
    n_cov = 0
    for f in sdk_fields:
        tag = ", ".join(f["tags"])
        if args.dto_struct:
            ok = covered(f, dto_norms)
        else:
            ok = None
        if ok:
            n_cov += 1
        mark = {True: "yes", False: "NO", None: "-"}[ok]
        print(
            f"| {f['field']} | {tag} | {'Y' if f['optional'] else ''} | "
            f"{'Y' if f['choice'] else ''} | {mark} |"
        )
    print()
    total = len(sdk_fields)
    if args.dto_struct:
        missing = [
            f["field"]
            for f in sdk_fields
            if not covered(f, dto_norms) and norm(f["field"]) not in EXCLUDE_DEFAULT
            and not any(norm(t) in EXCLUDE_DEFAULT for t in f["tags"])
        ]
        print(f"covered {n_cov}/{total}; "
              f"unmodeled (excluding extLst/spPr/txPr): {', '.join(missing) or 'none'}")
    else:
        print(f"{total} sdk fields; pass a DTO struct name for a coverage column.")
    if dto_fields:
        sdk_norms = set()
        for f in sdk_fields:
            sdk_norms.add(norm(f["field"]))
            for t in f["tags"]:
                sdk_norms.add(norm(t))
        extra = [f for f in dto_fields if norm(f) not in sdk_norms]
        if extra:
            print(f"dto-only fields (sugar/derived/scoping): {', '.join(extra)}")


if __name__ == "__main__":
    main()
