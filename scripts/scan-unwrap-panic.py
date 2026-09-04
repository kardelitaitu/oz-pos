#!/usr/bin/env python3
"""Workspace-wide inventory of production (non-test) unwrap()/expect() calls.

Scans crates/, apps/, platform/, modules/ for *.rs and reports every
unwrap()/expect() that appears OUTSIDE of test contexts:

  * files under */tests/ (integration test dirs)
  * `#[cfg(test)]` blocks (attribute may precede `mod` or `fn`)
  * `mod tests` / `mod test` blocks
  * `#[test]`-annotated functions

Classification hint: a `# SAFETY:`/`// INVARIANT:` comment on the same line
or the line above marks a documented invariant panic. The script prints
`[INVARIANT]` when such a comment is found so reviewers can distinguish
intentional setup panics from recoverable runtime panics.

Exit code 0 = inventory generated; the output is the machine-readable list.
Exit code 1 = `--fail-on-recoverable` set and at least one finding lacks a
documented invariant comment (the recoverable set must stay at zero, ADR #33).
Usage:
    python scripts/scan-unwrap-panic.py                             # default roots
    python scripts/scan-unwrap-panic.py --json                      # JSON summary
    python scripts/scan-unwrap-panic.py --fail-on-recoverable       # exit 1 on untagged findings (CI gate)
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

ROOTS = ["crates", "apps", "platform", "modules"]

# Dev-only artifacts that never ship in production builds: benchmark
# harnesses (only compiled by `cargo bench`) and helper modules that are
# gated behind `#[cfg(test)]` in their parent `mod` declaration.
DEV_ONLY_PATHS = (
    "/benches/",  # cargo bench harnesses
    "test_helpers.rs",  # #[cfg(test)]-gated from parent mod
)

UNWRAP_RE = re.compile(r"\.unwrap\(\)")
EXPECT_RE = re.compile(r"\.expect\(")
CFG_TEST_RE = re.compile(r"#\[cfg\s*\(\s*test\s*\)")
# Matches `#[test]`, `#[tokio::test]`, and attribute variants such as
# `#[tokio::test(flavor = "multi_thread")]` — all of them mark test fns.
TEST_ATTR_RE = re.compile(r"#\[(?:tokio::)?test(?:\]|\()")
MOD_TESTS_RE = re.compile(r"^\s*mod\s+(tests?)\b")
INVARIANT_COMMENT_RE = re.compile(r"(INVARIANT|SAFETY|cannot fail|must not fail|impossible)")


def strip_comment(line: str, block_depth: int = 0) -> tuple[str, int]:
    """Strip string-safe code from one line, returning (code, block_depth_after).

    `block_depth` carries Rust block-comment state ACROSS lines, and starts at 0.
    This is the fix for the audit-stamp false positives: the per-crate header is

        1: /*
        2: last audited ...
        3: crate: oz-cli | status: SAFE | lint: CLEAN
        4: findings: ... 0 unsafe blocks ... 4 production .unwrap() in seed_demo.rs ...
        5: next: None ...
        6: */

    and the previous version handled `/*` by breaking out of the line, which is
    only correct for a single-line `/* ... */`. Lines 2-5 carry no `/*` or `//`
    marker at all, so they were returned verbatim and scanned as code -- meaning
    a file whose stamp says "0 unsafe blocks" and "clean" FAILED the cleanliness
    gate because of the sentence that says so. Six of the 18 findings were that.

    Depth is counted rather than toggled because Rust block comments NEST:
    `/* outer /* inner */ still comment */` is one comment, and a boolean would
    resume scanning code in the middle of it.
    """
    out: list[str] = []
    i = 0
    n = len(line)
    in_str = False
    while i < n:
        c = line[i]
        if block_depth > 0:
            # Inside a comment: only `*/` and a nested `/*` matter. A `//` here
            # is comment text, not a terminator, and a `"` is not a string.
            if c == "/" and i + 1 < n and line[i + 1] == "*":
                block_depth += 1
                i += 2
                continue
            if c == "*" and i + 1 < n and line[i + 1] == "/":
                block_depth -= 1
                i += 2
                continue
            i += 1
            continue
        if in_str:
            out.append(c)
            if c == '"' and (i == 0 or line[i - 1] != "\\"):
                in_str = False
            i += 1
            continue
        if c == '"':
            in_str = True
            out.append(c)
            i += 1
            continue
        if c == "/" and i + 1 < n and line[i + 1] == "/":
            # Line comment (also covers `//!` and `///`): rest of line is prose.
            break
        if c == "/" and i + 1 < n and line[i + 1] == "*":
            block_depth += 1
            i += 2
            continue
        if c == "*" and i + 1 < n and line[i + 1] == "/":
            # Stray `*/` outside a comment (e.g. the closing line of a block that
            # started on a previous line when depth was mis-tracked). Swallow it
            # rather than emitting `*` and `/` as code.
            i += 2
            continue
        out.append(c)
        i += 1
    return "".join(out), block_depth


def is_invariant_line(line: str) -> bool:
    """True if the line itself carries a documented-invariant comment."""
    return bool(INVARIANT_COMMENT_RE.search(line))


def invariant_documented(lines: list[str], idx: int) -> bool:
    """Does the code at `lines[idx]` carry a documented invariant?

    Accepted forms: a marker on the same line, or in the contiguous run of `//`
    comment lines directly above the STATEMENT the line belongs to.

    Two relaxations over the old "same line or the one line above" rule, both
    forced by real failures:

    1. A comment BLOCK, not just one line. The old rule made the keyword have to
       sit adjacent to the call, which is hostile to the multi-line explanation
       that is actually worth writing; six legitimate comments here counted as
       undocumented.
    2. The block may sit above the start of a multi-line chain, not just above
       the finding. Without this the gate FIGHTS `cargo fmt`: writing the chain
       on one line to keep the marker adjacent gets rewrapped by fmt on the next
       commit (the pre-commit hook runs `cargo fmt --all`), which moves
       `.unwrap()` away from its comment and silently re-breaks the gate. A rule
       the formatter can violate on its own is not a rule.

    Blank lines are NOT skipped -- a comment separated from code by whitespace
    may document something else entirely.
    """
    if is_invariant_line(lines[idx]):
        return True

    # Walk back to the start of the enclosing statement: the first preceding line
    # that is indented LESS than the finding. Chain continuations are always
    # deeper than the statement that opens them.
    def indent_of(s: str) -> int:
        return len(s) - len(s.lstrip())

    start = idx
    base = indent_of(lines[idx])
    j = idx - 1
    while j >= 0:
        s = lines[j].strip()
        if not s or s.startswith("//"):
            break  # comment block or blank line: the statement starts after it
        if indent_of(lines[j]) < base:
            start = j
            break
        j -= 1
    else:
        start = 0

    # Now walk back over the contiguous comment block above `start`.
    k = start - 1
    while k >= 0:
        s = lines[k].strip()
        if not s.startswith("//"):
            break
        if is_invariant_line(s):
            return True
        k -= 1
    return False


def scan_file(path: Path) -> list[dict]:
    """Return finding dicts for unwrap/expect outside test contexts."""
    findings: list[dict] = []
    try:
        lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
    except OSError:
        return findings

    # Stack of (kind, depth) skip contexts. kind in {"cfg_test", "mod_tests", "test_fn"}
    skip_stack: list[tuple[str, int]] = []
    pending_cfg_test = False
    pending_test_attr = False
    pending_mod_tests = False
    prev_line = ""
    # Block-comment nesting carried from line to line. See strip_comment.
    block_depth = 0

    for lineno, raw in enumerate(lines, start=1):
        code, block_depth = strip_comment(raw, block_depth)

        # ── open new skip contexts ──────────────────────────────────────
        if not skip_stack:
            if CFG_TEST_RE.search(code):
                pending_cfg_test = True
            if TEST_ATTR_RE.search(code):
                pending_test_attr = True
            if MOD_TESTS_RE.search(code):
                pending_mod_tests = True

        open_brace = code.count("{")
        close_brace = code.count("}")

        if skip_stack:
            kind, depth = skip_stack[-1]
            depth += open_brace - close_brace
            if depth <= 0:
                skip_stack.pop()
            else:
                skip_stack[-1] = (kind, depth)
        else:
            # Consume pending contexts when a brace opens on this line.
            if (pending_cfg_test or pending_test_attr or pending_mod_tests) and "{" in code:
                if pending_mod_tests:
                    skip_stack.append(("mod_tests", 1 + (open_brace - close_brace)))
                elif pending_cfg_test:
                    skip_stack.append(("cfg_test", 1 + (open_brace - close_brace)))
                else:
                    skip_stack.append(("test_fn", 1 + (open_brace - close_brace)))
                pending_cfg_test = False
                pending_test_attr = False
                pending_mod_tests = False

        # If a pending context attribute was seen but this line had no brace
        # (e.g. `#[cfg(test)]` then `mod tests {` on the NEXT line), the
        # MOD_TESTS_RE match on the next line will handle it via pending_mod_tests.
        if not skip_stack:
            # attribute + declaration on the same line, e.g. `#[cfg(test)] mod tests {`
            if pending_cfg_test and "mod" in code and "{" in code:
                skip_stack.append(("cfg_test", 1 + (open_brace - close_brace)))
                pending_cfg_test = False
                pending_test_attr = False
                pending_mod_tests = False
            elif pending_test_attr and "fn" in code and "{" in code:
                skip_stack.append(("test_fn", 1 + (open_brace - close_brace)))
                pending_test_attr = False

        if skip_stack:
            prev_line = raw
            continue

        # ── scan production lines ────────────────────────────────────────
        for m in UNWRAP_RE.finditer(code):
            findings.append(
                {
                    "path": str(path),
                    "line": lineno,
                    "call": "unwrap",
                    "text": raw.strip(),
                    "invariant": invariant_documented(lines, lineno - 1),
                }
            )
        for m in EXPECT_RE.finditer(code):
            findings.append(
                {
                    "path": str(path),
                    "line": lineno,
                    "call": "expect",
                    "text": raw.strip(),
                    "invariant": invariant_documented(lines, lineno - 1),
                }
            )
        prev_line = raw

    return findings


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--json", action="store_true", help="emit JSON summary")
    parser.add_argument(
        "--fail-on-recoverable",
        action="store_true",
        help="exit 1 when any production unwrap/expect lacks a documented "
        "invariant comment (the recoverable set must stay at zero, ADR #33)",
    )
    parser.add_argument("--roots", nargs="*", default=ROOTS, help="roots to scan")
    args = parser.parse_args()

    all_findings: list[dict] = []
    for root in args.roots:
        root_path = Path(root)
        if not root_path.is_dir():
            continue
        for path in sorted(root_path.rglob("*.rs")):
            if "tests" in path.parts:
                continue
            if any(tok in str(path).replace("\\", "/") for tok in DEV_ONLY_PATHS):
                continue
            # Split test modules (`#[cfg(test)] mod foo_tests;` in the parent)
            # carry the `*_tests.rs` / `*_test.rs` filename convention; the
            # parent's cfg-gate is invisible to this per-file scan, so treat
            # those filenames as test code (ADR #33: test code is exempt).
            if re.search(r"_test(s)?\.rs$", path.name):
                continue
            all_findings.extend(scan_file(path))

    recoverable = [f for f in all_findings if not f["invariant"]]
    if args.fail_on_recoverable and recoverable:
        print(
            f"panic-inventory FAIL: {len(recoverable)} recoverable unwrap/expect "
            "call(s) lack a documented invariant comment (ADR #33):",
            file=sys.stderr,
        )
        for f in recoverable:
            print(
                f'{f["path"]}:{f["line"]}: {f["call"]}()  {f["text"]}',
                file=sys.stderr,
            )
        print(
            "Fix: add a // SAFETY: / // INVARIANT: comment on the same or "
            "immediately preceding line, or convert the call to a Result path.",
            file=sys.stderr,
        )
        return 1

    if args.json:
        by_file: dict[str, int] = {}
        invariant = 0
        for f in all_findings:
            by_file[f["path"]] = by_file.get(f["path"], 0) + 1
            if f["invariant"]:
                invariant += 1
        print(
            json.dumps(
                {
                    "total": len(all_findings),
                    "invariant_annotated": invariant,
                    "recoverable": len(recoverable),
                    "files": len(by_file),
                    "by_file": dict(sorted(by_file.items(), key=lambda kv: -kv[1])),
                },
                indent=2,
            )
        )
        return 0

    for f in all_findings:
        tag = " [INVARIANT]" if f["invariant"] else ""
        print(f'{f["path"]}:{f["line"]}: {f["call"]}()  {f["text"]}{tag}')
    if args.fail_on_recoverable:
        # Concise success line for the CI / check.sh gate — plain mode would
        # otherwise dump the whole inventory into the build log.
        print(
            f"{len(all_findings)} production unwrap/expect calls, all documented "
            "invariants",
            file=sys.stderr,
        )
    else:
        print(f"\n# total: {len(all_findings)} production unwrap/expect calls", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
