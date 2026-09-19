#!/usr/bin/env python3
"""Aggregate blind-judge scores.json files into recall / precision tables.

usage: score.py <scores.json> <A-tool-name> <B-tool-name> [more triples...]
The judge never sees tool names; the A/B assignment is revealed only here.
"""
import json
import sys


def tally(scores, side):
    t = {"high": [0, 0, 0], "medium": [0, 0, 0]}  # finding, lead, total
    for g in scores["gt"]:
        sev = "high" if g["severity"] in ("high", "critical") else "medium"
        t[sev][2] += 1
        tier = g[side]["tier"]
        if tier == "finding":
            t[sev][0] += 1
        elif tier == "lead":
            t[sev][1] += 1
    un = scores["unmatched"][side]
    cls = {c: sum(1 for u in un if u["class"] == c) for c in ("valid-unlisted", "invalid", "factually-wrong")}
    cls["factually-wrong"] += len(scores.get("factually_wrong_matched", {}).get(side, []))
    return t, cls


def main(argv):
    if len(argv) < 3 or len(argv) % 3:
        print(__doc__)
        return 2
    agg = {}
    for i in range(0, len(argv), 3):
        scores = json.load(open(argv[i]))
        for side, name in (("A", argv[i + 1]), ("B", argv[i + 2])):
            t, cls = tally(scores, side)
            a = agg.setdefault(name, {"high": [0, 0, 0], "medium": [0, 0, 0], "valid-unlisted": 0,
                                      "invalid": 0, "factually-wrong": 0})
            for sev in ("high", "medium"):
                a[sev] = [x + y for x, y in zip(a[sev], t[sev])]
            for c, n in cls.items():
                a[c] += n
            print(f"{argv[i]} {name}: H {t['high'][0]}+{t['high'][1]}L/{t['high'][2]}  "
                  f"M {t['medium'][0]}+{t['medium'][1]}L/{t['medium'][2]}  {cls}")
    print("\n| tool | High (finding) | High (+lead) | Medium (finding) | Medium (+lead) | invalid | factually wrong | valid-unlisted |")
    print("|---|---|---|---|---|---|---|---|")
    for name, a in agg.items():
        h, m = a["high"], a["medium"]
        pct = lambda n, d: f"{n}/{d} ({100 * n // max(d, 1)}%)"
        print(f"| {name} | {pct(h[0], h[2])} | {pct(h[0] + h[1], h[2])} | {pct(m[0], m[2])} | "
              f"{pct(m[0] + m[1], m[2])} | {a['invalid']} | {a['factually-wrong']} | {a['valid-unlisted']} |")


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
