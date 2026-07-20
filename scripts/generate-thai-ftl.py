#!/usr/bin/env python3
"""Generate Thai .th.ftl scaffolding from English .ftl bundles.

For each .ftl file in ui/src/locales/ (that doesn't already have .id or .th suffix),
copies its content to a .th.ftl sibling file with [TH] prefix markers on values.

This produces ready-to-translate Thai bundles. A professional translator only
needs to replace the English text between `[TH] … [/TH]` with proper Thai.

Usage:
    python scripts/generate-thai-ftl.py
"""

import os
import re
from pathlib import Path

LOCALES_DIR = Path(__file__).resolve().parent.parent / "ui" / "src" / "locales"

def wrap_thai_value(match: re.Match) -> str:
    """Wrap the value part of an FTL key-value pair with [TH] markers."""
    eq = match.group("eq")
    value = match.group("value")
    # Skip if already wrapped
    if value.strip().startswith("[TH]") and value.strip().endswith("[/TH]"):
        return match.group(0)
    return f"{match.group('key')}{eq}[TH] {value} [/TH]"

def main():
    en_files = sorted(LOCALES_DIR.glob("*.ftl"))
    en_files = [f for f in en_files if ".id." not in f.name and ".th." not in f.name]

    created = 0
    skipped = 0

    for en_file in en_files:
        # Derive name: sales.ftl -> sales.th.ftl
        stem = en_file.stem  # e.g. "sales"
        th_path = en_file.parent / f"{stem}.th.ftl"

        if th_path.exists():
            print(f"  SKIP: {th_path.name} (already exists)")
            skipped += 1
            continue

        content = en_file.read_text(encoding="utf-8")

        # Preserve comments (# lines) and blank lines as-is
        # Wrap value strings with [TH] ... [/TH]
        lines = content.split("\n")
        out_lines = []
        for line in lines:
            # Comments and blank lines: pass through unchanged
            if line.startswith("#") or line.strip() == "":
                out_lines.append(line)
                continue

            # FTL key = value lines: wrap value
            match = re.match(r'^(?P<key>[a-zA-Z0-9_-]+)\s*=\s*(?P<value>.+)$', line)
            if match:
                key = match.group("key")
                value = match.group("value")
                if value.strip().startswith("[TH]") and value.strip().endswith("[/TH]"):
                    out_lines.append(line)
                else:
                    out_lines.append(f"{key} = [TH] {value} [/TH]")
            else:
                # Multi-line attribute continuation (indented .attr = value)
                attr_match = re.match(r'^(\s+\.\S+\s*=\s*)(.+)$', line)
                if attr_match:
                    out_lines.append(f"{attr_match.group(1)}[TH] {attr_match.group(2)} [/TH]")
                else:
                    out_lines.append(line)

        th_path.write_text("\n".join(out_lines) + "\n", encoding="utf-8")
        print(f"  CREATE: {th_path.name}")
        created += 1

    print(f"\nDone: {created} created, {skipped} skipped, {len(en_files)} total")

if __name__ == "__main__":
    main()
