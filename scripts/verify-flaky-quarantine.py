#!/usr/bin/env python3
"""
scripts/verify-flaky-quarantine.py — AUDIT-27 CI-09 enforcement gate.

Validates scripts/flaky-quarantine.json:
  • schema sanity (version + entries)
  • every entry has test, owner, issue, reason, date, expiry
  • every entry references a GitHub issue (URL or #NN)
  • no entry is expired (expiry >= today)

Usage:
  python3 scripts/verify-flaky-quarantine.py            # fail on any violation
  python3 scripts/verify-flaky-quarantine.py --report   # print count, still gate

Exit code is 1 on any violation, 0 otherwise.
"""

import datetime
import json
import re
import sys
from pathlib import Path

MANIFEST = Path(__file__).resolve().parent / "flaky-quarantine.json"

REQUIRED_FIELDS = ("test", "owner", "issue", "reason", "date", "expiry")
ISSUE_RE = re.compile(r"^(https?://\S+|#\d+)$")
DATE_RE = re.compile(r"^\d{4}-\d{2}-\d{2}$")


def main() -> int:
    report_only = "--report" in sys.argv[1:]

    if not MANIFEST.exists():
        print(f"FAIL: manifest not found at {MANIFEST}")
        return 1

    try:
        data = json.loads(MANIFEST.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        print(f"FAIL: manifest is not valid JSON: {exc}")
        return 1

    if not isinstance(data, dict) or data.get("version") != 1:
        print("FAIL: manifest must be an object with version = 1")
        return 1

    entries = data.get("entries", [])
    if not isinstance(entries, list):
        print("FAIL: manifest 'entries' must be a list")
        return 1

    today = datetime.date.today()
    violations = []
    expired = 0

    for i, entry in enumerate(entries):
        if not isinstance(entry, dict):
            violations.append(f"entry[{i}] is not an object")
            continue

        label = entry.get("test") or f"entry[{i}]"

        for field in REQUIRED_FIELDS:
            value = entry.get(field)
            if not isinstance(value, str) or not value.strip():
                violations.append(f"{label}: missing required field '{field}'")

        issue = entry.get("issue")
        if issue and not ISSUE_RE.match(issue.strip()):
            violations.append(f"{label}: 'issue' must be a URL or #NN (got '{issue}')")

        for field in ("date", "expiry"):
            value = entry.get(field)
            if value and not DATE_RE.match(value):
                violations.append(f"{label}: '{field}' must be YYYY-MM-DD (got '{value}')")

        expiry = entry.get("expiry")
        if expiry and DATE_RE.match(expiry):
            exp_date = datetime.date.fromisoformat(expiry)
            if exp_date < today:
                expired += 1
                violations.append(
                    f"{label}: EXPIRED on {expiry} — re-investigate the flake, "
                    f"fix it, or renew the quarantine with an updated issue"
                )

    if report_only:
        print(f"INFO: {len(entries)} quarantined test(s), {expired} expired, "
              f"{len(violations)} violation(s)")

    if violations:
        print("FAIL: flaky-quarantine.json violates the CI-09 policy:")
        for v in violations:
            print(f"  - {v}")
        print("See docs/ci-pipeline.md and CONTRIBUTING.md for the quarantine lifecycle.")
        return 1

    print(f"PASS: quarantine manifest valid ({len(entries)} entries, none expired)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
