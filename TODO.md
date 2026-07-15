# 0.0.9

> **Goal:** TBD

---

## 📊 Current State

| Layer | Test files | Total tests | Status |
|-------|-----------|-------------|--------|
| Rust | 26 crates | ~800+ | ✅ All passing |
| UI/Vitest | 119 files | 1,939 | ✅ All passing (0 failures, 0 skipped, 0 ignored) |

---

## 🎯 Priorities

- [ ] TBD

---

## 🚦 Safety Rules

- **Never delete a test assertion** — only reorganize or deduplicate.
- **Run `vitest run` after every UI change**, `cargo test -p <crate>` after every Rust change.
- **Commit each completed checklist section separately** with `[skip ci]` if only
  test code changes.
- **If a test breaks**, revert to the last working commit and re-approach more carefully.
