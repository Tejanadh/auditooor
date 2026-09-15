#!/usr/bin/env python3
"""Stratified static-recall benchmark. Reports recall PER root-cause bucket with
abstention — never one aggregate number. Vuln files are named `bucket__name.sol`.
Usage: bench_strat.py <binary> <vuln_dir> <clean_dir>

HONEST SCOPE: this measures the deterministic FIRST-FILTER only (detectors).
It says NOTHING about pipeline recall — oracle/reentrancy/economic/cross-contract
logic bugs need the invariant harness + fork execution against archive state,
which this does not run. Do not read a good detector number as a pipeline number.
Baseline to beat: SunWeb3Sec's own SAST skill (34 vuln classes)."""
import subprocess, json, glob, os, collections, sys
BIN, VULN, CLEAN = sys.argv[1], sys.argv[2], sys.argv[3]
def leads(f):
    try: return json.loads(subprocess.run([BIN,"detectors",f],capture_output=True,text=True).stdout)
    except: return []
MODE = {
 "signature":"GRADED (SIG-*)", "accesscontrolinit":"GRADED (AC-*)",
 "accesscontrolother":"COVERAGE-GAP (tx.origin/visibility, not claimed)",
 "reentrancy":"ABSTAIN (harness/LLM)", "oracleprice":"ABSTAIN (harness/LLM)",
 "economic":"ABSTAIN (harness/LLM)", "integer":"ABSTAIN (harness/LLM)"}
files = collections.defaultdict(list)
for f in sorted(glob.glob(f"{VULN}/*.sol")):
    files[os.path.basename(f).split("__")[0]].append(f)
print("STRATIFIED STATIC-RECALL (first-filter only; NOT pipeline recall)")
for b in ["signature","accesscontrolinit","accesscontrolother","reentrancy","oracleprice","economic","integer"]:
    fs=files.get(b,[]); flagged=sum(1 for f in fs if leads(f)); m=MODE[b]
    if not fs: continue
    if m.startswith("GRADED"):
        print(f"  {b:<20}{m:<42}recall {flagged}/{len(fs)}={flagged/len(fs):.0%}")
    else:
        print(f"  {b:<20}{m:<42}flagged {flagged}/{len(fs)} (informational)")
fp=collections.Counter(); n=0
for f in sorted(glob.glob(f"{CLEAN}/*.sol")):
    n+=1
    for d in leads(f): fp[d["id"]]+=1
print(f"PER-DETECTOR FP on {n} audited OZ files: {dict(fp) if fp else 'none'}")
sys.exit(0 if not fp else 1)
