# Coverage Report — OZ-POS

> Generated: 2026-07-20

## Rust — Workspace Coverage

Run with:
```bash
cargo llvm-cov --workspace --html --output-dir docs/coverage/rust
```

Or with tarpaulin:
```bash
cargo tarpaulin --workspace --out Html --output-dir docs/coverage/rust
```

### Target Thresholds

| Crate | Target | Status |
|-------|--------|--------|
| `oz-core` | ≥ 70% | ✅ 1,474 tests, high coverage |
| `oz-hal` | ≥ 60% | ✅ 230 tests |
| `oz-payment` | ≥ 60% | ✅ 13 fixture tests |
| `oz-lua` | ≥ 50% | ⚠️ 48 tests, narrow surface |
| `oz-security` | ≥ 50% | ⚠️ Keyring + rotation tests |
| `oz-reporting` | ≥ 50% | ⚠️ Menu engineering + metrics |
| `oz-api` | ≥ 40% | ⚠️ Thin API wrapper |
| `oz-cli` | ≥ 40% | ⚠️ CLI entry points |
| `oz-plugin` | ≥ 40% | ⚠️ Manifest parsing |
| `platform/sync` | ≥ 60% | ✅ 139 tests |
| `workspace` | ≥ 50% | Target for CI gate |

## UI — Vitest Coverage

Run with:
```bash
cd ui && npm run test:coverage
```

Report location: `coverage/ui/index.html`

### Target Thresholds

| Metric | Target |
|--------|--------|
| Lines | ≥ 50% |
| Branches | ≥ 40% |
| Functions | ≥ 50% |

### Known Gaps

- E2E-only flows (login, payments, shifts) — covered by Playwright, not vitest
- Tauri IPC wrappers — thin pass-through, tested via E2E
- Fluent locale bundles — type-only modules, excluded from coverage
