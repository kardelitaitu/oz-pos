#!/usr/bin/env python3
"""Guard against float-typed columns for exact-decimal values.

Why this exists: REAL/DOUBLE PRECISION silently corrupt values that need
exact decimal semantics. The loyalty tier multiplier 1.4 was stored as
1.3999999999999999111, so a $22.50 sale at a 1.4x tier earned 31 points
where the owner's intent was 32 (LOYALTY-01, closed 2026-08-31 by the
earn_multiplier_millionths migration); the tender exchange rate had the
same class of bug at the compute site (MONEY-01). The repo convention is
fixed-point integers — `*_minor` (i64 cents) and `*_millionths` (i64
scaled decimals). This lint stops a NEW float column from entering the
schema without a conscious, written justification.

Scope: every .sql under crates/oz-core/migrations/ (SQLite side AND the
generated init.pg.sql — the PG file is where hand-ported drift historically
introduced DOUBLE PRECISION twins of flagged columns).

Rules:
  * A column definition whose type is REAL, FLOAT, DOUBLE, or DOUBLE
    PRECISION is a violation unless whitelisted.
  * Whitelist entries are content-anchored (file + table + column) and
    carry a justification. A whitelist entry that matches nothing is
    STALE and fails the check — the same discipline as the ERR-10
    error-policy whitelist: exemptions must die when the code dies.

Usage:
    python3 scripts/verify-migration-column-types.py                # full scan
    python3 scripts/verify-migration-column-types.py --staged-only  # only staged migration files
"""

from __future__ import annotations

import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

# Repo-relative, never anchored to a hardcoded checkout (AGENTS.md).
ROOT = Path(__file__).resolve().parent.parent
MIGRATIONS = ROOT / "crates" / "oz-core" / "migrations"

# Column-name + float-type pair. The column name is the word before the
# type keyword; matches inside comments are impossible (comments are
# stripped first). Constraint-leading keywords can never be a column name
# because they are excluded below.
FLOAT_TYPE = r"(?:DOUBLE\s+PRECISION|DOUBLE|REAL|FLOAT)"
COL_RE = re.compile(rf"\b\"?([A-Za-z_]\w*)\"?\s+{FLOAT_TYPE}\b")
ALTER_RE = re.compile(
    rf"ALTER\s+TABLE\s+\"?([A-Za-z_]\w*)\"?\s+ADD\s+(?:COLUMN\s+)?\"?([A-Za-z_]\w*)\"?\s+{FLOAT_TYPE}\b",
    re.IGNORECASE,
)
# The trailing \s*\( is load-bearing: without it, `CREATE TABLE IF NOT
# EXISTS "products"` backtracks the optional IF-group (quoted name fails
# the bare capture) and reports the table as "IF".
CREATE_RE = re.compile(
    r"CREATE\s+TABLE\s+(?:IF\s+NOT\s+EXISTS\s+)?\"?([A-Za-z_]\w*)\"?\s*\(", re.IGNORECASE
)
# Words that appear before a type but are not column names.
NOT_A_COLUMN = {"PRIMARY", "UNIQUE", "CHECK", "FOREIGN", "REFERENCES", "CONSTRAINT"}


@dataclass(frozen=True)
class Hit:
    file: str
    table: str
    column: str
    line: int


@dataclass(frozen=True)
class Allowed:
    file: str
    table: str
    column: str
    why: str


# Every exemption, with the reason it is not an exact-decimal value.
WHITELIST: tuple[Allowed, ...] = (
    Allowed(
        "20260813_init.sql", "loyalty_tiers", "earn_multiplier",
        "historical column: fresh-install replay target, converted to "
        "earn_multiplier_millionths by 20260831_loyalty_multiplier_fixedpoint.sql",
    ),
    Allowed(
        "20260813_init.sql", "products", "popularity_score",
        "analytics score recomputed from sales history; display-ranked, never money",
    ),
    Allowed(
        "20260813_init.pg.sql", "products", "popularity_score",
        "PG twin of the analytics score above",
    ),
    Allowed(
        "20260831_per_tenant_unique_rebuild.sql", "products_new", "popularity_score",
        "faithful copy of products.popularity_score during the uniqueness rebuild",
    ),
    # Floor-plan canvas geometry on the restaurant `tables` table —
    # display-only coordinates, never money.
    Allowed("20260813_init.sql", "tables", "pos_x", "floor-plan canvas geometry, display-only"),
    Allowed("20260813_init.sql", "tables", "pos_y", "floor-plan canvas geometry, display-only"),
    Allowed("20260813_init.sql", "tables", "width", "floor-plan canvas geometry, display-only"),
    Allowed("20260813_init.sql", "tables", "height", "floor-plan canvas geometry, display-only"),
    Allowed("20260813_init.pg.sql", "tables", "pos_x", "PG twin: floor-plan geometry"),
    Allowed("20260813_init.pg.sql", "tables", "pos_y", "PG twin: floor-plan geometry"),
    Allowed("20260813_init.pg.sql", "tables", "width", "PG twin: floor-plan geometry"),
    Allowed("20260813_init.pg.sql", "tables", "height", "PG twin: floor-plan geometry"),
)


def strip_comments(sql: str) -> str:
    """Blank out -- line comments and /* */ blocks, preserving line numbers."""
    sql = re.sub(r"/\*.*?\*/", lambda m: re.sub(r"[^\n]", " ", m.group(0)), sql, flags=re.S)
    return re.sub(r"--[^\n]*", "", sql)


def scan_file(path: Path) -> list[Hit]:
    text = strip_comments(path.read_text(encoding="utf-8"))
    hits: list[Hit] = []
    table = "?"
    for lineno, line in enumerate(text.splitlines(), start=1):
        m = CREATE_RE.search(line)
        if m:
            table = m.group(1)
        for am in ALTER_RE.finditer(line):
            hits.append(Hit(path.name, am.group(1), am.group(2), lineno))
        for cm in COL_RE.finditer(line):
            col = cm.group(1)
            if col.upper() in NOT_A_COLUMN:
                continue
            # ALTER ADD lines are already captured precisely by ALTER_RE;
            # COL_RE would double-report them with the wrong table context.
            if re.search(rf"ADD\s+(?:COLUMN\s+)?\"?{re.escape(col)}\"?\s+{FLOAT_TYPE}", line, re.IGNORECASE):
                continue
            hits.append(Hit(path.name, table, col, lineno))
    return hits


def main(argv: list[str]) -> int:
    staged_only = "--staged-only" in argv
    files = sorted(MIGRATIONS.glob("*.sql"))
    if staged_only:
        staged = set(
            subprocess.run(
                ["git", "diff", "--cached", "--name-only", "--diff-filter=ACM", "-z", "--",
                 "crates/oz-core/migrations/"],
                cwd=ROOT, capture_output=True, text=True, check=True,
            ).stdout.split("\0")
        )
        files = [f for f in files if str(f.relative_to(ROOT)).replace("\\", "/") in staged]
    if not files:
        return 0

    hits: list[Hit] = []
    for f in files:
        hits.extend(scan_file(f))

    allowed = {(a.file, a.table, a.column) for a in WHITELIST}
    violations = [h for h in hits if (h.file, h.table, h.column) not in allowed]
    # Stale-entry detection only makes sense on a FULL scan: a staged-only
    # pass sees a subset of files, so exemptions for unscanned columns
    # would false-flag (the ERR-10 scoping lesson).
    seen = {(h.file, h.table, h.column) for h in hits}
    stale = [] if staged_only else [a for a in WHITELIST if (a.file, a.table, a.column) not in seen]

    rc = 0
    for v in violations:
        print(
            f"error: {v.file}:{v.line} — column {v.table}.{v.column} is float-typed. "
            "Exact-decimal values (money, rates, multipliers) must be fixed-point "
            "integers (*_minor, *_millionths) per LOYALTY-01/MONEY-01. If this "
            "column genuinely needs a float (geometry, analytics), add a "
            "justified WHITELIST entry to this script.",
            file=sys.stderr,
        )
        rc = 1
    for a in stale:
        print(
            f"error: stale whitelist entry {a.file} {a.table}.{a.column} "
            f"({a.why}) — the column no longer exists; remove the exemption.",
            file=sys.stderr,
        )
        rc = 1
    if rc == 0:
        scope = "staged" if staged_only else f"{len(files)} files"
        print(f"ok: no unwhitelisted float columns ({scope} scanned, {len(hits)} float hits all exempt)")
    return rc


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
