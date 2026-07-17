#!/usr/bin/env python3
"""Per-file coverage for feature modules from cargo-llvm-cov JSON.

Usage:
    python3 scripts/coverage_top.py [path/to/coverage.json]

If no path is given, scans coverage/rust/ for .json files and uses
whichever is newest (typically coverage.json from cargo-llvm-cov).
"""
import json
import os
import sys
from pathlib import Path

# cargo-llvm-cov --output-path produces a JSON whose top-level is
# {"version": ..., "type": ..., "cargo_llvm_cov": ..., "data": [...]}.
# The 'data' array typically contains one entry scoped to the run; its
# 'files' array holds per-source-file coverage data.

DEFAULT_DIR = Path(__file__).resolve().parent.parent / "coverage" / "rust"

if len(sys.argv) > 1:
    path = sys.argv[1]
else:
    if DEFAULT_DIR.is_dir():
        candidates = sorted(DEFAULT_DIR.glob("*.json"), key=os.path.getmtime)
        if candidates:
            path = str(candidates[-1])
        else:
            print(f"MISSING: no .json files in {DEFAULT_DIR}")
            sys.exit(0)
    else:
        print(f"MISSING: directory not found: {DEFAULT_DIR}")
        sys.exit(0)

if not os.path.exists(path):
    print(f"MISSING: {path}")
    sys.exit(0)

with open(path) as f:
    top = json.load(f)

cwd = os.getcwd().replace("\\", "/")
keys = ("gift_card", "stock_count", "stock_transfer", "supplier", "purchase_order")

rows = []
for entry in top.get("data", []):
    for f in entry.get("files", []):
        # prefer 'filename'; fall back to 'name' in case of older layouts
        fn = f.get("filename") or f.get("name") or ""
        if not any(k in fn for k in keys):
            continue
        s = f.get("summary") or {}
        # cargo-llvm-cov summary uses 'line' / 'lines', 'region', 'function'
        # depending on version. Try both spellings.
        lines = s.get("line", s.get("lines", {}))
        funcs = s.get("function", s.get("functions", {}))
        lc = lines.get("covered", 0)
        lm = lines.get("missed", 0)
        ln = lc + lm
        pct = (100.0 * lc / ln) if ln else 0.0
        fc = funcs.get("covered", 0)
        fm = funcs.get("missed", 0)
        fn_total = fc + fm
        fpct = (100.0 * fc / fn_total) if fn_total else 0.0
        short = fn.replace(cwd, "").lstrip("/").lstrip("\\")
        rows.append((pct, short, lc, lm, fpct, fc, fm))

if not rows:
    print(f"NO MATCHES: scanned {sum(len(e.get('files', [])) for e in top.get('data', []))} files; none matched {keys}")
    sys.exit(0)

rows.sort(key=lambda r: r[0])
print("Per-file coverage for new feature modules")
print("=" * 90)
print(f"  {'LINES':>10}  {'FUNCS':>10}  PATH")
for pct, fn, c, m, fpct, fc, fm in rows:
    print(f"  L {pct:6.2f}% {c:4d}/{c + m:4d}  F {fpct:6.2f}% {fc:3d}/{fc + fm:3d}  {fn}")
