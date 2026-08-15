#!/usr/bin/env python3
r"""
scripts/verify-topology-parity.py — Keep the vendored topology semantic
contract byte-identical to the UI copy.

WHY
===

The topology semantic contract (`topologySemantics.json`) is consumed by
BOTH sides of the IPC boundary:

  1. Rust — `crates/oz-core/src/topology.rs` embeds a VENDORED copy
     (`crates/oz-core/src/topologySemantics.json`) via `include_str!` so
     compiling the cloud/desktop server never touches the UI tree
     (`.dockerignore` excludes `ui` entirely).
  2. TypeScript — `ui/src/features/stores/topologyContract.ts` and
     `topologyCard.ts` import the ORIGINAL `ui/src/features/stores/
     topologySemantics.json`.

If a developer edits the contract on one side and forgets the other, the
Rust and TS validation engines silently disagree about the same topology
graph — exactly the class of drift this script exists to catch before it
reaches production. The `oz-core` unit test
(`vendored_contract_matches_ui_canonical`) enforces the same invariant at
test time; this script makes it enforceable from CI and `scripts/check.sh`
without compiling anything.

USAGE
=====

    python3 scripts/verify-topology-parity.py     # exit 1 if the copies differ

EXIT CODES
==========

  0  — the two copies are byte-identical (or the UI copy is absent, which
       is legal in a server-only build context — oz-core vendors its own).
  1  — the copies drifted; the output names the files to reconcile.
"""

from __future__ import annotations

import sys
from pathlib import Path

VENDORED = Path("crates/oz-core/src/topologySemantics.json")
UI = Path("ui/src/features/stores/topologySemantics.json")


def main() -> int:
    if not VENDORED.exists():
        print(f"verify-topology-parity: missing vendored contract {VENDORED}")
        return 1
    if not UI.exists():
        print(
            "verify-topology-parity: ui/ absent (server-only build context) — "
            "vendored copy is authoritative here; nothing to compare."
        )
        return 0

    vendored = VENDORED.read_bytes()
    ui = UI.read_bytes()
    if vendored == ui:
        print(
            f"verify-topology-parity: OK — {VENDORED} and {UI} are "
            f"byte-identical ({len(vendored)} bytes)."
        )
        return 0

    # Locate the first differing line for an actionable message.
    vendored_lines = vendored.splitlines()
    ui_lines = ui.splitlines()
    first_diff = next(
        (
            i + 1
            for i, (a, b) in enumerate(zip(vendored_lines, ui_lines))
            if a != b
        ),
        min(len(vendored_lines), len(ui_lines)) + 1,
    )
    print(
        f"verify-topology-parity: DRIFT — {VENDORED} and {UI} differ "
        f"(first divergence around line {first_diff}).\n"
        "  Copy the edited copy across so the Rust include_str! and the TS "
        "import describe the same contract:\n"
        "    cp ui/src/features/stores/topologySemantics.json "
        "crates/oz-core/src/topologySemantics.json\n"
        "  (or the reverse, depending on which side owns the change)."
    )
    return 1


if __name__ == "__main__":
    sys.exit(main())
