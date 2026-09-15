#!/usr/bin/env python3
"""Detector precision/recall against a labeled real-contract corpus.
Usage: bench.py <binary> <vuln_dir> <clean_dir>"""
import subprocess, json, glob, os, sys
BIN, VULN, CLEAN = sys.argv[1], sys.argv[2], sys.argv[3]
def leads(f):
    try: return json.loads(subprocess.run([BIN,"detectors",f],capture_output=True,text=True).stdout)
    except Exception: return []
tp=fn=tn=fp=0
print(f"{'FILE':<30}{'LABEL':<7}{'IDs'}")
for f in sorted(glob.glob(f"{VULN}/*.sol")):
    d=leads(f); ok=len(d)>0; tp+=ok; fn+=not ok
    print(f"{os.path.basename(f):<30}{'VULN':<7}{'✓ '+','.join(sorted({x['id'] for x in d})) if ok else '✗ MISS'}")
for f in sorted(glob.glob(f"{CLEAN}/*.sol")):
    d=leads(f); clean=len(d)==0; tn+=clean; fp+=not clean
    print(f"{os.path.basename(f):<30}{'CLEAN':<7}{'✓ quiet' if clean else '✗ FP '+','.join(sorted({x['id'] for x in d}))}")
rec=tp/(tp+fn or 1); spec=tn/(tn+fp or 1); prec=tp/(tp+fp or 1)
print(f"\nRecall {tp}/{tp+fn}={rec:.0%} | Specificity {tn}/{tn+fp}={spec:.0%} | Precision {tp}/{tp+fp}={prec:.0%}")
sys.exit(0 if (fn==0 and fp==0) else 1)
