#!/usr/bin/env python3
"""Verify the Dockerfile.server cache-priming stage covers every workspace member.

DOCKER-09: `Dockerfile.server` manually copies each workspace member's
Cargo.toml into the builder stage to prime the dependency cache. If a member
is added to the root Cargo.toml but forgotten in the Dockerfile, the priming
build silently fails (it is best-effort) and the cache layer is dead weight.

This script parses the workspace `members` list from the root Cargo.toml and
asserts each member's manifest (or the inline dummy fallback for
apps/desktop-client and apps/tablet-client) is present in Dockerfile.server's
cache stage. Exit code 0 = consistent, 1 = drift.

Usage:
    python scripts/verify-dockerfile-workspace.py
"""

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CARGO_TOML = ROOT / "Cargo.toml"
DOCKERFILE = ROOT / "Dockerfile.server"

# These workspace members are NOT copied as manifests: the cache stage
# synthesizes inline dummy Cargo.tomls for them (they are excluded from the
# Docker build context by .dockerignore), so they are checked separately.
INLINE_DUMMY_MEMBERS = {"apps/desktop-client", "apps/tablet-client"}


def workspace_members() -> list[str]:
    text = CARGO_TOML.read_text(encoding="utf-8")
    m = re.search(r"\[workspace\]\s*(.*?)(?:\n\[|\Z)", text, re.S)
    if not m:
        sys.exit("error: could not locate [workspace] section in Cargo.toml")
    body = m.group(1)
    # Match `"crates/oz-core",` lines (trailing comma, CRLF-safe). Only the
    # members list itself — workspace.dependencies entries contain '='.
    members = [
        x for x in re.findall(r'^\s*"([^"]+)",?\s*$', body, re.M) if "=" not in x
    ]
    return sorted(set(members))


def dockerfile_text() -> str:
    return DOCKERFILE.read_text(encoding="utf-8")


def main() -> int:
    members = workspace_members()
    dockerfile = dockerfile_text()
    errors: list[str] = []

    for member in members:
        if member in INLINE_DUMMY_MEMBERS:
            # Inline dummy Cargo.toml is generated for these (printf ...)
            # because their real manifests are excluded from the build context.
            if f"> {member}/src/main.rs" not in dockerfile and f"> {member}/src/lib.rs" not in dockerfile:
                errors.append(
                    f"{member}: expected inline dummy src in Dockerfile.server cache stage"
                )
            continue
        if f"COPY {member}/Cargo.toml" not in dockerfile:
            errors.append(f"{member}: missing 'COPY {member}/Cargo.toml' in Dockerfile.server")
        if member.startswith("crates/") or member.startswith("modules/") or member.startswith("platform/"):
            src_dir = f"{member}/src"
            if src_dir not in dockerfile:
                errors.append(f"{member}: missing dummy src dir '{src_dir}' in cache stage")

    if errors:
        print("DOCKER-09 drift: Dockerfile.server cache-priming stage is out of sync with Cargo.toml workspace members:")
        for e in errors:
            print(f"  - {e}")
        print("Add the member's manifest COPY + dummy src dir to Dockerfile.server (see DOCKER-09).")
        return 1

    print(f"OK: all {len(members)} workspace members are represented in Dockerfile.server's cache stage.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
