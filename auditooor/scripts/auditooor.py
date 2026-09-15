#!/usr/bin/env python3
"""auditooor helper — the deterministic work the LLM must not spend tokens on.

Pure standard library. Subcommands:

  scope    <root> [files...]          in-scope .sol files, line counts, function inventory -> JSON
  bundle   <root> <bundle_dir> <refs>  build source.md + one bundle per hunter from scope.json
  coverage <bundle_dir>               audit [Feynman: X] markers in hunter outputs vs the function
                                      inventory; lists entry points NO hunter actually read
  parse    <bundle_dir>               extract every FINDING / LEAD block from hunter outputs -> JSON
  stats    <bundle_dir>               marker counts per hunter (depth verification)

Nothing here decides a finding. It measures, bundles, and parses.
"""
import json
import os
import re
import sys

EXCLUDE_DIRS = {"interfaces", "interface", "lib", "libs", "mocks", "mock", "test", "tests",
                "node_modules", "out", "cache", "artifacts", "script", "scripts", "foundry-test",
                ".git", "broadcast", "typechain", "typechain-types", "certora", "echidna", "medusa"}
EXCLUDE_FILE = re.compile(r"(\.t\.sol$|\.s\.sol$|Test[^/]*\.sol$|Mock[^/]*\.sol$)")

FUNC_RE = re.compile(
    r"\bfunction\s+([A-Za-z_]\w*)\s*\(([^)]*)\)([^{;]*)[{;]", re.S)
CONTRACT_RE = re.compile(r"\b(abstract\s+contract|contract|library|interface)\s+([A-Za-z_]\w*)")
MARKER_RE = re.compile(r"\[(Feynman|Socratic|Inversion)\s*:\s*([^\]\n]+)\]")
BLOCK_HEAD = re.compile(r"^(FINDING|LEAD)\s*\|(.*)$")

# Hunters: file name -> prompt family. Core 12 are the Pashov Audit Group personas (MIT).
HUNTERS = [
    ("math-precision-agent", "specialty"),
    ("access-control-agent", "specialty"),
    ("economic-security-agent", "specialty"),
    ("execution-trace-agent", "specialty"),
    ("invariant-agent", "specialty"),
    ("periphery-agent", "specialty"),
    ("first-principles-agent", "specialty"),
    ("asymmetry-agent", "specialty"),
    ("boundary-agent", "specialty"),
    ("numerical-gap-agent", "gap"),
    ("trust-gap-agent", "gap"),
    ("flow-gap-agent", "gap"),
    ("spec-divergence-agent", "specialty"),
    ("lifecycle-agent", "specialty"),
]
QUICK = {"math-precision-agent", "access-control-agent", "economic-security-agent",
         "asymmetry-agent", "spec-divergence-agent", "lifecycle-agent", "flow-gap-agent",
         "invariant-agent"}


def strip_comments(src):
    """Blank comments and string literals, preserving offsets and newlines."""
    out = list(src)
    i, n = 0, len(src)
    while i < n:
        c = src[i]
        nxt = src[i + 1] if i + 1 < n else ""
        if c == "/" and nxt == "/":
            j = src.find("\n", i)
            j = n if j == -1 else j
            for k in range(i, j):
                out[k] = " "
            i = j
        elif c == "/" and nxt == "*":
            j = src.find("*/", i + 2)
            j = n if j == -1 else j + 2
            for k in range(i, j):
                if src[k] != "\n":
                    out[k] = " "
            i = j
        elif c in "\"'":
            j = i + 1
            while j < n and src[j] != c:
                j += 2 if src[j] == "\\" else 1
            for k in range(i + 1, min(j, n)):
                if src[k] != "\n":
                    out[k] = " "
            i = j + 1
        else:
            i += 1
    return "".join(out)


def discover(root):
    files = []
    for d, dirs, fs in os.walk(root):
        dirs[:] = sorted(x for x in dirs if x not in EXCLUDE_DIRS and not x.startswith("."))
        for f in sorted(fs):
            p = os.path.join(d, f)
            if f.endswith(".sol") and not EXCLUDE_FILE.search(p):
                files.append(p)
    return files


def inventory(path):
    raw = open(path, encoding="utf-8", errors="replace").read()
    src = strip_comments(raw)
    contracts = [(m.start(), m.group(1).split()[-1], m.group(2)) for m in CONTRACT_RE.finditer(src)]
    funcs = []
    for m in FUNC_RE.finditer(src):
        name, attrs = m.group(1), " ".join(m.group(3).split())
        owner, kind = None, None
        for start, k, cname in contracts:
            if start < m.start():
                owner, kind = cname, k
        if kind == "interface":
            continue
        vis = next((v for v in ("external", "public", "internal", "private") if re.search(rf"\b{v}\b", attrs)), "public")
        mut = "view" if re.search(r"\b(view|pure)\b", attrs) else "write"
        line = raw.count("\n", 0, m.start()) + 1
        mods = [x for x in re.findall(r"\b([A-Za-z_]\w*)\b", attrs)
                if x not in {"external", "public", "internal", "private", "view", "pure", "payable",
                             "virtual", "override", "returns", "memory", "calldata", "storage",
                             "uint256", "address", "bool", "bytes32", "bytes", "string", "int256"}
                and not x.startswith("uint") and not x.startswith("int") and not x.startswith("bytes")]
        funcs.append({"contract": owner, "name": name, "line": line, "visibility": vis,
                      "mutability": mut, "payable": "payable" in attrs.split(),
                      "modifiers": sorted(set(mods))})
    return {"lines": raw.count("\n") + 1, "contracts": [c[2] for c in contracts if c[1] != "interface"],
            "functions": funcs}


def cmd_scope(args):
    root = os.path.abspath(args[0])
    files = [os.path.abspath(f) for f in args[1:]] if len(args) > 1 else discover(root)
    out = {"root": root, "files": []}
    for f in files:
        inv = inventory(f)
        inv["path"] = os.path.relpath(f, root)
        out["files"].append(inv)
    ext = [fn for f in out["files"] for fn in f["functions"]
           if fn["visibility"] in ("external", "public") and fn["mutability"] == "write"]
    out["totals"] = {"files": len(files), "lines": sum(f["lines"] for f in out["files"]),
                     "entry_points_state_changing": len(ext)}
    print(json.dumps(out, indent=1))


def cmd_bundle(args):
    root, bdir, refs = (os.path.abspath(a) for a in args[:3])
    quick = "--quick" in args
    scope = json.load(open(os.path.join(bdir, "scope.json")))
    with open(os.path.join(bdir, "source.md"), "w") as out:
        for f in scope["files"]:
            body = open(os.path.join(root, f["path"]), encoding="utf-8", errors="replace").read()
            out.write(f"### {f['path']}\n\n```solidity\n{body}\n```\n\n")
    common_tail = ["shared-rules.md"]
    mapfile = os.path.join(bdir, "map.md")
    report = []
    for name, family in HUNTERS:
        if quick and name not in QUICK:
            continue
        parts = [os.path.join(bdir, "source.md")]
        if os.path.exists(mapfile):
            parts.append(mapfile)
        parts += [os.path.join(refs, "senior-auditor-sop.md"),
                  os.path.join(refs, "hunters", name + ".md")]
        parts += [os.path.join(refs, t) for t in common_tail]
        dst = os.path.join(bdir, f"{name}.bundle.md")
        with open(dst, "w") as out:
            for p in parts:
                out.write(open(p, encoding="utf-8").read().rstrip() + "\n\n---\n\n")
        n = sum(1 for _ in open(dst))
        report.append({"hunter": name, "family": family, "bundle": dst, "lines": n})
    print(json.dumps(report, indent=1))


def outputs(bdir):
    for f in sorted(os.listdir(bdir)):
        if f.endswith(".out.md"):
            yield f[: -len(".out.md")], open(os.path.join(bdir, f), encoding="utf-8", errors="replace").read()


def cmd_stats(args):
    bdir = args[0]
    rows = []
    for hunter, text in outputs(bdir):
        c = {"Feynman": 0, "Socratic": 0, "Inversion": 0}
        for m in MARKER_RE.finditer(text):
            c[m.group(1)] += 1
        f = len(re.findall(r"^FINDING\s*\|", text, re.M))
        l = len(re.findall(r"^LEAD\s*\|", text, re.M))
        rows.append({"hunter": hunter, **c, "findings": f, "leads": l,
                     "shallow": c["Feynman"] < 5 or c["Inversion"] == 0})
    print(json.dumps(rows, indent=1))


def cmd_coverage(args):
    bdir = args[0]
    scope = json.load(open(os.path.join(bdir, "scope.json")))
    seen = {}
    for hunter, text in outputs(bdir):
        for m in MARKER_RE.finditer(text):
            if m.group(1) != "Feynman":
                continue
            label = m.group(2)
            for c, f in re.findall(r"([A-Za-z_]\w*)\s*\.\s*([A-Za-z_]\w*)", label):
                seen.setdefault((c.lower(), f.lstrip("_").lower()), set()).add(hunter)
            for tok in re.findall(r"[A-Za-z_]\w*", label):
                seen.setdefault((None, tok.lstrip("_").lower()), set()).add(hunter)
    rows, uncovered = [], []
    for f in scope["files"]:
        for fn in f["functions"]:
            if fn["mutability"] != "write":
                continue
            name = fn["name"].lstrip("_").lower()
            exact = seen.get(((fn["contract"] or "").lower(), name), set())
            dotted_elsewhere = any(k[0] and k[1] == name for k in seen)
            loose = set() if dotted_elsewhere else seen.get((None, name), set())
            hunters = sorted(exact | loose)
            row = {"file": f["path"], "contract": fn["contract"], "function": fn["name"],
                   "line": fn["line"], "visibility": fn["visibility"], "hunters": len(hunters)}
            rows.append(row)
            if fn["visibility"] in ("external", "public") and len(hunters) < 2:
                uncovered.append(row)
    total = sum(1 for r in rows if r["visibility"] in ("external", "public"))
    print(json.dumps({"state_changing_entry_points": total,
                      "thin_or_uncovered": len(uncovered),
                      "threshold": "fewer than 2 hunters emitted [Feynman] for it",
                      "targets": uncovered}, indent=1))


def cmd_parse(args):
    bdir = args[0]
    items = []
    for hunter, text in outputs(bdir):
        lines = text.splitlines()
        i = 0
        while i < len(lines):
            m = BLOCK_HEAD.match(lines[i].strip().strip("`"))
            if not m:
                i += 1
                continue
            item = {"hunter": hunter, "kind": m.group(1)}
            head = m.group(2)
            gk = re.search(r"group_key\s*:\s*(.*)$", head)
            if gk:
                item["group_key"] = gk.group(1).strip()
                head = head[: gk.start()]
            for part in head.split("|"):
                if ":" in part:
                    k, v = part.split(":", 1)
                    item[k.strip()] = v.strip()
            j = i + 1
            key = None
            while j < len(lines) and not BLOCK_HEAD.match(lines[j].strip().strip("`")):
                ln = lines[j]
                if re.match(r"^(DONE|SKEPTIC DONE|MAP DONE)\s*\|", ln.strip()) or ln.strip().startswith("[Feynman") or ln.strip().startswith("[Inversion") or ln.strip().startswith("[Socratic") or ln.strip() in ("```", "---"):
                    break
                km = re.match(r"^([a-z_]{3,24}):\s?(.*)$", ln)
                if km:
                    key = km.group(1)
                    item[key] = km.group(2)
                elif key and ln.strip() and not ln.startswith("["):
                    item[key] += "\n" + ln
                elif not ln.strip() and key in (None,):
                    pass
                j += 1
            items.append(item)
            i = j
    by = {}
    for it in items:
        k = (it.get("contract", "?").lower(), it.get("function", "?").lower().split("(")[0])
        by.setdefault(k, []).append(it)
    print(json.dumps({"total": len(items),
                      "findings": sum(1 for x in items if x["kind"] == "FINDING"),
                      "leads": sum(1 for x in items if x["kind"] == "LEAD"),
                      "unique_contract_function": len(by),
                      "items": items}, indent=1))


def main():
    if len(sys.argv) < 3:
        print(__doc__)
        return 2
    cmd, args = sys.argv[1], sys.argv[2:]
    fn = {"scope": cmd_scope, "bundle": cmd_bundle, "coverage": cmd_coverage,
          "parse": cmd_parse, "stats": cmd_stats}.get(cmd)
    if not fn:
        print(__doc__)
        return 2
    return fn(args) or 0


if __name__ == "__main__":
    sys.exit(main())
