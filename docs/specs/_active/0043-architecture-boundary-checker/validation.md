# Validation plan

## Acceptance criteria

- [ ] The checker runs from the repository root on Windows and POSIX shells.
- [ ] The checker obtains the real Cargo graph through `cargo metadata` when no
      fixture is supplied.
- [ ] A metadata fixture can be supplied for deterministic tests.
- [ ] Normal production module-to-module dependencies are reported.
- [ ] `oz-core` production dependencies on business modules are reported.
- [ ] `platform-startup` composition dependencies are allowed by explicit rule.
- [ ] Dev-only dependencies do not fail the strict gate.
- [ ] Direct production UI `invoke()` calls outside `ui/src/api/` are reported.
- [ ] API-layer calls, comments, tests, and dev mocks do not create findings.
- [ ] Existing findings are shown as tracked transitional debt.
- [ ] A new finding not present in the baseline exits non-zero.
- [ ] An expired or stale baseline entry exits non-zero.
- [ ] `--report-only` exits zero while still showing findings.
- [ ] `--json` output is stable and contains rule, path, line, target, and status.
- [ ] Malformed metadata exits with a distinct non-zero error code.
- [ ] No runtime Rust, database, Tauri, or UI behavior changes.

## Required tests

The script test suite must cover:

1. clean fixture;
2. module-to-module production dependency;
3. `oz-core` upward dependency;
4. allowed `platform-startup` composition dependency;
5. dev-dependency exclusion;
6. direct UI invoke outside API;
7. allowed API-layer invoke;
8. comments/test/dev-mock exclusion;
9. baseline suppression with report visibility;
10. new violation failure;
11. expired baseline failure;
12. malformed metadata failure;
13. Windows path normalization;
14. JSON output schema basics.

## Commands

Focused tests:

```bash
node --test scripts/__tests__/verify-architecture-boundaries.test.mjs
python3 scripts/verify-architecture-boundaries.py --report-only
python3 scripts/verify-architecture-boundaries.py --json
```

Static validation after implementation:

```bash
python3 scripts/verify-architecture-boundaries.py --strict
cargo fmt --all -- --check
```

If the checker is added to the repository gate, also validate the gate
vocabulary:

```bash
python3 scripts/verify-ci-docs-drift.py
```

The full workspace test/clippy gates are not required to validate the first
static-only pilot unless the implementation changes Rust source or manifests.
