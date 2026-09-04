"""Prove the R36-01 fix by running the anchor test under several host timezones.

Node resolves the process zone from TZ at startup, so a single vitest run can
never demonstrate host-independence -- the assertion that looks green on a UTC
runner is exactly the one that was red on a UTC+7 workstation. This runs the
same file under each zone and requires identical results.

Before the fix this script fails: isoToday(null) used the device calendar, so
TZ=Asia/Jakarta and TZ=Pacific/Kiritimati disagree with TZ=UTC near day
boundaries. That is the regression being pinned.
"""
from __future__ import annotations

import io
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
UI = ROOT / "ui"
# R36-01's pure anchor test, plus R36-05's component-level one: the dashboard
# renders its default range into <input type=date>, so asserting on the DOM is
# the only way to prove the whole plumbing (fetch -> state -> re-seed -> render)
# is host-independent, not just the helper in isolation.
TESTS = [
    "src/__tests__/analyticsTimezoneAnchor.test.ts",
    "src/__tests__/DashboardScreen.test.tsx",
]

# Chosen to straddle the date line and both sides of UTC, and to include the
# zone that broke PR #95. Kiritimati is UTC+14 -- the largest offset that can
# put the local calendar a full day ahead of UTC.
#
# DO NOT TRIM THIS LIST WITHOUT REDOING THIS ARITHMETIC. A given zone only
# disagrees with UTC during part of the day: local date differs from the UTC
# date when UTC-time-of-day is below |offset| for west zones, and at or above
# 24-|offset| for east zones. So:
#   Los_Angeles (UTC-7/-8) differs while UTC < 17:00 (16:00 in winter)
#   Kiritimati  (UTC+14)   differs once UTC >= 10:00
# Those two windows overlap and together cover all 24 hours, so at least one
# zone always disagrees -- the check is sensitive no matter when it runs.
# Jakarta is kept because it is the zone that actually broke PR #95, not
# because it adds coverage. Dropping both LA and Kiritimati would leave the
# check passing at some hours and failing at others, which is the worst
# possible property for a regression gate.
ZONES = ["UTC", "Asia/Jakarta", "Pacific/Kiritimati", "America/Los_Angeles"]

NPMX = "npm.cmd" if os.name == "nt" else "npm"

results: dict[str, str] = {}
failed = False

print(f"=== timezone invariance ({len(TESTS)} file(s) x {len(ZONES)} zones) ===")
for tz in ZONES:
    env = {**os.environ, "TZ": tz}
    r = subprocess.run(
        [NPMX, "exec", "--silent", "--", "vitest", "run", *TESTS, "--reporter=json"],
        cwd=UI, env=env, capture_output=True, text=True,
    )
    # vitest's json reporter prints its own banner lines; find the JSON object.
    out = r.stdout
    start = out.find("{")
    end = out.rfind("}")
    summary = "PARSE-ERROR"
    if start != -1 and end > start:
        import json
        try:
            data = json.loads(out[start:end + 1])
            passed = data.get("numPassedTests", 0)
            broken = data.get("numFailedTests", 0)
            summary = f"{passed} passed, {broken} failed"
            if broken:
                failed = True
                for res in data.get("testResults", []):
                    for a in res.get("assertionResults", []):
                        if a.get("status") == "failed":
                            summary += " | FAIL: " + a.get("title", "?")[:60]
        except Exception as exc:  # noqa: BLE001
            summary = f"PARSE-ERROR ({exc})"
    else:
        failed = True
        summary = f"NO JSON (exit {r.returncode}) :: " + (r.stderr or out)[-200:]

    results[tz] = summary
    print(f"  TZ={tz:22s} {summary}")

print()
distinct = set(results.values())
if len(distinct) == 1 and not failed:
    print(f"PASS: identical result under {len(ZONES)} host zones -> {distinct.pop()}")
    sys.exit(0)

print("FAIL: the anchored range depends on the host timezone.")
for tz, s in results.items():
    print(f"  {tz:22s} {s}")
sys.exit(1)
