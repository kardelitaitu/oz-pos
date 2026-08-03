#!/usr/bin/env python3
r"""
scripts/verify-ci-docs-drift.py — Catch CI documentation drift between
docs/ci-pipeline.md, the workflow definitions, and the local runners.

WHY
===

`docs/ci-pipeline.md` is the canonical CI dashboard (AUDIT-27 CI-08).
When a job is renamed, removed, or moved between workflows, the docs
tables go stale and contributors trust a matrix that no longer matches
what CI actually runs. Likewise, `scripts/check.sh` (repository gate)
and `scripts/check-ui.mjs` (the `check:all` gate) are documented as
sharing a common gate vocabulary — if one drifts, "all checks passed"
means different things per entry point.

Since AUDIT-27 CI-08 the gate vocabulary + status live in a SINGLE
source of truth: `scripts/gates.json`. This script derives everything
from that manifest and verifies, fail-closed:

  1. **Jobs:** every job name referenced in `docs/ci-pipeline.md` (the
     Job Matrix and Pre-Merge Validation Gates tables) exists as a real
     job in `.github/workflows/*.yml`.
  2. **Workflow inventory:** every workflow file named in the Workflow
     inventory table exists on disk.
  3. **Gate vocabulary (manifest-driven):** every gate in
     `scripts/gates.json` is declared by the runners it lists
     (`check.sh` / `check:all` needles matched against the extracted
     labels). A manifest gate that no runner declares is drift.
  4. **Gate CI mapping + status (manifest-driven):** every gate's
     `ci.workflow`/`ci.job` reference exists, and the workflow honours
     the gate's declared status:
       * `required`            → the job must NOT have
                                 `continue-on-error: true`
       * `advisory`            → the job (or a step when
                                 `advisory_at: "step"`) MUST have
                                 `continue-on-error: true`
       * `required-on-push`    → the job's `continue-on-error` must be a
                                 conditional `${{ ... }}` expression
                                 (advisory on PR, required on push)

Workflow jobs that exist but are not documented, and runner labels
that no manifest gate covers, are reported as informational notes —
the docs/manifest are never stale in the fail direction.

USAGE
=====

    python3 scripts/verify-ci-docs-drift.py           # strict: exit 1 on drift
    python3 scripts/verify-ci-docs-drift.py --report-only   # always exit 0
    python3 scripts/verify-ci-docs-drift.py --verbose       # show OK rows

EXIT CODES
==========

  * 0  no drift: every documented job/workflow exists, manifest gates
        are declared and their status matches the workflows.
  * 1  a documented job is missing, a workflow file is missing, a
        manifest gate is undeclared, or a gate status contradicts its
        workflow.
  * 2  a runtime error occurred (docs, workflows dir, or gates.json
        missing/invalid).
"""

import argparse
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DOCS = ROOT / "docs" / "ci-pipeline.md"
WORKFLOWS_DIR = ROOT / ".github" / "workflows"
GATES_MANIFEST = ROOT / "scripts" / "gates.json"
CHECK_SH = ROOT / "scripts" / "check.sh"
CHECK_UI = ROOT / "scripts" / "check-ui.mjs"

VALID_STATUS = {"required", "advisory", "required-on-push"}

# ── Docs parsing ──────────────────────────────────────────────────────
# Backticked kebab-case token (job IDs / workflow filenames).
KUBE_TOKEN = re.compile(r"`([a-z][a-z0-9-]*)`")
# Job Matrix table row: first cell is a backticked job ID.
JOB_MATRIX_ROW = re.compile(r"^\|\s*`([a-z][a-z0-9-]*)`\s*\|")
# Gate table row: capture the Job cell (second cell) for token extraction.
GATE_TABLE_ROW = re.compile(r"^\|\s*[^|]+\|\s*(.+?)\s*\|")
# Any backticked `*.yml` token — used across the WHOLE Workflow inventory
# section (a `.yml`-suffixed token cannot collide with `main`/`ui/e2e/**`
# branch/path mentions), so combined rows like `android.yml` / `ios.yml`
# are caught rather than requiring first-cell anchoring.
WORKFLOW_TOKEN = re.compile(r"`([a-z0-9][a-z0-9-]*\.yml)`")
# Job Matrix table row: job ID (first cell) + Blocks marker (last cell).
JOB_MATRIX_FULL_ROW = re.compile(r"^\|\s*`([a-z][a-z0-9-]*)`\s*\|.*\|\s*([^|]+?)\s*\|$")

# Job-level `continue-on-error:` (4-space indent under a job key).
JOB_COE = re.compile(r"^ {4}continue-on-error:\s*(.+?)\s*$")
# Step-level `continue-on-error: true` (8-space indent under a step).
STEP_COE_TRUE = re.compile(r"^ {8}continue-on-error:\s*true\s*$")


def load_gates() -> list[dict] | None:
    """Load and validate scripts/gates.json. Returns None on failure.

    Fail-CLOSED schema validation: a malformed manifest (e.g. a `runners`
    value that is a string instead of a list of needles) must never make
    the gate pass vacuously. Every structural violation returns None →
    the caller exits 2.
    """
    if not GATES_MANIFEST.is_file():
        print(f"error: gate manifest not found: {GATES_MANIFEST}", file=sys.stderr)
        return None
    try:
        data = json.loads(GATES_MANIFEST.read_text(encoding="utf-8"))
    except json.JSONDecodeError as e:
        print(f"error: gate manifest invalid JSON: {e}", file=sys.stderr)
        return None
    gates = data.get("gates")
    if not isinstance(gates, list) or not gates:
        print("error: gate manifest has no non-empty 'gates' list", file=sys.stderr)
        return None

    seen_ids: set[str] = set()
    for g in gates:
        if not isinstance(g, dict) or not isinstance(g.get("id"), str) or not g["id"]:
            print("error: gate manifest entry missing non-empty string 'id'", file=sys.stderr)
            return None
        gid = g["id"]
        if gid in seen_ids:
            print(f"error: duplicate gate id '{gid}' in gates.json", file=sys.stderr)
            return None
        seen_ids.add(gid)

        status = g.get("status")
        if status not in VALID_STATUS:
            print(
                f"error: gate '{gid}' has invalid status "
                f"'{status}' (expected {sorted(VALID_STATUS)})",
                file=sys.stderr,
            )
            return None

        runners = g.get("runners")
        if runners is not None:
            if not isinstance(runners, dict):
                print(f"error: gate '{gid}' 'runners' must be an object", file=sys.stderr)
                return None
            for runner, needles in runners.items():
                if runner not in ("check.sh", "check:all"):
                    print(f"error: gate '{gid}' has unknown runner '{runner}'", file=sys.stderr)
                    return None
                if (
                    not isinstance(needles, list)
                    or not needles
                    or not all(isinstance(n, str) and n.strip() for n in needles)
                ):
                    print(
                        f"error: gate '{gid}' runner '{runner}' needles must be a "
                        f"non-empty list of non-empty strings",
                        file=sys.stderr,
                    )
                    return None

        ci = g.get("ci")
        if ci is not None:
            if (
                not isinstance(ci, dict)
                or not isinstance(ci.get("workflow"), str)
                or not ci["workflow"]
                or not isinstance(ci.get("job"), str)
                or not ci["job"]
            ):
                print(
                    f"error: gate '{gid}' 'ci' must have non-empty string "
                    f"'workflow' and 'job' fields",
                    file=sys.stderr,
                )
                return None
            advisory_at = ci.get("advisory_at")
            if advisory_at is not None:
                if status != "advisory":
                    print(
                        f"error: gate '{gid}' ci.advisory_at is only valid for "
                        f"advisory gates",
                        file=sys.stderr,
                    )
                    return None
                if advisory_at not in ("job", "step"):
                    print(
                        f"error: gate '{gid}' ci.advisory_at must be 'job' or 'step'",
                        file=sys.stderr,
                    )
                    return None

        if status == "required-on-push" and ci is None:
            print(
                f"error: gate '{gid}' has status required-on-push but no 'ci' "
                f"mapping (its conditionality is workflow-enforced)",
                file=sys.stderr,
            )
            return None
    return gates


def doc_section(lines: list[str], title: str) -> list[str]:
    """Return the lines under a `## <title>` section (until the next `## `)."""
    out: list[str] = []
    in_section = False
    for line in lines:
        if line.startswith("## "):
            in_section = line[3:].strip() == title
            continue
        if in_section:
            out.append(line)
    return out


def workflow_jobs(path: Path) -> set[str]:
    """Job IDs declared under the `jobs:` block of a workflow file.

    This repo's convention (and GitHub Actions' usual style) is job IDs
    indented exactly 2 spaces under `jobs:`; the `on:` trigger keys (also
    2-space indented) appear BEFORE `jobs:`, so we only start collecting
    after the `jobs:` line and stop at the next 0-indent top-level key.
    """
    jobs: set[str] = set()
    in_jobs = False
    for line in path.read_text(encoding="utf-8").splitlines():
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        if not in_jobs:
            if re.match(r"^jobs:\s*(#.*)?$", stripped):
                in_jobs = True
            continue
        if re.match(r"^  [a-z0-9][a-z0-9_-]*:\s*$", line):
            jobs.add(line.strip()[:-1].strip())
        elif not line.startswith("  "):
            in_jobs = False  # 0-indent top-level key — jobs block ended
    return jobs


def job_coe(wf_path: Path, job: str) -> tuple[str | None, bool]:
    """Wrapper for job_coe_from_text that reads the file fresh."""
    return job_coe_from_text(wf_path.read_text(encoding="utf-8"), job)


def job_coe_from_text(text: str, job: str) -> tuple[str | None, bool]:
    """Return (job-level continue-on-error value, has-step-level COE:true)
    for the named job inside a workflow file.

    Job-level keys sit at 4-space indent under `  <job>:`; step-level
    keys sit at 8-space indent under a `      - name:` step. Values are
    returned raw (e.g. `true`, `${{ ... }}`), so callers can distinguish
    a bare `true` from a conditional expression.
    """
    lines = text.splitlines()
    in_jobs = False
    in_job = False
    job_level: str | None = None
    step_coe = False
    for line in lines:
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        if not in_jobs:
            if re.match(r"^jobs:\s*(#.*)?$", stripped):
                in_jobs = True
            continue
        # 2-space indent → job header.
        if re.match(r"^  [a-z0-9][a-z0-9_-]*:\s*$", line):
            in_job = line.strip()[:-1].strip() == job
            continue
        if not in_job:
            continue
        if line.startswith("  ") and not line.startswith("    "):
            continue  # 2-space key inside block (shouldn't happen) — skip
        m = JOB_COE.match(line)
        if m and job_level is None:
            job_level = m.group(1)
        if STEP_COE_TRUE.match(line):
            step_coe = True
        if not line.startswith("    ") and stripped:
            # 0-indent top-level key → jobs block ended.
            if not line.startswith("  "):
                break
    return job_level, step_coe


def check_sh_gates(path: Path) -> set[str]:
    """Gate labels from scripts/check.sh (`step "x"` + `echo -n "x..."`)."""
    text = path.read_text(encoding="utf-8")
    gates = set(re.findall(r'step\s+"([^"]+)"', text))
    # Non-step gates are announced with `echo -n "label... "` (the step()
    # helper's own `echo -n "${step_str}. checking ${name}... "` is excluded
    # by the `${` guard). Strip the trailing ellipsis/space to get the label.
    for label in re.findall(r'echo -n "([^"]+)"', text):
        if "${" in label:
            continue
        gates.add(label.rstrip(". \t"))
    return {g.lower() for g in gates}


def check_ui_gates(path: Path) -> set[str]:
    """Gate labels from scripts/check-ui.mjs (`gate('x')` + skip pushes)."""
    text = path.read_text(encoding="utf-8")
    gates = set(re.findall(r"gate\('([^']+)'", text))
    gates.update(re.findall(r"gate:\s*'([^']+)'", text))
    return {g.lower() for g in gates}


def has_needle(gates: set[str], needles: tuple[str, ...]) -> bool:
    """True if any extracted gate label contains any needle (ci-agnostic)."""
    return any(n in g for g in gates for n in needles)


def main() -> int:
    # Drift reports carry doc markers (✅/⚠️) — a cp1252 Windows console
    # must never crash with UnicodeEncodeError instead of failing the gate.
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
        sys.stderr.reconfigure(encoding="utf-8", errors="replace")
    except (AttributeError, ValueError):
        pass  # non-reconfigurable stream (e.g. some embedded runners)

    parser = argparse.ArgumentParser(
        description=(
            "Verify docs/ci-pipeline.md and scripts/gates.json only reference "
            "jobs/workflows that exist and that runners + workflows match the "
            "manifest gate vocabulary and status."
        )
    )
    parser.add_argument(
        "--report-only",
        action="store_true",
        help="Always exit 0; print the report and return.",
    )
    parser.add_argument(
        "--verbose",
        action="store_true",
        help="Print OK rows in addition to problems.",
    )
    args = parser.parse_args()

    gates = load_gates()
    if gates is None:
        return 2
    if not DOCS.is_file():
        print(f"error: docs not found: {DOCS}", file=sys.stderr)
        return 2
    if not WORKFLOWS_DIR.is_dir():
        print(f"error: workflows dir not found: {WORKFLOWS_DIR}", file=sys.stderr)
        return 2

    docs_lines = DOCS.read_text(encoding="utf-8").splitlines()
    workflow_files = sorted(WORKFLOWS_DIR.glob("*.yml"))
    workflows_by_name = {wf.name: wf for wf in workflow_files}

    # Fail-open protection: if a required section is renamed or emptied,
    # an empty parse result would make the gate vacuously PASS. Treat a
    # missing/empty required section as a structural error instead.
    required_sections = [
        "Job Matrix (ci.yml)",
        "Pre-Merge Validation Gates",
        "Workflow inventory",
    ]
    missing_sections = [
        title
        for title in required_sections
        if not doc_section(docs_lines, title)
    ]
    if missing_sections:
        print(
            f"error: docs/ci-pipeline.md is missing required section(s) — "
            f"{', '.join(missing_sections)}",
            file=sys.stderr,
        )
        return 2

    # ── 1. Jobs: documented → workflows ─────────────────────────────
    matrix_jobs = {
        m.group(1)
        for m in (JOB_MATRIX_ROW.match(line) for line in doc_section(docs_lines, "Job Matrix (ci.yml)"))
        if m
    }
    if not matrix_jobs:
        print(
            "error: Job Matrix (ci.yml) section contains no job rows",
            file=sys.stderr,
        )
        return 2

    gate_jobs: set[str] = set()
    for line in doc_section(docs_lines, "Pre-Merge Validation Gates"):
        m = GATE_TABLE_ROW.match(line)
        if m:
            gate_jobs.update(KUBE_TOKEN.findall(m.group(1)))
    if not gate_jobs:
        print(
            "error: Pre-Merge Validation Gates section contains no job rows",
            file=sys.stderr,
        )
        return 2

    # Cache per-workflow job sets + raw text once (status checks loop over
    # every ci-mapped gate; re-parsing per gate would be redundant work).
    all_jobs: set[str] = set()
    ci_jobs: set[str] = set()
    jobs_by_workflow: dict[str, set[str]] = {}
    wf_texts: dict[str, str] = {}
    for wf in workflow_files:
        jobs = workflow_jobs(wf)
        jobs_by_workflow[wf.name] = jobs
        wf_texts[wf.name] = wf.read_text(encoding="utf-8")
        all_jobs |= jobs
        if wf.name == "ci.yml":
            ci_jobs = jobs

    missing_jobs = sorted((matrix_jobs | gate_jobs) - all_jobs)
    # Informational: the docs' Job Matrix catalogs ci.yml specifically, so
    # flag jobs ADDED to ci.yml that the docs don't mention (the fail
    # direction is docs-referenced-but-missing; this is the reverse).
    undocumented = sorted(ci_jobs - (matrix_jobs | gate_jobs))
    # Informational: a matrix job that only exists in another workflow
    # (e.g. moved to nightly.yml) — the docs table is titled "(ci.yml)",
    # so note it even though the job exists somewhere.
    matrix_not_in_ci = sorted(matrix_jobs - ci_jobs)

    # ── 2. Workflow inventory: named files exist ────────────────────
    # Extract every backticked `*.yml` token from the whole section so
    # combined rows like `android.yml` / `ios.yml` are also captured.
    inventory_files = set(
        WORKFLOW_TOKEN.findall("\n".join(doc_section(docs_lines, "Workflow inventory")))
    )
    if not inventory_files:
        print(
            "error: Workflow inventory section contains no *.yml references",
            file=sys.stderr,
        )
        return 2
    missing_files = sorted(f for f in inventory_files if not (WORKFLOWS_DIR / f).is_file())

    # ── 3. Gate vocabulary: manifest → runners ──────────────────────
    sh_gates = check_sh_gates(CHECK_SH) if CHECK_SH.is_file() else set()
    ui_gates = check_ui_gates(CHECK_UI) if CHECK_UI.is_file() else set()

    gate_problems: list[str] = []
    for gate in gates:
        gid, label = gate["id"], gate["label"]
        runners = gate.get("runners") or {}
        sh_needles = tuple(runners.get("check.sh") or ())
        ui_needles = tuple(runners.get("check:all") or ())
        if sh_needles and not has_needle(sh_gates, sh_needles):
            gate_problems.append(
                f"manifest gate '{gid}' ({label}) not declared in scripts/check.sh"
            )
        if ui_needles and not has_needle(ui_gates, ui_needles):
            gate_problems.append(
                f"manifest gate '{gid}' ({label}) not declared in check:all"
            )

    # Informational: labels the runners declare that no manifest gate covers.
    all_sh_needles = {
        n.lower() for g in gates for n in ((g.get("runners") or {}).get("check.sh") or ())
    }
    all_ui_needles = {
        n.lower() for g in gates for n in ((g.get("runners") or {}).get("check:all") or ())
    }
    sh_uncatalogued = sorted(s for s in sh_gates if not any(n in s for n in all_sh_needles))
    ui_uncatalogued = sorted(s for s in ui_gates if not any(n in s for n in all_ui_needles))

    # ── 4. Gate CI mapping + status: manifest → workflows ───────────
    status_problems: list[str] = []
    for gate in gates:
        ci = gate.get("ci")
        if not ci:
            continue
        gid, status = gate["id"], gate["status"]
        wf_name = ci.get("workflow")
        job = ci.get("job")
        wf_path = workflows_by_name.get(wf_name)
        if wf_path is None:
            status_problems.append(
                f"manifest gate '{gid}': workflow file '{wf_name}' does not exist"
            )
            continue
        if job not in jobs_by_workflow.get(wf_name, set()):
            status_problems.append(
                f"manifest gate '{gid}': job '{job}' not found in {wf_name}"
            )
            continue
        job_level, has_step_coe = job_coe_from_text(wf_texts[wf_name], job)
        if status == "required":
            if (job_level or "").strip() == "true":
                status_problems.append(
                    f"manifest gate '{gid}': status required but job "
                    f"'{wf_name}/{job}' has continue-on-error: true"
                )
        elif status == "advisory":
            advisory_at = ci.get("advisory_at", "job")
            if advisory_at == "step":
                if not has_step_coe:
                    status_problems.append(
                        f"manifest gate '{gid}': status advisory (advisory_at: step) "
                        f"but no step in '{wf_name}/{job}' has continue-on-error: true"
                    )
            else:
                v = (job_level or "").strip()
                if v != "true" and not v.startswith("${{"):
                    status_problems.append(
                        f"manifest gate '{gid}': status advisory but job "
                        f"'{wf_name}/{job}' lacks continue-on-error: true "
                        f"(got {v or 'none'})"
                    )
        elif status == "required-on-push":
            v = (job_level or "").strip()
            if not v.startswith("${{"):
                status_problems.append(
                    f"manifest gate '{gid}': status required-on-push but job "
                    f"'{wf_name}/{job}' continue-on-error must be a conditional "
                    f"${{{{ ... }}}} expression (got {v or 'none'})"
                )

    # ── 5. Docs Job Matrix Blocks column ↔ manifest status ─────────
    # The docs table's last column carries a per-job status marker
    # (✅ Required / ⚠️ Advisory / ⚠️ Advisory on PR, ✅ Required on
    # push / Push path). Cross-check it against the manifest status of
    # every ci.yml-mapped gate so the docs status prose cannot drift
    # from the manifest the way the old hardcoded lists did.
    docs_status_problems: list[str] = []
    for line in doc_section(docs_lines, "Job Matrix (ci.yml)"):
        m = JOB_MATRIX_FULL_ROW.match(line)
        if not m:
            continue
        job_id, blocks = m.group(1), m.group(2).strip()
        if "advisory on pr" in blocks.lower() and "required on push" in blocks.lower():
            expected = "required-on-push"
        elif "advisory" in blocks.lower():
            expected = "advisory"
        elif "required" in blocks.lower():
            expected = "required"
        else:
            continue  # e.g. "Push path" or an unrecognised marker — no status semantics
        for gate in gates:
            ci = gate.get("ci") or {}
            if ci.get("workflow") != "ci.yml" or ci.get("job") != job_id:
                continue
            if gate["status"] != expected:
                docs_status_problems.append(
                    f"docs Job Matrix marks '{job_id}' as '{blocks}' but gates.json "
                    f"status is '{gate['status']}' (gate '{gate['id']}')"
                )

    # ── Report ──────────────────────────────────────────────────────
    manifest_counts: dict[str, int] = {"required": 0, "advisory": 0, "required-on-push": 0}
    for g in gates:
        manifest_counts[g["status"]] += 1

    print(
        f"verify-ci-docs-drift: {len(workflow_files)} workflow file(s), "
        f"{len(all_jobs)} job(s); docs reference {len(matrix_jobs)} matrix "
        f"job(s) + {len(gate_jobs)} gate job(s)."
    )
    print(
        f"  gates.json: {len(gates)} gate(s) "
        f"({manifest_counts['required']} required, "
        f"{manifest_counts['advisory']} advisory, "
        f"{manifest_counts['required-on-push']} required-on-push)."
    )
    print(
        f"  check.sh declares {len(sh_gates)} gate(s); check:all declares "
        f"{len(ui_gates)} gate(s)."
    )
    print()

    if args.verbose and not (
        missing_jobs or missing_files or gate_problems or status_problems or docs_status_problems
    ):
        print("  OK: every documented job/workflow exists; manifest gates match runners + workflows.")
        print()

    if missing_jobs:
        print(f"  MISSING JOBS (documented but no matching workflow job) — {len(missing_jobs)}:")
        for job in missing_jobs:
            print(f"    {job}")
        print()
    if missing_files:
        print(f"  MISSING WORKFLOW FILES (inventory names nothing on disk) — {len(missing_files)}:")
        for f in missing_files:
            print(f"    {f}")
        print()
    if gate_problems:
        print(f"  GATE VOCABULARY DRIFT — {len(gate_problems)}:")
        for p in gate_problems:
            print(f"    {p}")
        print()
    if status_problems:
        print(f"  GATE STATUS DRIFT (workflow contradicts manifest) — {len(status_problems)}:")
        for p in status_problems:
            print(f"    {p}")
        print()
    if docs_status_problems:
        print(
            f"  DOCS STATUS DRIFT (docs Job Matrix contradicts manifest) — "
            f"{len(docs_status_problems)}:"
        )
        for p in docs_status_problems:
            print(f"    {p}")
        print()
    if undocumented:
        print(
            f"  note: {len(undocumented)} ci.yml job(s) exist but are not "
            f"referenced in docs/ci-pipeline.md (informational):"
        )
        print("    " + ", ".join(undocumented))
        print()
    if matrix_not_in_ci:
        print(
            f"  note: {len(matrix_not_in_ci)} matrix job(s) are not in "
            f"ci.yml (titled 'Job Matrix (ci.yml)') — verify the docs table "
            f"header matches where they live (informational):"
        )
        print("    " + ", ".join(matrix_not_in_ci))
        print()
    if sh_uncatalogued or ui_uncatalogued:
        extras = sh_uncatalogued + [f"[check:all] {x}" for x in ui_uncatalogued]
        print(
            f"  note: {len(extras)} runner label(s) not covered by any "
            f"gates.json gate (informational):"
        )
        print("    " + ", ".join(extras))
        print()

    problems = (
        len(missing_jobs)
        + len(missing_files)
        + len(gate_problems)
        + len(status_problems)
        + len(docs_status_problems)
    )
    print(f"verify-ci-docs-drift: {problems} drift item(s).")
    return 0 if (args.report_only or problems == 0) else 1


if __name__ == "__main__":
    sys.exit(main())
